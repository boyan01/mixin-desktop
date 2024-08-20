use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use sdk::ConversationCategory;

use crate::db::Error;

#[derive(Clone)]
pub struct ConversationDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(sqlx::Type, Clone, Eq, PartialEq)]
#[sqlx(transparent)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationStatus(pub(crate) i32);

impl ConversationStatus {
    pub const START: ConversationStatus = ConversationStatus(0);
    pub const FAILURE: ConversationStatus = ConversationStatus(1);
    pub const SUCCESS: ConversationStatus = ConversationStatus(2);
    pub const QUIT: ConversationStatus = ConversationStatus(3);
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub conversation_id: String,
    pub owner_id: Option<String>,
    pub category: Option<ConversationCategory>,
    pub name: String,
    pub icon_url: String,
    pub announcement: String,
    pub code_url: String,
    pub created_at: DateTime<Utc>,
    pub status: ConversationStatus,
    pub mute_until: DateTime<Utc>,
    pub expire_in: i64,
}

impl ConversationDao {
    pub async fn find_conversation_by_id(
        &self,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, Error> {
        let result = sqlx::query_as::<_, Conversation>(
            "SELECT * FROM conversations WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn insert(&self, conversation: &Conversation) -> Result<(), Error> {
        let _ = sqlx::query(
            r#"INSERT OR REPLACE INTO
         conversations (conversation_id, owner_id,
            category, name, icon_url, announcement,
            code_url, created_at, status, mute_until, expire_in)
            VALUES
            (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&conversation.conversation_id)
        .bind(&conversation.owner_id)
        .bind(&conversation.category)
        .bind(&conversation.name)
        .bind(&conversation.icon_url)
        .bind(&conversation.announcement)
        .bind(&conversation.code_url)
        .bind(conversation.created_at)
        .bind(&conversation.status)
        .bind(conversation.mute_until)
        .bind(conversation.expire_in)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update_status(&self, cid: &str, status: ConversationStatus) -> Result<(), Error> {
        let _ = sqlx::query("UPDATE conversations SET status = ? WHERE conversation_id = ?")
            .bind(status)
            .bind(cid)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_expire_in(&self, cid: &str, expire_in: i64) -> Result<(), Error> {
        let _ = sqlx::query("UPDATE conversations SET expire_in = ? WHERE conversation_id = ?")
            .bind(expire_in)
            .bind(cid)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
