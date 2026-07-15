use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use futures::StreamExt;
use log::{error, info, warn};
use serde::Deserialize;
use strum_macros::Display;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::interval;
use tokio_stream::wrappers::ReceiverStream;

use sdk::err::error_code::{BAD_DATA, FORBIDDEN};
use sdk::message_category::MessageCategory;
use sdk::{
    message_category, AttachmentMessage, BlazeAckMessage, BlazeMessage, BlazeMessageParam, Client,
    MessageStatus, PlainJsonMessage, ACKNOWLEDGE_MESSAGE_RECEIPTS, CREATE_MESSAGE, PIN_MESSAGE,
    RECALL_MESSAGE, SENDING_MESSAGE,
};

use crate::core::constants::TEAM_MIXIN_USER_ID;
use crate::core::crypto::encrypted_protocol;
use crate::core::message::sender::{MessageResult, MessageSender};
use crate::core::model::{AttachmentExtra, ConversationService};
use crate::core::util::generate_conversation_id;
use crate::db::mixin::job::{
    Job, JobDao, SYNC_INSCRIPTION_MESSAGE, UPDATE_ASSET, UPDATE_STICKER, UPDATE_TOKEN,
};
use crate::db::MixinDatabase;

const MAX_TEXT_LENGTH: usize = 64 * 1024;

pub struct JobService {
    job_dao: JobDao,
    ack_job_signer: Sender<()>,
    session_ack_job_signer: Sender<()>,
    work_job_signer: Sender<()>,
    params: Mutex<Option<JobParams>>,
}

