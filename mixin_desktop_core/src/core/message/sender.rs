use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use log::{error, info, warn};
use tokio::time::{interval_at, sleep, Duration, Instant};
use uuid::Uuid;

use sdk::err::error_code::{BAD_DATA, CONVERSATION_CHECKSUM_INVALID_ERROR, FORBIDDEN};
use sdk::{
    message_category, BlazeMessage, BlazeMessageParam, BlazeMessageParamSession,
    BlazeSignalKeyMessage, MessageStatus, PlainJsonMessage, SignalKey, SignalKeyCount, NO_KEY,
    RESEND_KEY, RESEND_MESSAGES,
};

use crate::core::constants::TEAM_MIXIN_USER_ID;
use sdk::generate_conversation_id;

use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::message::blaze::Blaze;
use crate::core::model::signal::SignalService;
use crate::core::model::ConversationService;
use crate::db::signal::ratchet_sender_key::{ratchet_sender_key_status, RatchetSenderKey};
use crate::db::MixinDatabase;

#[derive(Clone)]
pub struct MessageSender {
    blaze: Arc<Blaze>,
    conversation: ConversationService,
    database: Arc<MixinDatabase>,
    account_id: String,
    session_id: String,
    signal_protocol: Arc<SignalProtocol>,
    signal_service: SignalService,
    last_signal_key_refresh: Arc<Mutex<Option<std::time::Instant>>>,
}

pub struct MessageResult {
    pub success: bool,
    pub retry: bool,
    pub error_code: Option<i64>,
}

impl MessageSender {
    pub async fn send_plain_json_to_session(
        &self,
        action: &str,
        content: String,
        recipient_session_id: Option<String>,
    ) -> Result<()> {
        let conversation_id = self
            .database
            .participant_dao
            .find_any_joined_conversation_id(&self.account_id)
            .await?
            .unwrap_or_else(|| {
                generate_conversation_id(&self.account_id, TEAM_MIXIN_USER_ID).to_string()
            });
        let plain_text = PlainJsonMessage {
            action: action.to_string(),
            content: Some(content),
            ..PlainJsonMessage::default()
        };
        let encoded = Base64::encode_string(&serde_json::to_vec(&plain_text)?);
        let message = BlazeMessage::new_plain_json(
            &conversation_id,
            self.get_check_sum(&conversation_id).await?,
            &self.account_id,
            encoded,
            recipient_session_id,
        );
        self.deliver(message).await?;
        Ok(())
    }

    pub fn new(
        blaze: Arc<Blaze>,
        conversation: ConversationService,
        database: Arc<MixinDatabase>,
        account_id: String,
        session_id: String,
        signal_protocol: Arc<SignalProtocol>,
        signal_service: SignalService,
    ) -> Self {
        MessageSender {
            blaze,
            conversation,
            database,
            account_id,
            session_id,
            signal_protocol,
            last_signal_key_refresh: Arc::new(Mutex::new(None)),
            signal_service,
        }
    }
}

const MAX_CHANNEL_ATTEMPTS: usize = 3;
const MAX_CHECKSUM_ATTEMPTS: usize = 2;

