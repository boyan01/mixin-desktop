use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use log::{error, info, warn};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use sdk::err::error_code::{BAD_DATA, CONVERSATION_CHECKSUM_INVALID_ERROR, FORBIDDEN};
use sdk::{
    message_category, BlazeMessage, BlazeMessageParam, BlazeMessageParamSession,
    BlazeSignalKeyMessage, MessageStatus, PlainJsonMessage, SignalKey, SignalKeyCount, UserSession,
    NO_KEY, RESEND_KEY, RESEND_MESSAGES,
};

use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::message::blaze::Blaze;
use crate::core::model::signal::SignalService;
use crate::core::model::ConversationService;
use crate::core::util::unique_object_id;
use crate::db::signal::ratchet_sender_key::{ratchet_sender_key_status, RatchetSenderKey};
use crate::db::MixinDatabase;

#[derive(Clone)]
pub struct MessageSender {
    blaze: Arc<Blaze>,
    conversation: ConversationService,
    database: Arc<MixinDatabase>,
    account_id: String,
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
    pub fn new(
        blaze: Arc<Blaze>,
        conversation: ConversationService,
        database: Arc<MixinDatabase>,
        account_id: String,
        signal_protocol: Arc<SignalProtocol>,
        signal_service: SignalService,
    ) -> Self {
        MessageSender {
            blaze,
            conversation,
            database,
            account_id,
            signal_protocol,
            last_signal_key_refresh: Arc::new(Mutex::new(None)),
            signal_service,
        }
    }
}

pub enum ProcessSignalKeyAction<'a> {
    AddParticipant(&'a str),
    RemoveParticipant(&'a str),
    ResendKey,
}

impl MessageSender {
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
            let mut last = self.last_signal_key_refresh.lock().unwrap();
            if let Some(last) = *last {
                if now - last < Duration::from_secs(60) {
                    return Ok(());
                }
            }
            *last = Some(now);
        }

        let data = self
            .signal_keys_channel(BlazeMessage::new_count_signal_keys())
            .await?
            .and_then(|m| m.data)
            .ok_or(anyhow!("Failed to get signal keys count"))?;

        let key_count: SignalKeyCount = serde_json::from_value(data)?;
        info!("signal keys count: {}", key_count.one_time_pre_keys_count);

        let has_push_signal_keys = self
            .signal_protocol
            .signal_database
            .crypto_key_value
            .has_push_signal_keys();

        if has_push_signal_keys && key_count.one_time_pre_keys_count >= 500 {
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
            .await;
        info!("Registering new pre keys... {}", conversation_id);
        Ok(())
    }

    pub async fn signal_keys_channel(
        &self,
        blaze_message: BlazeMessage,
    ) -> Result<Option<BlazeMessage>> {
        let bm = self.blaze.send_message(blaze_message.clone()).await?;
        if let Some(err) = &bm.error {
            error!(
                "failed to signal_keys_channel: {} {}",
                err.code, err.description
            );
            return if err.code == FORBIDDEN {
                Ok(None)
            } else {
                sleep(Duration::from_secs(1)).await;
                Box::pin(self.signal_keys_channel(blaze_message)).await
            };
        }
        Ok(Some(bm))
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
            let address = format!("{}:{}", recipient_id, sid);
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
                &cid,
                &format!("{}:{}", uid, SignalProtocol::device_id(Some(sid))?),
            )
            .await?;

        Ok(())
    }

    pub async fn send_sender_key(&self, cid: &str, uid: &str, sid: &str) -> Result<bool> {
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

        if let Some(key) = keys.first() {
            self.signal_protocol
                .process_session(uid, key)
                .await
                .map_err(|e| anyhow!("failed to process session: {e}"))?
        } else {
            self.database
                .participant_session_dao
                .insert(
                    cid,
                    &[UserSession {
                        user_id: uid.to_string(),
                        session_id: sid.to_string(),
                        platform: None,
                        public_key: None,
                    }],
                )
                .await?;
            return Ok(false);
        }

        let (encrypted, no_key) = self
            .signal_protocol
            .encrypt_sender_key(cid, uid, SignalProtocol::device_id(Some(sid))?)
            .await
            .map_err(|e| anyhow!("failed to encrypt sender key: {e}"))?;
        if no_key {
            return Ok(false);
        }
        let messages = vec![BlazeSignalKeyMessage {
            message_id: Uuid::new_v4().to_string(),
            recipient_id: uid.to_string(),
            data: encrypted,
            session_id: Some(sid.to_string()),
        }];
        let check_sum = self.get_check_sum(cid).await?;
        let bm = BlazeMessage::new_signal_key_message(cid.to_string(), messages, check_sum);
        let result = self.deliver(bm).await?;
        if result.retry {
            return Box::pin(self.send_sender_key(cid, uid, sid)).await;
        }
        if result.success {
            self.database
                .participant_session_dao
                .insert_session(cid, uid, sid, 1)
                .await?;
        }

        Ok(result.success)
    }

    pub async fn get_check_sum(&self, cid: &str) -> Result<String> {
        let sessions = self
            .database
            .participant_session_dao
            .get_participant_sessions(cid)
            .await?;
        let mut sessions = sessions
            .into_iter()
            .map(|session| session.session_id)
            .collect::<Vec<_>>();
        sessions.sort();
        Ok(unique_object_id(&sessions).to_string())
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

        let result = self.blaze.send_message(msg.clone()).await?;

        if let Some(err) = &result.error {
            warn!(
                "failed to send message, code :{}, description: {}, {}",
                err.code,
                err.description,
                Backtrace::capture()
            );
            if err.code == CONVERSATION_CHECKSUM_INVALID_ERROR {
                let cid = msg.params.as_ref().and_then(|p| p.conversation_id.as_ref());
                if let Some(cid) = cid {
                    self.conversation.sync_conversation(cid).await?;
                }
                Ok(MessageResult {
                    success: false,
                    retry: true,
                    error_code: Some(err.code),
                })
            } else if err.code == FORBIDDEN || err.code == BAD_DATA {
                Ok(MessageResult {
                    success: false,
                    retry: false,
                    error_code: Some(err.code),
                })
            } else {
                // sleep for 10 seconds
                sleep(Duration::from_secs(1)).await;
                Box::pin(self.deliver(msg)).await
            }
        } else {
            Ok(MessageResult {
                success: true,
                retry: false,
                error_code: None,
            })
        }
    }
}