impl JobService {
    pub fn new(
        database: Arc<MixinDatabase>,
        message_sender: Arc<MessageSender>,
        client: Arc<Client>,
        user_id: String,
        primary_session_id: Option<String>,
        private_key: Vec<u8>,
        session_id: String,
    ) -> Self {
        let (ack_sender, ack_receiver) = channel(1);
        let (session_ack_sender, session_ack_receiver) = channel(1);
        let (work_sender, work_receiver) = channel(1);

        let params = JobParams {
            database: database.clone(),
            client: client.clone(),
            receiver: HashMap::from([
                (JobCategory::Ack, ack_receiver),
                (JobCategory::SessionAck, session_ack_receiver),
                (JobCategory::Work, work_receiver),
            ]),
            user_id,
            primary_session_id,
            message_sender,
            private_key,
            session_id,
        };
        JobService {
            job_dao: database.job_dao.clone(),
            ack_job_signer: ack_sender,
            session_ack_job_signer: session_ack_sender,
            work_job_signer: work_sender,
            params: Mutex::new(Some(params)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let params = self
            .params
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow!("job service is already running"))?;
        start_all_jobs(params).await;
        Ok(())
    }

    pub async fn add(&self, job: &Job) -> Result<()> {
        let signaler = match job.action.as_str() {
            ACKNOWLEDGE_MESSAGE_RECEIPTS => &self.ack_job_signer,
            CREATE_MESSAGE => &self.session_ack_job_signer,
            SENDING_MESSAGE
            | PIN_MESSAGE
            | RECALL_MESSAGE
            | UPDATE_ASSET
            | UPDATE_TOKEN
            | UPDATE_STICKER
            | SYNC_INSCRIPTION_MESSAGE => &self.work_job_signer,
            _ => bail!("unknown job action: {}", job.action),
        };
        self.job_dao.insert_job(job).await?;
        let _ = signaler.try_send(());

        Ok(())
    }
}

#[derive(Debug, Display, Eq, PartialEq, Hash)]
enum JobCategory {
    Ack,
    SessionAck,
    Work,
}

struct JobParams {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
    receiver: HashMap<JobCategory, Receiver<()>>,
    user_id: String,
    primary_session_id: Option<String>,
    message_sender: Arc<MessageSender>,
    private_key: Vec<u8>,
    session_id: String,
}

async fn start_all_jobs(mut params: JobParams) {
    tokio::join!(
        run_job(
            params.receiver.remove(&JobCategory::Ack),
            AckJobRunner {
                database: params.database.clone(),
                client: params.client.clone(),
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::SessionAck),
            SessionAckJob {
                database: params.database.clone(),
                user_id: params.user_id.clone(),
                sender: params.message_sender.clone(),
                primary_session_id: params.primary_session_id,
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::Work),
            WorkJobRunner {
                database: params.database,
                client: params.client,
                user_id: params.user_id,
                session_id: params.session_id,
                private_key: params.private_key,
                sender: params.message_sender,
            },
        )
    );
}

async fn run_job(receiver: Option<Receiver<()>>, trigger: impl JobTrigger) {
    let receiver = match receiver {
        Some(r) => r,
        None => {
            error!("{} job receiver is None, exit", trigger.category());
            return;
        }
    };
    let mut stream = ReceiverStream::new(receiver);
    let mut interval = interval(Duration::from_secs(42));
    loop {
        tokio::select! {
           msg = stream.next() => {
                if msg.is_none() {
                    error!("job receiver closed");
                    break;
                }
            }
            _ = interval.tick() => {
                info!("time out");
            }
        }
        if let Err(err) = trigger.trigger().await {
            error!("failed to trigger job {}: {:?}", trigger.category(), err);
        }
    }
}

trait JobTrigger {
    async fn trigger(&self) -> Result<()>;
    fn category(&self) -> JobCategory;
}

struct AckJobRunner {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
}

struct SessionAckJob {
    database: Arc<MixinDatabase>,
    user_id: String,
    sender: Arc<MessageSender>,
    primary_session_id: Option<String>,
}

struct WorkJobRunner {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
    user_id: String,
    session_id: String,
    private_key: Vec<u8>,
    sender: Arc<MessageSender>,
}

#[derive(Debug, Deserialize)]
struct SendingPayload {
    message_id: String,
    recipient_id: Option<String>,
    session_id: Option<String>,
    #[serde(default)]
    resend: bool,
    #[serde(default)]
    silent: bool,
    #[serde(default, alias = "expireIn")]
    expire_in: i64,
}

impl JobTrigger for AckJobRunner {
    async fn trigger(&self) -> Result<()> {
        let jobs = self.database.job_dao.ack_jobs().await?;
        info!("trigger ack job runner: {:?}", jobs.len());
        if jobs.is_empty() {
            return Ok(());
        }
        let mut job_ids = vec![];
        let mut acks = vec![];
        for job in jobs {
            if let Some(m) = job
                .blaze_message
                .and_then(|m| serde_json::from_str::<BlazeAckMessage>(&m).ok())
            {
                acks.push(m);
            } else {
                error!("AckJobRunner: failed to parse message: {:?}", job.job_id);
            }
            job_ids.push(job.job_id);
        }

        let result = self.client.message_api.acknowledgements(&acks).await;
        if let Err(err) = result {
            error!("failed to ack messages: {:?}", err);
        } else {
            info!("ack messages success");
            self.database.job_dao.delete_jobs(&job_ids).await?;
        }
        Ok(())
    }