pub enum ProcessSignalKeyAction<'a> {
    AddParticipant(&'a str),
    RemoveParticipant(&'a str),
    ResendKey,
}

impl MessageSender {
    pub async fn maintain_signal_keys(&self) {
        let has_push_signal_keys = self
            .signal_protocol
            .signal_database
            .crypto_key_value
            .has_push_signal_keys();
        let startup = if has_push_signal_keys {
            self.check_signal_keys().await
        } else {
            self.push_signal_keys().await
        };
        if let Err(err) = startup {
            warn!("failed to maintain signal keys at startup: {err}");
        }

        let period = Duration::from_secs(24 * 60 * 60);
        let mut interval = interval_at(Instant::now() + period, period);
        loop {
            interval.tick().await;
            if let Err(err) = self.check_signal_keys().await {
                warn!("failed to maintain signal keys periodically: {err}");
            }
        }
    }

    async fn check_signal_keys(&self) -> Result<()> {
        let key_count = self.signal_service.key_count().await?;
        info!("signal keys count: {key_count}");
        if key_count > 500 {
            return Ok(());
        }
        self.push_signal_keys().await
    }

    async fn push_signal_keys(&self) -> Result<()> {
        self.signal_service.push_keys().await?;
        info!("registered new pre keys");
        Ok(())
    }

    pub async fn send_process_signal_key<'a>(
        &self,
        data: &sdk::BlazeMessageData,
        action: ProcessSignalKeyAction<'a>,
    ) -> Result<()> {
        match action {
            ProcessSignalKeyAction::ResendKey => {
                let result = self
                    .send_sender_key(&data.conversation_id, &data.user_id, &data.session_id)
                    .await?;
                if !result {
                    self.send_no_key_message(&data.conversation_id, &data.user_id)
                        .await?;
                }
            }
            ProcessSignalKeyAction::RemoveParticipant(pid) => {
                self.database
                    .participant_dao
                    .remove_participant(&data.conversation_id, pid)
                    .await?;
                self.database
                    .participant_session_dao
                    .remove_participant(&data.conversation_id, pid)
                    .await?;
                self.database
                    .participant_session_dao
                    .clear_status(&data.conversation_id)
                    .await?;
                self.signal_protocol
                    .signal_database
                    .sender_key_dao
                    .delete_sender_key(&data.conversation_id, &self.account_id, 1)
                    .await?;
            }
            ProcessSignalKeyAction::AddParticipant(pid) => {
                self.conversation
                    .refresh_session(&data.conversation_id, &[pid.to_string()])
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn refresh_signal_key(&self, conversation_id: &str) -> Result<()> {
        info!("start refresh signal key: {}", conversation_id);
        let now = std::time::Instant::now();
        {
            let last = self.last_signal_key_refresh.lock().unwrap();
            if let Some(last) = *last {
                if now - last < Duration::from_secs(60) {
                    return Ok(());
                }
            }
        }

        let data = self
            .signal_keys_channel(BlazeMessage::new_count_signal_keys())
            .await?
            .and_then(|m| m.data)
            .ok_or(anyhow!("Failed to get signal keys count"))?;

        let key_count: SignalKeyCount = serde_json::from_value(data)?;
        info!("signal keys count: {}", key_count.one_time_pre_keys_count);

        if key_count.one_time_pre_keys_count >= 500 {
            *self.last_signal_key_refresh.lock().unwrap() = Some(now);
            return Ok(());
        }

        let bm = BlazeMessage::new_sync_signal_keys(
            self.signal_service
                .generate_keys()
                .await
                .map_err(|e| anyhow!("Failed to generate keys: {e}"))?,
        );

        self.signal_keys_channel(bm).await?;
        self.signal_protocol
            .signal_database
            .crypto_key_value
            .set_has_push_signal_keys(true)
            .await?;
        *self.last_signal_key_refresh.lock().unwrap() = Some(now);
        info!("Registering new pre keys... {}", conversation_id);
        Ok(())
    }

    pub async fn signal_keys_channel(
        &self,
        blaze_message: BlazeMessage,
    ) -> Result<Option<BlazeMessage>> {
        for attempt in 0..MAX_CHANNEL_ATTEMPTS {
            let bm = self.blaze.send_message(blaze_message.clone()).await?;
            let Some(err) = &bm.error else {
                return Ok(Some(bm));
            };
            error!(
                "failed to signal_keys_channel: {} {}",
                err.code, err.description
            );
            if err.code == FORBIDDEN {
                return Ok(None);
            }
            if attempt + 1 < MAX_CHANNEL_ATTEMPTS {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            bail!(
                "signal keys channel failed after {} attempts: {} {}",
                MAX_CHANNEL_ATTEMPTS,
                err.code,
                err.description
            );
        }
        unreachable!()
    }

    pub async fn request_resend_key(
        &self,
        cid: &str,
        recipient_id: &str,
        mid: &str,
        sid: &str,
    ) -> Result<()> {
        let message = PlainJsonMessage {
            action: RESEND_KEY.to_string(),
            message_id: Some(mid.to_string()),
            ..PlainJsonMessage::default()
        };
        let message = serde_json::to_vec(&message)?;
        let encoded = Base64::encode_string(&message);
        let bm = BlazeMessage::new_plain_json(
            cid,
            self.get_check_sum(cid).await?,
            recipient_id,
            encoded,
            sid.to_string(),
        );

        let result = self.deliver(bm).await?;
        if result.success {
            let address = format!("{}:{}", recipient_id, SignalProtocol::device_id(Some(sid))?);
            self.signal_protocol
                .signal_database
                .ratchet_sender_key_dao
                .insert_sender_key(&RatchetSenderKey {
                    group_id: cid.to_string(),
                    sender_id: address,
                    status: ratchet_sender_key_status::REQUESTING.to_string(),
                    message_id: None,
                    created_at: Utc::now().to_rfc3339(),
                })
                .await?;
        }
        Ok(())
    }

    pub async fn request_resend_message(&self, cid: &str, uid: &str, sid: &str) -> Result<()> {
        let messages = self
            .database
            .message_dao
            .find_failed_message(cid, uid)
            .await?;
        if messages.is_empty() {
            return Ok(());
        }

        let message = PlainJsonMessage {
            action: RESEND_MESSAGES.to_string(),
            messages: Some(messages),
            ..PlainJsonMessage::default()
        };
        let message = serde_json::to_vec(&message)?;
        let encoded = Base64::encode_string(&message);
        let bm = BlazeMessage::new_plain_json(
            cid,
            self.get_check_sum(cid).await?,
            uid,
            encoded,
            sid.to_string(),
        );

        self.deliver(bm).await?;
        self.signal_protocol
            .signal_database
            .ratchet_sender_key_dao
            .delete(
                cid,
                &format!("{}:{}", uid, SignalProtocol::device_id(Some(sid))?),
            )
            .await?;

        Ok(())
    }

    pub async fn send_sender_key(&self, cid: &str, uid: &str, sid: &str) -> Result<bool> {
        if !self.check_signal_session(uid, sid).await? {
            return Ok(false);
        }

        for attempt in 0..MAX_CHECKSUM_ATTEMPTS {
            let encrypted = self
                .signal_protocol
                .encrypt_sender_key(cid, uid, SignalProtocol::device_id(Some(sid))?)
                .await
                .map_err(|e| anyhow!("failed to encrypt sender key: {e}"))?;
            let Some(encrypted) = encrypted else {
                return Ok(false);
            };
            let messages = vec![BlazeSignalKeyMessage {
                message_id: Uuid::new_v4().to_string(),
                recipient_id: uid.to_string(),
                data: encrypted,
                session_id: Some(sid.to_string()),
            }];
            let check_sum = self.get_check_sum(cid).await?;
            let bm = BlazeMessage::new_signal_key_message(cid.to_string(), messages, check_sum);
            let result = self.deliver(bm).await?;
            if result.success {
                self.database
                    .participant_session_dao
                    .update_status(cid, uid, sid, 1)
                    .await?;
                return Ok(true);
            }
            if !result.retry || attempt + 1 == MAX_CHECKSUM_ATTEMPTS {
                return Ok(false);
            }
        }
        Ok(false)
    }

    pub async fn check_signal_session(&self, uid: &str, sid: &str) -> Result<bool> {
        if self.signal_protocol.contains_session(uid, sid).await? {
            return Ok(true);
        }

        let request_keys = vec![BlazeMessageParamSession {
            user_id: uid.to_string(),
            session_id: sid.to_string(),
        }];
        let blaze_message = BlazeMessage::new_consume_session_signal_keys(request_keys);
        let data = self
            .signal_keys_channel(blaze_message)
            .await?
            .and_then(|e| e.data);

        let Some(data) = data else {
            return Ok(false);
        };
        let keys: Vec<SignalKey> = serde_json::from_value(data)?;
        let Some(key) = keys
            .iter()
            .find(|key| key.user_id == uid && key.session_id == sid)
        else {
            return Ok(false);
        };
        self.signal_protocol
            .process_session(uid, key)
            .await
            .map_err(|e| anyhow!("failed to process session: {e}"))?;
        Ok(true)
    }

    async fn distribute_sender_keys(&self, cid: &str) -> Result<()> {
        let sessions = self
            .database
            .participant_session_dao
            .not_sent_participant_sessions(cid, &self.session_id)
            .await?;
        for session in sessions {
            self.send_sender_key(cid, &session.user_id, &session.session_id)
                .await?;
        }
        Ok(())
    }

    async fn prepare_group_signal_session(&self, cid: &str) -> Result<()> {
        let has_sender_key = self
            .signal_protocol
            .protocol_store
            .sender_key_store
            .exists_sender_key(cid, &self.account_id)
            .await?;
        let has_participant_sessions = !self
            .database
            .participant_session_dao
            .get_participant_sessions(cid)
            .await?
            .is_empty();
        if !has_sender_key || !has_participant_sessions {
            self.conversation.refresh_conversation(cid).await?;
        }
        self.distribute_sender_keys(cid).await?;
        if !self
            .signal_protocol
            .protocol_store
            .sender_key_store
            .exists_sender_key(cid, &self.account_id)
            .await?
        {
            bail!("sender key is unavailable for conversation {cid}");
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_signal_message(
        &self,
        message_id: &str,
        cid: &str,
        category: &str,
        content: &str,
        quote_message_id: Option<&str>,
        mentions: Option<&[String]>,
        silent: bool,
        expire_in: i32,
    ) -> Result<MessageResult> {
        self.prepare_group_signal_session(cid).await?;
        for attempt in 0..MAX_CHECKSUM_ATTEMPTS {
            let data = self
                .signal_protocol
                .encrypt_group_message(cid, content.as_bytes())
                .await
                .map_err(|e| anyhow!("failed to encrypt group message: {e}"))?;
            let bm = BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(cid.to_string()),
                conversation_checksum: Some(self.get_check_sum(cid).await?),
                message_id: Some(message_id.to_string()),
                category: Some(category.to_string()),
                data: Some(data),
                quote_message_id: quote_message_id.map(str::to_string),
                mentions: mentions.map(<[String]>::to_vec),
                silent: Some(silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            });
            let result = self.deliver(bm).await?;
            if !result.retry || attempt + 1 == MAX_CHECKSUM_ATTEMPTS {
                return Ok(result);
            }
            self.prepare_group_signal_session(cid).await?;
        }
        unreachable!()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn resend_signal_message(
        &self,
        original_message_id: &str,
        cid: &str,
        recipient_id: &str,
        recipient_session_id: &str,
        category: &str,
        content: &str,
        quote_message_id: Option<&str>,
        mentions: Option<&[String]>,
        silent: bool,
        expire_in: i32,
    ) -> Result<MessageResult> {
        if !self
            .check_signal_session(recipient_id, recipient_session_id)
            .await?
        {
            return Ok(MessageResult {
                success: false,
                retry: true,
                error_code: None,
            });
        }
        let data = self
            .signal_protocol
            .encrypt_session_message(
                content.as_bytes(),
                recipient_id,
                recipient_session_id,
                Some(original_message_id),
            )
            .await
            .map_err(|e| anyhow!("failed to encrypt session message: {e}"))?;
        let bm = BlazeMessage::new_param_blaze(BlazeMessageParam {
            conversation_id: Some(cid.to_string()),
            conversation_checksum: Some(self.get_check_sum(cid).await?),
            recipient_id: Some(recipient_id.to_string()),
            message_id: Some(Uuid::new_v4().to_string()),
            category: Some(category.to_string()),
            data: Some(data),
            quote_message_id: quote_message_id.map(str::to_string),
            session_id: Some(recipient_session_id.to_string()),
            mentions: mentions.map(<[String]>::to_vec),
            silent: Some(silent),
            expire_in: Some(expire_in),
            ..BlazeMessageParam::default()
        });
        self.deliver(bm).await
    }

    pub async fn get_check_sum(&self, cid: &str) -> Result<String> {
        let sessions = self
            .database
            .participant_session_dao
            .get_participant_sessions(cid)
            .await?;
        let sessions = sessions
            .into_iter()
            .map(|session| session.session_id)
            .collect::<Vec<_>>();
        Ok(generate_conversation_checksum(sessions))
    }

    pub async fn send_no_key_message(&self, cid: &str, uid: &str) -> Result<()> {
        let plain_text = PlainJsonMessage {
            action: NO_KEY.to_string(),
            ..PlainJsonMessage::default()
        };

        let encoded = Base64::encode_string(&serde_json::to_vec(&plain_text)?);

        let bm = BlazeMessage::new_param_blaze(BlazeMessageParam {
            conversation_id: Some(cid.to_string()),
            conversation_checksum: Some(self.get_check_sum(cid).await?),
            recipient_id: Some(uid.to_string()),
            message_id: Some(Uuid::new_v4().to_string()),
            category: Some(message_category::PLAIN_JSON.to_string()),
            data: Some(encoded),
            status: Some(MessageStatus::Sending.into()),
            ..BlazeMessageParam::default()
        });
        self.deliver(bm).await?;
        Ok(())
    }

    pub async fn deliver(&self, msg: BlazeMessage) -> Result<MessageResult> {
        if let Some(params) = &msg.params {
            if params.conversation_id.is_some() && params.conversation_checksum.is_none() {
                bail!("invalid message: missing checksum");
            }
        }

        for attempt in 0..MAX_CHANNEL_ATTEMPTS {
            let result = self.blaze.send_message(msg.clone()).await?;
            let Some(err) = &result.error else {
                return Ok(MessageResult {
                    success: true,
                    retry: false,
                    error_code: None,
                });
            };
            warn!(
                "failed to send message, code :{}, description: {}, {}",
                err.code,
                err.description,
                Backtrace::capture()
            );
            if err.code == CONVERSATION_CHECKSUM_INVALID_ERROR {
                let cid = msg.params.as_ref().and_then(|p| p.conversation_id.as_ref());
                if let Some(cid) = cid {
                    self.conversation.refresh_conversation(cid).await?;
                }
                return Ok(MessageResult {
                    success: false,
                    retry: true,
                    error_code: Some(err.code),
                });
            }
            if err.code == FORBIDDEN || err.code == BAD_DATA {
                return Ok(MessageResult {
                    success: false,
                    retry: false,
                    error_code: Some(err.code),
                });
            }
            if attempt + 1 < MAX_CHANNEL_ATTEMPTS {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            return Ok(MessageResult {
                success: false,
                retry: true,
                error_code: Some(err.code),
            });
        }
        unreachable!()
    }
}

fn generate_conversation_checksum(mut sessions: Vec<String>) -> String {
    if sessions.is_empty() {
        return String::new();
    }
    sessions.sort();
    format!("{:x}", md5::compute(sessions.concat()))
}

#[cfg(test)]
mod tests {
    use super::generate_conversation_checksum;

    #[test]
    fn conversation_checksum_matches_flutter() {
        assert_eq!(generate_conversation_checksum(Vec::new()), "");
        assert_eq!(
            generate_conversation_checksum(vec!["session-b".into(), "session-a".into()]),
            "34b4db93ff28fd2f9949d674f2f654c5"
        );
    }
}
