use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use tokio::sync::Mutex;

use crate::db::Error;

pub const MESSAGE_STATUS_OFFSET: &str = "messages_status_offset";

#[derive(Clone)]
pub struct OffsetDao {
    pool: sqlx::Pool<sqlx::Sqlite>,
    write_lock: Arc<Mutex<()>>,
}

impl OffsetDao {
    pub fn new(pool: sqlx::Pool<sqlx::Sqlite>) -> Self {
        Self {
            pool,
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn message_status_offset(&self) -> Result<Option<DateTime<Utc>>, Error> {
        sqlx::query_scalar::<_, String>("SELECT timestamp FROM offsets WHERE \"key\" = ?")
            .bind(MESSAGE_STATUS_OFFSET)
            .fetch_optional(&self.pool)
            .await?
            .map(|value| {
                if let Ok(millis) = value.parse::<i64>() {
                    return DateTime::from_timestamp_millis(millis).ok_or_else(|| {
                        anyhow::anyhow!("invalid message status offset: {value}").into()
                    });
                }

                DateTime::parse_from_rfc3339(&value)
                    .map(|value| value.with_timezone(&Utc))
                    .map_err(|error| {
                        anyhow::anyhow!("invalid message status offset {value}: {error}").into()
                    })
            })
            .transpose()
    }

    pub async fn save_message_status_offset(&self, updated_at: DateTime<Utc>) -> Result<(), Error> {
        let _guard = self.write_lock.lock().await;
        if self
            .message_status_offset()
            .await?
            .is_some_and(|current| current > updated_at)
        {
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO offsets (\"key\", timestamp) VALUES (?, ?) \
             ON CONFLICT(\"key\") DO UPDATE SET timestamp = excluded.timestamp",
        )
        .bind(MESSAGE_STATUS_OFFSET)
        .bind(updated_at.to_rfc3339_opts(SecondsFormat::AutoSi, true))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::db::mixin::MixinDatabase;

    #[tokio::test]
    async fn message_status_offset_only_moves_forward() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let first = Utc.timestamp_nanos(1_700_000_000_000_000_000);
        let second = Utc.timestamp_nanos(1_700_000_001_000_000_000);

        database
            .offset_dao
            .save_message_status_offset(second)
            .await
            .unwrap();
        database
            .offset_dao
            .save_message_status_offset(first)
            .await
            .unwrap();

        assert_eq!(
            database.offset_dao.message_status_offset().await.unwrap(),
            Some(second)
        );
    }
}