    fn category(&self) -> JobCategory {
        JobCategory::Ack
    }
}

impl JobTrigger for SessionAckJob {
    async fn trigger(&self) -> Result<()> {
        if self.primary_session_id.is_none() {
            warn!("primary session id is None, skip session ack job");
            return Ok(());
        };

        let conversation_id = self
            .database
            .participant_dao
            .find_any_joined_conversation_id(&self.user_id)
            .await?
            .unwrap_or_else(|| {
                generate_conversation_id(&self.user_id, TEAM_MIXIN_USER_ID).to_string()
            });

        let jobs = self.database.job_dao.session_ack_jobs().await?;
        if jobs.is_empty() {
            return Ok(());
        }
        let mut job_ids = vec![];
        let mut acks = vec![];
        for job in jobs {
            if let Some(m) = job
                .blaze_message
                .and_then(|m| serde_json::from_str::<BlazeAckMessage>(&m).ok())
            {
                acks.push(m);
            } else {
                error!("failed to parse message: {:?}", job.job_id);
            }
            job_ids.push(job.job_id);
        }

        let plain_text = PlainJsonMessage {
            action: ACKNOWLEDGE_MESSAGE_RECEIPTS.to_string(),
            messages: None,
            user_id: None,
            message_id: None,
            session_id: None,
            content: None,
            ack_messages: Some(acks),
        };

        let encoded = serde_json::to_vec(&plain_text)?;
        let encoded = Base64::encode_string(&encoded);

        let bm = BlazeMessage::new_plain_json(
            &conversation_id,
            self.sender.get_check_sum(&conversation_id).await?,
            &self.user_id,
            encoded,
            self.primary_session_id.clone(),
        );

        let result = self.sender.deliver(bm).await?;

        if is_terminal_result(&result) {
            self.database.job_dao.delete_jobs(&job_ids).await?;
        } else if let Some(err) = result.error_code {
            error!("failed to ack messages, code: {:?}", err);
        }
        Ok(())
    }

    fn category(&self) -> JobCategory {
        JobCategory::SessionAck
    }
}

impl JobTrigger for WorkJobRunner {
    async fn trigger(&self) -> Result<()> {
        self.run_sending_jobs().await?;
        self.run_update_asset_jobs().await?;
        self.run_update_token_jobs().await?;
        self.run_update_sticker_jobs().await?;
        self.run_sync_inscription_jobs().await?;
        Ok(())
    }

    fn category(&self) -> JobCategory {
        JobCategory::Work
    }
}

impl WorkJobRunner {
    async fn run_sending_jobs(&self) -> Result<()> {
        for job in self.database.job_dao.sending_jobs().await? {
            let result = match job.action.as_str() {
                PIN_MESSAGE => {
                    self.send_control_message(&job, message_category::MESSAGE_PIN)
                        .await
                }
                RECALL_MESSAGE => {
                    self.send_control_message(&job, message_category::MESSAGE_RECALL)
                        .await
                }
                SENDING_MESSAGE => self.send_user_message(&job).await,
                _ => Ok(()),
            };
            if let Err(err) = result {
                error!("failed to run sending job {}: {err}", job.job_id);
            }
        }
        Ok(())
    }

