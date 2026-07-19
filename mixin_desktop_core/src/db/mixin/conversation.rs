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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConversationListItem {
    pub conversation_id: String,
    pub owner_id: String,
    pub name: String,
    pub avatar_url: String,
    pub category: String,
    pub draft: String,
    pub status: i32,
    pub last_read_message_id: Option<String>,
    pub last_message: String,
    pub last_message_category: Option<String>,
    pub last_message_status: Option<String>,
    pub last_message_sender_id: Option<String>,
    pub last_message_sender_name: Option<String>,
    pub last_message_action: Option<String>,
    pub last_message_participant_id: Option<String>,
    pub last_message_participant_name: Option<String>,
    pub last_message_created_at: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub unseen_count: i64,
    pub mention_count: i64,
    pub is_muted: bool,
    pub is_verified: bool,
    pub is_scam: bool,
    pub is_bot: bool,
    pub is_bot_group: bool,
    pub membership: Option<String>,
    pub is_pinned: bool,
    pub relationship: String,
    pub identity_number: String,
    pub circle_ids: String,
    pub participant_count: i64,
    pub group_avatar_data: String,
}

impl ConversationListItem {
    pub fn updated_at_millis(&self) -> i64 {
        self.last_message_created_at
            .unwrap_or_else(|| self.created_at.timestamp_millis())
    }
}

