use crate::db::mixin::expired_message::ExpiredMessageDao;
use crate::db::mixin::message::MessageDao;
use crate::db::mixin::message_mention::MessageMentionDao;
use crate::db::MixinDatabase;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachmentExtra {
    pub attachment_id: String,
    pub message_id: String,
    pub shareable: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

pub struct MessageService {
    pub database: Arc<MixinDatabase>,
    pub message_dao: MessageDao,
    pub message_mention_dao: MessageMentionDao,
    pub expired_message_dao: ExpiredMessageDao,
}

impl MessageService {
    pub fn new(database: Arc<MixinDatabase>) -> Self {
        Self {
            message_dao: database.message_dao.clone(),
            message_mention_dao: database.message_mention_dao.clone(),
            expired_message_dao: database.expired_message_dao.clone(),
            database,
        }
    }
}

impl MessageService {
    pub async fn mark_message_read(
        &self,
        messages: &[String],
        update_expired: bool,
    ) -> anyhow::Result<()> {
        self.message_dao.mark_message_read(messages).await?;
        if update_expired {
            self.expired_message_dao
                .mark_expired_message_read(messages)
                .await?;
        }
        Ok(())
    }
}