    async fn send_control_message(&self, job: &Job, category: &str) -> Result<()> {
        let conversation_id = job
            .conversation_id
            .as_deref()
            .ok_or_else(|| anyhow!("control job has no conversation id"))?;
        let content = job
            .blaze_message
            .as_deref()
            .ok_or_else(|| anyhow!("control job has no payload"))?;
        let result = self
            .sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(conversation_id.to_string()),
                conversation_checksum: Some(self.sender.get_check_sum(conversation_id).await?),
                message_id: Some(uuid::Uuid::new_v4().to_string()),
                category: Some(category.to_string()),
                data: Some(Base64::encode_string(content.as_bytes())),
                status: Some(MessageStatus::Sending.into()),
                ..BlazeMessageParam::default()
            }))
            .await?;
        if is_terminal_result(&result) {
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
        }
        Ok(())
    }

    async fn send_user_message(&self, job: &Job) -> Result<()> {
        let payload_text = job
            .blaze_message
            .as_deref()
            .ok_or_else(|| anyhow!("sending job has no payload"))?;
        let payload = serde_json::from_str::<SendingPayload>(payload_text).unwrap_or_else(|_| {
            SendingPayload {
                message_id: payload_text.to_string(),
                recipient_id: job.user_id.clone(),
                session_id: None,
                resend: job.resend_message_id.is_some(),
                silent: false,
                expire_in: 0,
            }
        });
        let resend = payload.resend || job.resend_message_id.is_some();
        let message = if resend {
            self.database
                .message_dao
                .find_message_by_id(&payload.message_id)
                .await?
        } else {
            self.database
                .message_dao
                .find_sending_message(&payload.message_id)
                .await?
        };
        let Some(message) = message else {
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(());
        };
        let Some(conversation) = self
            .database
            .conversation_dao
            .find_conversation_by_id(&message.conversation_id)
            .await?
        else {
            error!("conversation {} not found", message.conversation_id);
            return Ok(());
        };
        let conversation_service = ConversationService::new(
            self.database.clone(),
            self.client.clone(),
            self.user_id.clone(),
        );
        if let Err(error) = conversation_service
            .ensure_conversation_exists(&conversation)
            .await
        {
            if error.downcast_ref::<sdk::ApiError>().is_some_and(
                |error| matches!(error, sdk::ApiError::Server(error) if error.code == BAD_DATA),
            ) {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                return Ok(());
            }
            return Err(error);
        }
        let stored_content = message
            .content
            .as_deref()
            .ok_or_else(|| anyhow!("message {} has no content", message.message_id))?;
        let mut outbound_content = if message.category.is_transcript() {
            let mut transcripts = self
                .database
                .transcript_message_dao
                .find_by_transcript_id(&message.message_id)
                .await?;
            if transcripts.is_empty() {
                bail!("message {} has no transcript entries", message.message_id);
            }
            for transcript in &mut transcripts {
                sanitize_transcript_app_card(transcript);
            }
            serde_json::to_string(&transcripts)?
        } else {
            stored_content.to_string()
        };
        let mut sent_content = None;
        if message.category.is_text() || message.category.is_post() {
            outbound_content = truncate_utf16(&outbound_content, MAX_TEXT_LENGTH).to_string();
            sent_content = Some(outbound_content.clone());
        } else if message.category.is_attachment() {
            if let Ok(decoded) = Base64::decode_vec(&outbound_content) {
                if let Ok(attachment) = serde_json::from_slice::<AttachmentMessage>(&decoded) {
                    sent_content = Some(serde_json::to_string(&AttachmentExtra {
                        attachment_id: attachment.attachment_id,
                        message_id: message.message_id.clone(),
                        shareable: attachment.shareable,
                        created_at: attachment.created_at,
                    })?);
                }
            }
        }
        let content = outbound_content.as_str();
        let mentions = if message.category.is_signal() && message.category.is_text() {
            let identity_numbers = mention_identity_numbers(content);
            let user_ids = self
                .database
                .user_dao
                .find_user_ids_by_identity_numbers(&identity_numbers)
                .await?;
            (!user_ids.is_empty()).then_some(user_ids)
        } else {
            None
        };
        if payload.expire_in < 0 {
            self.database
                .message_dao
                .update_message_status(&message.message_id, MessageStatus::Failed)
                .await?;
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(());
        }
        let expire_in = i32::try_from(payload.expire_in)
            .with_context(|| format!("invalid expire_in: {}", payload.expire_in))?;

        let result = if message.category.is_plain()
            || message.category.is_app_card()
            || message.category.is_app_button_group()
        {
            self.send_plain_message(
                &message,
                &message.category,
                payload.recipient_id.as_deref(),
                content,
                payload.silent,
                expire_in,
            )
            .await?
        } else if message.category.is_signal() {
            let content = if message.category.is_live() {
                Base64::encode_string(content.as_bytes())
            } else {
                content.to_string()
            };
            if resend {
                self.sender
                    .resend_signal_message(
                        &message.message_id,
                        &message.conversation_id,
                        payload
                            .recipient_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("resend job has no recipient"))?,
                        payload
                            .session_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("resend job has no session"))?,
                        &message.category,
                        &content,
                        message.quote_message_id.as_deref(),
                        mentions.as_deref(),
                        payload.silent,
                        expire_in,
                    )
                    .await?
            } else {
                self.sender
                    .send_signal_message(
                        &message.message_id,
                        &message.conversation_id,
                        &message.category,
                        &content,
                        message.quote_message_id.as_deref(),
                        mentions.as_deref(),
                        payload.silent,
                        expire_in,
                    )
                    .await?
            }
        } else if message.category.is_encrypted() {
            match self
                .send_encrypted_message(&message, &payload, content, expire_in)
                .await?
            {
                Some(result) => result,
                None => {
                    let category =
                        encrypted_to_plain_category(&message.category).ok_or_else(|| {
                            anyhow!("invalid encrypted category: {}", message.category)
                        })?;
                    self.database
                        .message_dao
                        .update_message_category(&message.message_id, &category)
                        .await?;
                    self.send_plain_message(
                        &message,
                        &category,
                        payload.recipient_id.as_deref(),
                        content,
                        payload.silent,
                        expire_in,
                    )
                    .await?
                }
            }
        } else {
            self.database
                .message_dao
                .update_message_status(&message.message_id, MessageStatus::Failed)
                .await?;
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(());
        };

        if is_terminal_result(&result) {
            if result.success && !resend {
                if let Some(content) = sent_content.as_deref() {
                    self.database
                        .message_dao
                        .update_message_content_and_status(
                            &message.message_id,
                            content,
                            MessageStatus::Sent,
                        )
                        .await?;
                } else {
                    self.database
                        .message_dao
                        .update_message_status(&message.message_id, MessageStatus::Sent)
                        .await?;
                }
                if payload.expire_in > 0 {
                    self.database
                        .expired_message_dao
                        .insert(
                            &message.message_id,
                            payload.expire_in,
                            Some(Utc::now().timestamp() + payload.expire_in),
                        )
                        .await?;
                }
            }
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
        }
        Ok(())
    }

    async fn send_plain_message(
        &self,
        message: &crate::db::mixin::message::Message,
        category: &String,
        recipient_id: Option<&str>,
        content: &str,
        silent: bool,
        expire_in: i32,
    ) -> Result<MessageResult> {
        let encoded = if category.is_text()
            || category.is_post()
            || category.is_live()
            || category.is_location()
            || category.is_transcript()
            || category.is_app_card()
            || category.is_app_button_group()
        {
            Base64::encode_string(content.as_bytes())
        } else {
            content.to_string()
        };
        self.sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(message.conversation_id.clone()),
                conversation_checksum: Some(
                    self.sender.get_check_sum(&message.conversation_id).await?,
                ),
                recipient_id: recipient_id.map(str::to_string),
                message_id: Some(message.message_id.clone()),
                category: Some(category.clone()),
                data: Some(encoded),
                quote_message_id: message.quote_message_id.clone(),
                silent: Some(silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            }))
            .await
    }

    async fn send_encrypted_message(
        &self,
        message: &crate::db::mixin::message::Message,
        payload: &SendingPayload,
        content: &str,
        expire_in: i32,
    ) -> Result<Option<MessageResult>> {
        let mut remote = self
            .database
            .participant_session_dao
            .participant_session_key_without_self(&message.conversation_id, &self.user_id)
            .await?;
        if remote.is_none() {
            ConversationService::new(
                self.database.clone(),
                self.client.clone(),
                self.user_id.clone(),
            )
            .refresh_conversation(&message.conversation_id)
            .await?;
            remote = self
                .database
                .participant_session_dao
                .participant_session_key_without_self(&message.conversation_id, &self.user_id)
                .await?;
        }
        let Some(remote) = remote else {
            return Ok(None);
        };
        let other_self = self
            .database
            .participant_session_dao
            .other_participant_session_key(
                &message.conversation_id,
                &self.user_id,
                &self.session_id,
            )
            .await?;

        let mut sessions = vec![(
            remote.session_id,
            Base64::decode_vec(
                remote
                    .public_key
                    .as_deref()
                    .ok_or_else(|| anyhow!("recipient session has no public key"))?,
            )?,
        )];
        if let Some(other) = other_self {
            sessions.push((
                other.session_id,
                Base64::decode_vec(
                    other
                        .public_key
                        .as_deref()
                        .ok_or_else(|| anyhow!("own session has no public key"))?,
                )?,
            ));
        }
        let session_refs = sessions
            .iter()
            .map(|(session_id, key)| (session_id.as_str(), key.as_slice()))
            .collect::<Vec<_>>();
        let plaintext = if message.category.is_attachment()
            || message.category.is_sticker()
            || message.category.is_contact()
            || message.category.is_live()
        {
            Base64::decode_vec(content)?
        } else {
            content.as_bytes().to_vec()
        };
        let encrypted =
            encrypted_protocol::encrypt_message(&self.private_key, &plaintext, &session_refs)?;
        let result = self
            .sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(message.conversation_id.clone()),
                conversation_checksum: Some(
                    self.sender.get_check_sum(&message.conversation_id).await?,
                ),
                message_id: Some(message.message_id.clone()),
                category: Some(message.category.clone()),
                data: Some(Base64::encode_string(&encrypted)),
                quote_message_id: message.quote_message_id.clone(),
                silent: Some(payload.silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            }))
            .await?;
        Ok(Some(result))
    }

    async fn run_update_asset_jobs(&self) -> Result<()> {
        for job in self.database.job_dao.update_asset_jobs().await? {
            let Some(asset_id) = job.blaze_message.as_deref() else {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                continue;
            };
            let result: Result<()> = async {
                let asset = self.client.asset_api.get_asset_by_id(asset_id).await?;
                let chain = self.client.asset_api.get_chain(&asset.chain_id).await?;
                self.database.asset_dao.insert_asset(&asset).await?;
                self.database.asset_dao.insert_chain(&chain).await?;
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                Ok(())
            }
            .await;
            if let Err(err) = result {
                warn!("failed to update asset {asset_id}: {err}");
            }
        }
        Ok(())
    }

    async fn run_update_token_jobs(&self) -> Result<()> {
        for job in self.database.job_dao.update_token_jobs().await? {
            let Some(asset_id) = job.blaze_message.as_deref() else {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                continue;
            };
            let result: Result<()> = async {
                let token = self.client.token_api.get_asset_by_id(asset_id).await?;
                let chain = self.client.asset_api.get_chain(&token.chain_id).await?;
                self.database.asset_dao.insert_token(&token).await?;
                self.database.asset_dao.insert_chain(&chain).await?;
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                Ok(())
            }
            .await;
            if let Err(err) = result {
                warn!("failed to update token {asset_id}: {err}");
            }
        }
        Ok(())
    }

    async fn run_update_sticker_jobs(&self) -> Result<()> {
        for job in self.database.job_dao.update_sticker_jobs().await? {
            let Some(sticker_id) = job.blaze_message.as_deref() else {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                continue;
            };
            let result: Result<()> =
                match self.client.account_api.get_sticker_by_id(sticker_id).await {
                    Ok(sticker) => self
                        .database
                        .sticker_dao
                        .insert(&sticker)
                        .await
                        .map_err(Into::into),
                    Err(error) => Err(error.into()),
                };
            match result {
                Ok(()) => {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                Err(error)
                    if error.downcast_ref::<sdk::ApiError>().is_some_and(
                        |error| matches!(error, sdk::ApiError::Server(error) if error.status == 404 || error.code == 404),
                    ) =>
                {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                Err(error) => {
                    self.database
                        .job_dao
                        .reschedule_job(
                            &job.job_id,
                            Utc::now().naive_utc() + sticker_backoff(job.run_count),
                            job.run_count.saturating_add(1),
                        )
                        .await?;
                    warn!("failed to update sticker {sticker_id}: {error}");
                }
            }
        }
        Ok(())
    }

    async fn run_sync_inscription_jobs(&self) -> Result<()> {
        for job in self
            .database
            .job_dao
            .sync_inscription_message_jobs()
            .await?
        {
            let Some(message_id) = job.blaze_message.as_deref() else {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                continue;
            };
            let result = self.sync_inscription_message(message_id).await;
            match result {
                Ok(()) => {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                Err(err) => warn!("failed to sync inscription message {message_id}: {err}"),
            }
        }
        Ok(())
    }

    async fn sync_inscription_message(&self, message_id: &str) -> Result<()> {
        let Some(message) = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
        else {
            warn!("inscription message not found: {message_id}");
            return Ok(());
        };
        let Some(hash) = message.content.as_deref() else {
            warn!("inscription message has no hash: {message_id}");
            return Ok(());
        };
        if let Err(error) = hex::decode(hash).context("invalid inscription hash") {
            warn!("failed to sync inscription message {message_id}: {error}");
            return Ok(());
        }

        let item = match self.database.inscription_dao.find_item(hash).await? {
            Some(item) => item,
            None => {
                let item = match self.client.token_api.get_inscription_item(hash).await {
                    Ok(item) => item,
                    Err(error) => {
                        warn!("failed to get inscription {hash}: {error}");
                        return Ok(());
                    }
                };
                self.database.inscription_dao.insert_item(&item).await?;
                item
            }
        };
        let collection = match self
            .database
            .inscription_dao
            .find_collection(&item.collection_hash)
            .await?
        {
            Some(collection) => collection,
            None => {
                let collection = match self
                    .client
                    .token_api
                    .get_inscription_collection(&item.collection_hash)
                    .await
                {
                    Ok(collection) => collection,
                    Err(error) => {
                        warn!(
                            "failed to get inscription collection {}: {error}",
                            item.collection_hash
                        );
                        return Ok(());
                    }
                };
                self.database
                    .inscription_dao
                    .insert_collection(&collection)
                    .await?;
                collection
            }
        };
        let content = serde_json::json!({
            "collection_hash": collection.collection_hash,
            "inscription_hash": item.inscription_hash,
            "sequence": item.sequence,
            "content_type": item.content_type,
            "content_url": item.content_url,
            "name": collection.name,
            "icon_url": collection.icon_url,
        })
        .to_string();
        self.database
            .message_dao
            .update_message_content_and_status(message_id, &content, message.status)
            .await?;
        Ok(())
    }
}

fn is_terminal_result(result: &MessageResult) -> bool {
    result.success || matches!(result.error_code, Some(BAD_DATA | FORBIDDEN))
}

fn sticker_backoff(run_count: i32) -> chrono::Duration {
    match run_count {
        i32::MIN..=0 => chrono::Duration::minutes(1),
        1 => chrono::Duration::minutes(5),
        2 => chrono::Duration::minutes(15),
        3 => chrono::Duration::hours(1),
        _ => chrono::Duration::hours(6),
    }
}

fn truncate_utf16(value: &str, max_code_units: usize) -> &str {
    if value.encode_utf16().count() <= max_code_units {
        return value;
    }
    let mut code_units = 0;
    let mut end = 0;
    for (index, character) in value.char_indices() {
        let next = code_units + character.len_utf16();
        if next > max_code_units {
            break;
        }
        code_units = next;
        end = index + character.len_utf8();
    }
    &value[..end]
}

fn mention_identity_numbers(content: &str) -> Vec<String> {
    let mut identities = Vec::new();
    for (index, _) in content.match_indices('@') {
        let identity = content[index + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if identity.len() >= 4 && !identities.contains(&identity) {
            identities.push(identity);
        }
    }
    identities
}

fn encrypted_to_plain_category(category: &str) -> Option<String> {
    category
        .strip_prefix("ENCRYPTED_")
        .map(|suffix| format!("PLAIN_{suffix}"))
}

fn sanitize_transcript_app_card(
    transcript: &mut crate::db::mixin::transcript_message::TranscriptMessage,
) {
    if !transcript.category.is_app_card() {
        return;
    }
    let Some(content) = transcript.content.as_mut() else {
        return;
    };
    let Ok(mut card) = serde_json::from_str::<serde_json::Value>(content) else {
        return;
    };
    if card["action"].as_str().unwrap_or_default().is_empty()
        && card["actions"].as_array().is_some_and(|actions| {
            actions
                .iter()
                .any(|action| !is_shareable_card_action(action))
        })
    {
        card.as_object_mut().map(|card| card.remove("actions"));
        if let Ok(sanitized) = serde_json::to_string(&card) {
            *content = sanitized;
        }
    }
}

fn is_shareable_card_action(action: &serde_json::Value) -> bool {
    let Some(action) = action.get("action").and_then(serde_json::Value::as_str) else {
        return false;
    };
    let Ok(uri) = url::Url::parse(action) else {
        return false;
    };
    let mixin_host = matches!(uri.host_str(), Some("mixin.one" | "www.mixin.one"));
    let web_send =
        mixin_host && uri.path_segments().and_then(|mut path| path.next()) == Some("send");
    let mixin_send = uri.scheme() == "mixin" && uri.host_str() == Some("send");
    let has_user = uri
        .query_pairs()
        .any(|(key, value)| key == "user" && !value.trim().is_empty());
    if (web_send || mixin_send) && has_user {
        return true;
    }
    matches!(uri.scheme(), "http" | "https") && !web_send
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::db::mixin::transcript_message::TranscriptMessage;

    use super::{
        encrypted_to_plain_category, is_terminal_result, mention_identity_numbers,
        sanitize_transcript_app_card, sticker_backoff, truncate_utf16,
    };
    use crate::core::message::sender::MessageResult;

    #[test]
    fn truncates_using_dart_utf16_length_semantics() {
        assert_eq!(truncate_utf16("ab😀c", 4), "ab😀");
        assert_eq!(truncate_utf16("ab😀c", 3), "ab");
        assert_eq!(truncate_utf16("short", 10), "short");
    }

    #[test]
    fn extracts_unique_flutter_style_mentions() {
        assert_eq!(
            mention_identity_numbers("hi @1234 @12 @1234 and @56789x"),
            vec!["1234", "56789"]
        );
    }

    #[test]
    fn maps_encrypted_category_to_flutter_plain_fallback() {
        assert_eq!(
            encrypted_to_plain_category("ENCRYPTED_TRANSCRIPT").as_deref(),
            Some("PLAIN_TRANSCRIPT")
        );
        assert_eq!(encrypted_to_plain_category("SIGNAL_TEXT"), None);
    }

    #[test]
    fn treats_forbidden_and_bad_data_as_terminal_delivery_results() {
        for code in [
            sdk::err::error_code::FORBIDDEN,
            sdk::err::error_code::BAD_DATA,
        ] {
            assert!(is_terminal_result(&MessageResult {
                success: false,
                retry: false,
                error_code: Some(code),
            }));
        }
    }

    #[test]
    fn uses_flutter_sticker_retry_backoff() {
        assert_eq!(sticker_backoff(0), chrono::Duration::minutes(1));
        assert_eq!(sticker_backoff(1), chrono::Duration::minutes(5));
        assert_eq!(sticker_backoff(2), chrono::Duration::minutes(15));
        assert_eq!(sticker_backoff(3), chrono::Duration::hours(1));
        assert_eq!(sticker_backoff(4), chrono::Duration::hours(6));
        assert_eq!(sticker_backoff(99), chrono::Duration::hours(6));
    }

    #[test]
    fn removes_unsafe_actions_from_forwarded_transcript_card() {
        let mut transcript = TranscriptMessage {
            transcript_id: "root".into(),
            message_id: "child".into(),
            user_id: None,
            user_full_name: None,
            category: sdk::message_category::APP_CARD.into(),
            created_at: Utc::now(),
            content: Some(
                r#"{"action":"","actions":[{"action":"https://mixin.one/send?conversation=id"}]}"#
                    .into(),
            ),
            media_url: None,
            media_name: None,
            media_size: None,
            media_width: None,
            media_height: None,
            media_mime_type: None,
            media_duration: None,
            media_status: None,
            media_waveform: None,
            thumb_image: None,
            thumb_url: None,
            media_key: None,
            media_digest: None,
            media_created_at: None,
            sticker_id: None,
            shared_user_id: None,
            mentions: None,
            quote_id: None,
            quote_content: None,
            caption: None,
        };

        sanitize_transcript_app_card(&mut transcript);

        let card: serde_json::Value = serde_json::from_str(&transcript.content.unwrap()).unwrap();
        assert!(card.get("actions").is_none());
    }
}
