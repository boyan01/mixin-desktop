use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use sdk::ConversationCategory;

use crate::db::mixin::message::Message;
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

    pub async fn update_for_message(
        &self,
        message: &Message,
        current_user_id: &str,
    ) -> Result<(), Error> {
        if message.user_id == current_user_id {
            sqlx::query(
                "UPDATE conversations SET last_message_id = ?, last_message_created_at = ?, \
                 draft = '' WHERE conversation_id = ?",
            )
            .bind(&message.message_id)
            .bind(message.created_at.and_utc().timestamp_millis())
            .bind(&message.conversation_id)
            .execute(&self.0)
            .await?;
            return Ok(());
        }

        let latest = sqlx::query_as::<_, (String, NaiveDateTime)>(
            "SELECT message_id, created_at FROM messages WHERE conversation_id = ? \
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(&message.conversation_id)
        .fetch_optional(&self.0)
        .await?;
        let unseen: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM messages WHERE conversation_id = ? \
             AND status IN ('SENT', 'DELIVERED') AND user_id != ?",
        )
        .bind(&message.conversation_id)
        .bind(current_user_id)
        .fetch_one(&self.0)
        .await?;
        let (last_message_id, last_message_created_at) = latest
            .map(|(message_id, created_at)| {
                (
                    Some(message_id),
                    Some(created_at.and_utc().timestamp_millis()),
                )
            })
            .unwrap_or((None, None));
        sqlx::query(
            "UPDATE conversations SET unseen_message_count = ?, last_message_id = ?, \
             last_message_created_at = ? WHERE conversation_id = ?",
        )
        .bind(unseen)
        .bind(last_message_id)
        .bind(last_message_created_at)
        .bind(&message.conversation_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update_read_state(
        &self,
        conversation_id: &str,
        current_user_id: &str,
    ) -> Result<(), Error> {
        let last_read_message_id = sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND status = 'READ' \
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(&self.0)
        .await?;
        let unseen: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM messages WHERE conversation_id = ? \
             AND status IN ('SENT', 'DELIVERED') AND user_id != ?",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_one(&self.0)
        .await?;
        sqlx::query(
            "UPDATE conversations SET last_read_message_id = ?, unseen_message_count = ? \
             WHERE conversation_id = ?",
        )
        .bind(last_read_message_id)
        .bind(unseen)
        .bind(conversation_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use sdk::MessageStatus;

    use super::*;
    use crate::db::MixinDatabase;

    #[tokio::test]
    async fn updates_last_message_and_unseen_count_like_flutter() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let now = Utc::now();
        database
            .conversation_dao
            .insert(&Conversation {
                conversation_id: "conversation".into(),
                owner_id: Some("other".into()),
                category: Some(ConversationCategory::Contact),
                name: String::new(),
                icon_url: String::new(),
                announcement: String::new(),
                code_url: String::new(),
                created_at: now,
                status: ConversationStatus::SUCCESS,
                mute_until: now,
                expire_in: 0,
            })
            .await
            .unwrap();

        let newest = Message {
            message_id: "newest".into(),
            conversation_id: "conversation".into(),
            user_id: "other".into(),
            category: "PLAIN_TEXT".into(),
            status: MessageStatus::Sent,
            created_at: now.naive_utc(),
            ..Message::default()
        };
        let older = Message {
            message_id: "older".into(),
            created_at: (now - TimeDelta::seconds(1)).naive_utc(),
            ..newest.clone()
        };
        for message in [&newest, &older] {
            database.message_dao.insert_message(message).await.unwrap();
            database
                .conversation_dao
                .update_for_message(message, "me")
                .await
                .unwrap();
        }

        let row = sqlx::query_as::<_, (Option<String>, Option<i64>, Option<i64>)>(
            "SELECT last_message_id, last_message_created_at, unseen_message_count \
             FROM conversations WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.conversation_dao.0)
        .await
        .unwrap();
        assert_eq!(
            row,
            (Some("newest".into()), Some(now.timestamp_millis()), Some(2))
        );

        database
            .message_dao
            .mark_message_read(&["newest".into()])
            .await
            .unwrap();
        database
            .conversation_dao
            .update_read_state("conversation", "me")
            .await
            .unwrap();
        let row = sqlx::query_as::<_, (Option<String>, Option<i64>)>(
            "SELECT last_read_message_id, unseen_message_count \
             FROM conversations WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.conversation_dao.0)
        .await
        .unwrap();
        assert_eq!(row, (Some("newest".into()), Some(1)));

        let own = Message {
            message_id: "own".into(),
            conversation_id: "conversation".into(),
            user_id: "me".into(),
            category: "PLAIN_TEXT".into(),
            status: MessageStatus::Sent,
            created_at: (now + TimeDelta::seconds(1)).naive_utc(),
            ..Message::default()
        };
        database.message_dao.insert_message(&own).await.unwrap();
        database
            .conversation_dao
            .update_for_message(&own, "me")
            .await
            .unwrap();
        let row = sqlx::query_as::<_, (Option<String>, Option<i64>, Option<String>)>(
            "SELECT last_message_id, last_message_created_at, draft \
             FROM conversations WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.conversation_dao.0)
        .await
        .unwrap();
        assert_eq!(
            row,
            (
                Some("own".into()),
                Some((now + TimeDelta::seconds(1)).timestamp_millis()),
                Some(String::new()),
            )
        );
    }
}
