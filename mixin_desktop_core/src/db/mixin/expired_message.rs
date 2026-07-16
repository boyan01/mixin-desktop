use chrono::Utc;
use sqlx::{FromRow, Sqlite};

use db::Error;

use crate::db;
use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::util::{expand_var, expand_var_with_index, BindList, BindListForQuery};

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ExpiredMessage {
    pub message_id: String,
    pub expire_in: i64,
    pub expire_at: Option<i64>,
}

#[derive(Clone)]
pub struct ExpiredMessageDao(pub(crate) sqlx::Pool<Sqlite>);

impl ExpiredMessageDao {
    pub async fn insert(
        &self,
        message_id: &str,
        expire_in: i64,
        expire_at: Option<i64>,
    ) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO expired_messages (message_id, expire_in, expire_at) VALUES (?, ?, ?) \
             ON CONFLICT(message_id) DO UPDATE SET \
             expire_in = excluded.expire_in, expire_at = excluded.expire_at",
        )
        .bind(message_id)
        .bind(expire_in)
        .bind(expire_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update_message_expired_at(&self, data: &[(String, i64)]) -> Result<u64, Error> {
        let mut transaction = self.0.begin().await?;
        let mut rows_affected = 0;
        for (message_id, expire_at) in data {
            rows_affected += sqlx::query(
                "UPDATE expired_messages SET expire_at = ? WHERE message_id = ? \
                 AND (expire_at IS NULL OR expire_at > ?)",
            )
            .bind(expire_at)
            .bind(message_id)
            .bind(expire_at)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        }
        transaction.commit().await?;
        Ok(rows_affected)
    }

    pub async fn mark_expired_message_read(&self, message_ids: &[String]) -> Result<u64, Error> {
        let now = Utc::now().timestamp();
        let mut rows_affected = 0;
        for chunk in message_ids.chunks(MARK_LIMIT - 1) {
            let sql = format!(
                "UPDATE expired_messages SET expire_at = CAST((?1 + expire_in) AS INTEGER) \
                 WHERE (expire_at > (?1 + expire_in) OR expire_at IS NULL) \
                 AND message_id IN ({})",
                expand_var_with_index(2, chunk.len())
            );
            rows_affected += sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(now)
                .bind_list(chunk)
                .execute(&self.0)
                .await?
                .rows_affected();
        }
        Ok(rows_affected)
    }

    pub async fn get_current_expired_messages(&self) -> Result<Vec<ExpiredMessage>, Error> {
        self.get_expired_messages(Utc::now().timestamp()).await
    }

    pub async fn get_expired_messages(
        &self,
        current_time: i64,
    ) -> Result<Vec<ExpiredMessage>, Error> {
        let messages = sqlx::query_as::<_, ExpiredMessage>(
            "SELECT * FROM expired_messages WHERE expire_at <= ? ORDER BY expire_at ASC",
        )
        .bind(current_time)
        .fetch_all(&self.0)
        .await?;
        Ok(messages)
    }

    pub async fn get_first_expired_message(&self) -> Result<Option<ExpiredMessage>, Error> {
        let message = sqlx::query_as::<_, ExpiredMessage>(
            "SELECT * FROM expired_messages WHERE expire_at IS NOT NULL \
             ORDER BY expire_at ASC LIMIT 1",
        )
        .fetch_optional(&self.0)
        .await?;
        Ok(message)
    }

    pub async fn get_expired_message_by_id(
        &self,
        message_id: &str,
    ) -> Result<Option<ExpiredMessage>, Error> {
        let message = sqlx::query_as::<_, ExpiredMessage>(
            "SELECT * FROM expired_messages WHERE message_id = ? LIMIT 1",
        )
        .bind(message_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(message)
    }

    pub async fn get_expired_messages_by_ids(
        &self,
        message_ids: &[String],
    ) -> Result<Vec<ExpiredMessage>, Error> {
        let mut messages = Vec::new();
        for chunk in message_ids.chunks(MARK_LIMIT) {
            let sql = format!(
                "SELECT * FROM expired_messages WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            messages.extend(
                sqlx::query_as::<_, ExpiredMessage>(sqlx::AssertSqlSafe(sql))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?,
            );
        }
        Ok(messages)
    }

    pub async fn delete_by_message_id(&self, message_id: &str) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM expired_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_by_message_ids(&self, message_ids: &[String]) -> Result<u64, Error> {
        let mut rows_affected = 0;
        for chunk in message_ids.chunks(MARK_LIMIT) {
            let sql = format!(
                "DELETE FROM expired_messages WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            rows_affected += sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind_list(chunk)
                .execute(&self.0)
                .await?
                .rows_affected();
        }
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::mixin::MixinDatabase;

    #[tokio::test]
    async fn preserves_expire_in_when_read_receipt_updates_expire_at() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = database.expired_message_dao;

        dao.insert("message", 60, None).await.unwrap();
        dao.update_message_expired_at(&[("message".to_string(), 200)])
            .await
            .unwrap();

        let message = dao
            .get_expired_message_by_id("message")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(message.expire_in, 60);
        assert_eq!(message.expire_at, Some(200));
    }

    #[tokio::test]
    async fn marks_messages_read_in_seconds_and_chunks_placeholders() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = database.expired_message_dao;
        let message_ids = (0..MARK_LIMIT + 5)
            .map(|index| format!("message-{index}"))
            .collect::<Vec<_>>();

        for message_id in &message_ids {
            dao.insert(message_id, 60, None).await.unwrap();
        }
        assert_eq!(
            dao.mark_expired_message_read(&message_ids).await.unwrap(),
            message_ids.len() as u64
        );

        let first = dao.get_first_expired_message().await.unwrap().unwrap();
        let now = Utc::now().timestamp();
        assert!(first.expire_at.unwrap() >= now + 59);
        assert!(first.expire_at.unwrap() <= now + 61);

        let selected = dao
            .get_expired_messages_by_ids(&message_ids[MARK_LIMIT - 2..])
            .await
            .unwrap();
        assert_eq!(selected.len(), 7);
    }
}
