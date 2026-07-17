use std::sync::Arc;

use anyhow::Result;
use base64ct::{Base64, Encoding};
use log::{error, warn};
use sdk::{BlazeAckMessage, BlazeMessage, PlainJsonMessage, ACKNOWLEDGE_MESSAGE_RECEIPTS};

use crate::core::constants::TEAM_MIXIN_USER_ID;
use crate::core::message::sender::MessageSender;
use crate::core::util::generate_conversation_id;
use crate::db::MixinDatabase;

use super::{is_terminal_result, JobCategory, JobTrigger};

pub(super) struct SessionAckJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) user_id: String,
    pub(super) sender: Arc<MessageSender>,
    pub(super) primary_session_id: Option<String>,
}

impl JobTrigger for SessionAckJobRunner {
    async fn trigger(&self) -> Result<bool> {
        if self.primary_session_id.is_none() {
            warn!("primary session id is None, skip session ack job");
            return Ok(false);
        };

        let conversation_id = self
            .database
            .participant_dao
            .find_any_joined_conversation_id(&self.user_id)
            .await?
            .unwrap_or_else(|| {
                generate_conversation_id(&self.user_id, TEAM_MIXIN_USER_ID).to_string()
            });

        loop {
            let jobs = self.database.job_dao.session_ack_jobs().await?;
            if jobs.is_empty() {
                return Ok(false);
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
            } else {
                if let Some(err) = result.error_code {
                    error!("failed to ack messages, code: {:?}", err);
                }
                return Ok(true);
            }
        }
    }

    fn category(&self) -> JobCategory {
        JobCategory::SessionAck
    }
}