impl ConversationDao {
    pub async fn count_items(
        &self,
        category: &str,
        circle_id: Option<&str>,
        keyword: &str,
        unseen_only: bool,
    ) -> Result<i64, Error> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
SELECT COUNT(*)
FROM conversations conversation
INNER JOIN users owner ON owner.user_id = conversation.owner_id
LEFT JOIN messages last_message ON last_message.message_id = conversation.last_message_id
WHERE conversation.category IN ('CONTACT', 'GROUP')
  AND (
      ?1 = 'chats'
      OR (?1 = 'contacts' AND conversation.category = 'CONTACT'
          AND owner.relationship = 'FRIEND' AND owner.app_id IS NULL)
      OR (?1 = 'groups' AND conversation.category = 'GROUP')
      OR (?1 = 'bots' AND conversation.category = 'CONTACT' AND owner.app_id IS NOT NULL)
      OR (?1 = 'strangers' AND conversation.category = 'CONTACT'
          AND owner.relationship = 'STRANGER' AND owner.app_id IS NULL)
      OR (?1 = 'circle' AND EXISTS (
          SELECT 1 FROM circle_conversations circle_conversation
          WHERE circle_conversation.conversation_id = conversation.conversation_id
            AND circle_conversation.circle_id = ?2
      ))
  )
  AND (?3 = FALSE OR conversation.unseen_message_count > 0)
  AND (
      ?4 = ''
      OR CASE WHEN conversation.category = 'GROUP'
              THEN conversation.name ELSE owner.full_name END LIKE '%' || ?4 || '%' COLLATE NOCASE
      OR owner.identity_number LIKE '%' || ?4 || '%' COLLATE NOCASE
      OR COALESCE(last_message.content, '') LIKE '%' || ?4 || '%' COLLATE NOCASE
  )
            "#,
        )
        .bind(category)
        .bind(circle_id.unwrap_or_default())
        .bind(unseen_only)
        .bind(keyword.trim())
        .fetch_one(&self.0)
        .await?;
        Ok(count)
    }

    pub async fn list_items(
        &self,
        category: &str,
        circle_id: Option<&str>,
        keyword: &str,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationListItem>, Error> {
        let result = sqlx::query_as::<_, ConversationListItem>(
            r#"
SELECT conversation.conversation_id,
       COALESCE(conversation.owner_id, '') AS owner_id,
       CASE
           WHEN conversation.category = 'GROUP' THEN conversation.name
           ELSE owner.full_name
       END AS name,
       CASE
           WHEN conversation.category = 'GROUP' THEN conversation.icon_url
           ELSE owner.avatar_url
       END AS avatar_url,
       conversation.category,
       COALESCE(conversation.draft, '') AS draft,
       conversation.status AS status,
       conversation.last_read_message_id,
       COALESCE(last_message.content, '') AS last_message,
       last_message.category AS last_message_category,
       last_message.status AS last_message_status,
       last_message.user_id AS last_message_sender_id,
       last_message_sender.full_name AS last_message_sender_name,
       last_message.action AS last_message_action,
       last_message.participant_id AS last_message_participant_id,
       last_message_participant.full_name AS last_message_participant_name,
       conversation.last_message_created_at,
       conversation.created_at,
       COALESCE(conversation.unseen_message_count, 0) AS unseen_count,
       (
           SELECT COUNT(1)
           FROM message_mentions mention
           WHERE mention.conversation_id = conversation.conversation_id
             AND COALESCE(mention.has_read, FALSE) = FALSE
       ) AS mention_count,
       CASE
           WHEN conversation.category = 'GROUP'
               THEN COALESCE(conversation.mute_until > CURRENT_TIMESTAMP, FALSE)
           ELSE COALESCE(owner.mute_until > CURRENT_TIMESTAMP, FALSE)
       END AS is_muted,
       COALESCE(owner.is_verified, FALSE) AS is_verified,
       COALESCE(owner.is_scam, FALSE) AS is_scam,
       (conversation.category = 'CONTACT' AND owner.app_id IS NOT NULL) AS is_bot,
       (conversation.category = 'CONTACT' AND owner.app_id IS NOT NULL AND EXISTS (
           SELECT 1
           FROM messages bot_group_message
           WHERE bot_group_message.conversation_id = conversation.conversation_id
             AND bot_group_message.user_id != conversation.owner_id
       )) AS is_bot_group,
       CASE WHEN conversation.category = 'CONTACT' THEN owner.membership ELSE NULL END AS membership,
       conversation.pin_time IS NOT NULL AS is_pinned,
       COALESCE(owner.relationship, '') AS relationship,
       COALESCE(owner.identity_number, '') AS identity_number,
       COALESCE((
           SELECT GROUP_CONCAT(circle_conversation.circle_id)
           FROM circle_conversations circle_conversation
           WHERE circle_conversation.conversation_id = conversation.conversation_id
       ), '') AS circle_ids,
       CASE WHEN conversation.category = 'GROUP' THEN (
           SELECT COUNT(1)
           FROM participants participant_count
           WHERE participant_count.conversation_id = conversation.conversation_id
       ) ELSE 0 END AS participant_count,
       CASE WHEN conversation.category = 'GROUP' THEN COALESCE((
           SELECT GROUP_CONCAT(
               avatar_user.user_id || CHAR(31) ||
               COALESCE(avatar_user.full_name, '') || CHAR(31) ||
               COALESCE(avatar_user.avatar_url, ''),
               CHAR(30)
           )
           FROM (
               SELECT user.user_id, user.full_name, user.avatar_url
               FROM participants participant
               INNER JOIN users user ON user.user_id = participant.user_id
               WHERE participant.conversation_id = conversation.conversation_id
               ORDER BY participant.created_at DESC
               LIMIT 4
           ) avatar_user
       ), '') ELSE '' END AS group_avatar_data
FROM conversations conversation
INNER JOIN users owner ON owner.user_id = conversation.owner_id
LEFT JOIN messages last_message ON last_message.message_id = conversation.last_message_id
LEFT JOIN users last_message_sender ON last_message_sender.user_id = last_message.user_id
LEFT JOIN users last_message_participant ON last_message_participant.user_id = last_message.participant_id
WHERE conversation.category IN ('CONTACT', 'GROUP')
  AND (
      ?1 = 'chats'
      OR (?1 = 'contacts' AND conversation.category = 'CONTACT'
          AND owner.relationship = 'FRIEND' AND owner.app_id IS NULL)
      OR (?1 = 'groups' AND conversation.category = 'GROUP')
      OR (?1 = 'bots' AND conversation.category = 'CONTACT' AND owner.app_id IS NOT NULL)
      OR (?1 = 'strangers' AND conversation.category = 'CONTACT'
          AND owner.relationship = 'STRANGER' AND owner.app_id IS NULL)
      OR (?1 = 'circle' AND EXISTS (
          SELECT 1 FROM circle_conversations circle_conversation
          WHERE circle_conversation.conversation_id = conversation.conversation_id
            AND circle_conversation.circle_id = ?2
      ))
  )
  AND (?3 = FALSE OR conversation.unseen_message_count > 0)
  AND (
      ?4 = ''
      OR CASE WHEN conversation.category = 'GROUP'
              THEN conversation.name ELSE owner.full_name END LIKE '%' || ?4 || '%' COLLATE NOCASE
      OR owner.identity_number LIKE '%' || ?4 || '%' COLLATE NOCASE
      OR COALESCE(last_message.content, '') LIKE '%' || ?4 || '%' COLLATE NOCASE
  )
ORDER BY conversation.pin_time DESC,
         (conversation.status != 3 AND LENGTH(COALESCE(conversation.draft, '')) > 0) DESC,
         conversation.last_message_created_at DESC,
         conversation.created_at DESC
LIMIT ?5 OFFSET ?6
            "#,
        )
        .bind(category)
        .bind(circle_id.unwrap_or_default())
        .bind(unseen_only)
        .bind(keyword.trim())
        .bind(limit.clamp(1, 500))
        .bind(offset.max(0))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn set_pinned(&self, conversation_id: &str, pinned: bool) -> Result<(), Error> {
        sqlx::query("UPDATE conversations SET pin_time = ? WHERE conversation_id = ?")
            .bind(pinned.then(Utc::now))
            .bind(conversation_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn set_mute_until(
        &self,
        conversation_id: &str,
        owner_id: &str,
        category: &str,
        mute_until: DateTime<Utc>,
    ) -> Result<(), Error> {
        if category == "GROUP" {
            sqlx::query("UPDATE conversations SET mute_until = ? WHERE conversation_id = ?")
                .bind(mute_until)
                .bind(conversation_id)
                .execute(&self.0)
                .await?;
        } else {
            sqlx::query("UPDATE users SET mute_until = ? WHERE user_id = ?")
                .bind(mute_until)
                .bind(owner_id)
                .execute(&self.0)
                .await?;
        }
        Ok(())
    }

    pub async fn delete_local(&self, conversation_id: &str) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        crate::db::mixin::message_fts::delete_conversation_fts(&mut transaction, conversation_id)
            .await?;
        for query in [
            "DELETE FROM message_mentions WHERE conversation_id = ?",
            "DELETE FROM pin_messages WHERE conversation_id = ?",
            "DELETE FROM messages WHERE conversation_id = ?",
            "DELETE FROM conversations WHERE conversation_id = ?",
        ] {
            sqlx::query(query)
                .bind(conversation_id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

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
            r#"INSERT INTO conversations (
                conversation_id, owner_id, category, name, icon_url, announcement,
                code_url, created_at, status, mute_until, expire_in
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(conversation_id) DO UPDATE SET
                owner_id = excluded.owner_id,
                category = excluded.category,
                name = excluded.name,
                icon_url = excluded.icon_url,
                announcement = excluded.announcement,
                code_url = excluded.code_url,
                created_at = excluded.created_at,
                status = excluded.status,
                mute_until = excluded.mute_until,
                expire_in = excluded.expire_in
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
    async fn refreshing_conversation_preserves_local_list_state() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let now = Utc::now();
        let mut conversation = Conversation {
            conversation_id: "conversation".into(),
            owner_id: Some("owner".into()),
            category: Some(ConversationCategory::Contact),
            name: "Old name".into(),
            icon_url: String::new(),
            announcement: String::new(),
            code_url: String::new(),
            created_at: now,
            status: ConversationStatus::SUCCESS,
            mute_until: now,
            expire_in: 0,
        };
        database
            .conversation_dao
            .insert(&conversation)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE conversations SET pin_time = ?, last_message_id = ?, \
             last_message_created_at = ?, last_read_message_id = ?, \
             unseen_message_count = ?, draft = ? WHERE conversation_id = ?",
        )
        .bind(now)
        .bind("last-message")
        .bind(now.timestamp_millis())
        .bind("last-read-message")
        .bind(3_i64)
        .bind("draft")
        .bind(&conversation.conversation_id)
        .execute(&database.conversation_dao.0)
        .await
        .unwrap();

        conversation.name = "New name".into();
        conversation.status = ConversationStatus::QUIT;
        conversation.expire_in = 60;
        database
            .conversation_dao
            .insert(&conversation)
            .await
            .unwrap();

        let row = sqlx::query_as::<
            _,
            (
                String,
                i32,
                i64,
                Option<String>,
                Option<i64>,
                Option<String>,
                Option<i64>,
                Option<String>,
                Option<DateTime<Utc>>,
            ),
        >(
            "SELECT name, status, expire_in, last_message_id, last_message_created_at, \
             last_read_message_id, unseen_message_count, draft, pin_time \
             FROM conversations WHERE conversation_id = ?",
        )
        .bind(&conversation.conversation_id)
        .fetch_one(&database.conversation_dao.0)
        .await
        .unwrap();
        assert_eq!(
            row,
            (
                "New name".into(),
                ConversationStatus::QUIT.0,
                60,
                Some("last-message".into()),
                Some(now.timestamp_millis()),
                Some("last-read-message".into()),
                Some(3),
                Some("draft".into()),
                Some(now),
            )
        );
    }

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

    #[tokio::test]
    async fn lists_conversations_for_flutter() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO users (user_id, identity_number, full_name, avatar_url, \
             is_verified, created_at, mute_until) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("other")
        .bind("7000")
        .bind("Mixin User")
        .bind("https://example.com/avatar.png")
        .bind(true)
        .bind(now)
        .bind(now + TimeDelta::hours(1))
        .execute(&database.conversation_dao.0)
        .await
        .unwrap();
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
        let message = Message {
            message_id: "message".into(),
            conversation_id: "conversation".into(),
            user_id: "other".into(),
            category: "PLAIN_TEXT".into(),
            content: Some("Hello from Rust".into()),
            status: MessageStatus::Sent,
            created_at: now.naive_utc(),
            ..Message::default()
        };
        database.message_dao.insert_message(&message).await.unwrap();
        database
            .conversation_dao
            .update_for_message(&message, "me")
            .await
            .unwrap();
        sqlx::query("UPDATE conversations SET last_read_message_id = ? WHERE conversation_id = ?")
            .bind("read-boundary")
            .bind("conversation")
            .execute(&database.conversation_dao.0)
            .await
            .unwrap();

        let count = database
            .conversation_dao
            .count_items("chats", None, "", false)
            .await
            .unwrap();
        let items = database
            .conversation_dao
            .list_items("chats", None, "", false, 10, 0)
            .await
            .unwrap();

        assert_eq!(count, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Mixin User");
        assert_eq!(items[0].last_message, "Hello from Rust");
        assert_eq!(
            items[0].last_read_message_id.as_deref(),
            Some("read-boundary")
        );
        assert_eq!(items[0].unseen_count, 1);
        assert!(items[0].is_muted);
        assert!(items[0].is_verified);
        assert_eq!(items[0].updated_at_millis(), now.timestamp_millis());
        assert_eq!(
            database
                .conversation_dao
                .count_items("chats", None, "Mixin", true)
                .await
                .unwrap(),
            1
        );
        assert!(database
            .conversation_dao
            .list_items("chats", None, "", false, 10, 1)
            .await
            .unwrap()
            .is_empty());

        database
            .message_fts_dao
            .upsert("message", "conversation", "Hello from Rust")
            .await
            .unwrap();
        database
            .conversation_dao
            .delete_local("conversation")
            .await
            .unwrap();
        let remaining: i64 = sqlx::query_scalar(
            "SELECT (SELECT COUNT(*) FROM conversations) + \
             (SELECT COUNT(*) FROM messages) + \
             (SELECT COUNT(*) FROM fts.messages_metas)",
        )
        .fetch_one(&database.conversation_dao.0)
        .await
        .unwrap();
        assert_eq!(remaining, 0);
    }
}
