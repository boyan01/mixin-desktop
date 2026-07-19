use chrono::{NaiveDateTime, Utc};
use sqlx::{QueryBuilder, Sqlite};
use uuid::Uuid;

use sdk::blaze_message::{CREATE_MESSAGE, PIN_MESSAGE, RECALL_MESSAGE};
use sdk::message::{BlazeAckMessage, RecallMessage};
use sdk::{ACKNOWLEDGE_MESSAGE_RECEIPTS, SENDING_MESSAGE};

use crate::core::util::unique_object_id;
use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::util::{expand_var, BindListForQuery};
use crate::db::Error;

#[derive(Clone)]
pub struct JobDao(pub(crate) sqlx::Pool<Sqlite>);

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Job {
    pub job_id: String,
    pub action: String,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: NaiveDateTime,
    pub order_id: Option<i32>,
    pub priority: i32,
    pub user_id: Option<String>,
    pub blaze_message: Option<String>,
    pub conversation_id: Option<String>,
    pub resend_message_id: Option<String>,
    pub run_count: i32,
}

pub const UPDATE_STICKER: &str = "LOCAL_UPDATE_STICKER";
pub const UPDATE_ASSET: &str = "LOCAL_UPDATE_ASSET";
pub const UPDATE_TOKEN: &str = "LOCAL_UPDATE_TOKEN";
pub const SYNC_INSCRIPTION_MESSAGE: &str = "LOCAL_SYNC_INSCRIPTION_MESSAGE";
pub const MIGRATE_FTS: &str = "LOCAL_MIGRATE_FTS";

fn is_deduplicated_resource_action(action: &str) -> bool {
    matches!(
        action,
        UPDATE_STICKER | UPDATE_ASSET | UPDATE_TOKEN | SYNC_INSCRIPTION_MESSAGE
    )
}

impl Job {
    fn new() -> Self {
        Job {
            job_id: Uuid::new_v4().to_string(),
            action: Default::default(),
            created_at: Utc::now().naive_utc(),
            order_id: None,
            priority: 5,
            user_id: None,
            blaze_message: None,
            conversation_id: None,
            resend_message_id: None,
            run_count: 0,
        }
    }

    pub fn create_ack_job(
        action: &str,
        message_id: &str,
        status: &str,
        expire_at: Option<i64>,
    ) -> Job {
        let message = BlazeAckMessage {
            message_id: message_id.to_string(),
            status: status.to_string(),
            expire_at,
        };
        let job_id =
            unique_object_id(&[message.message_id.as_str(), message.status.as_str(), action])
                .to_string();
        let message = serde_json::to_string(&message).ok();
        Job {
            job_id,
            action: action.to_string(),
            blaze_message: message,
            ..Job::new()
        }
    }

    pub fn create_mention_read_ack_job(cid: &str, message_id: &str) -> Job {
        Job {
            action: CREATE_MESSAGE.to_string(),
            conversation_id: Some(cid.to_string()),
            blaze_message: serde_json::to_string(&BlazeAckMessage {
                message_id: message_id.to_string(),
                status: "MENTION_READ".to_string(),
                expire_at: None,
            })
            .ok(),
            ..Self::new()
        }
    }

    pub fn create_send_pin_job(conversation_id: &str, encoded: &str) -> Job {
        Job {
            action: PIN_MESSAGE.to_string(),
            conversation_id: Some(conversation_id.to_string()),
            blaze_message: Some(encoded.to_string()),
            ..Self::new()
        }
    }

    pub fn create_send_recall_job(conversation_id: &str, message_id: &str) -> Job {
        Job {
            conversation_id: Some(conversation_id.to_string()),
            action: RECALL_MESSAGE.to_string(),
            blaze_message: serde_json::to_string(&RecallMessage {
                message_id: message_id.to_string(),
            })
            .ok(),
            ..Self::new()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_sending_job(
        message_id: &str,
        conversation_id: &str,
        recipient_id: Option<&str>,
        recipient_session_id: Option<&str>,
        resend: bool,
        silent: bool,
        expire_in: i64,
    ) -> Job {
        let payload = serde_json::json!({
            "message_id": message_id,
            "recipient_id": recipient_id,
            "session_id": recipient_session_id,
            "resend": resend,
            "silent": silent,
            "expire_in": expire_in,
        });
        let job_id = if resend {
            unique_object_id(&[
                SENDING_MESSAGE,
                message_id,
                recipient_id.unwrap_or_default(),
                recipient_session_id.unwrap_or_default(),
            ])
            .to_string()
        } else {
            Uuid::new_v4().to_string()
        };
        Job {
            job_id,
            action: SENDING_MESSAGE.to_string(),
            conversation_id: Some(conversation_id.to_string()),
            user_id: recipient_id.map(str::to_string),
            resend_message_id: resend.then(|| message_id.to_string()),
            blaze_message: Some(payload.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_sticker_job(sticker_id: &str) -> Job {
        Job {
            job_id: unique_object_id(&[UPDATE_STICKER, sticker_id]).to_string(),
            action: UPDATE_STICKER.to_string(),
            blaze_message: Some(sticker_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_asset_job(asset_id: &str) -> Job {
        Job {
            job_id: unique_object_id(&[UPDATE_ASSET, asset_id]).to_string(),
            action: UPDATE_ASSET.to_string(),
            blaze_message: Some(asset_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_token_job(asset_id: &str) -> Job {
        Job {
            job_id: unique_object_id(&[UPDATE_TOKEN, asset_id]).to_string(),
            action: UPDATE_TOKEN.to_string(),
            blaze_message: Some(asset_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_sync_inscription_message_job(message_id: &str) -> Job {
        Job {
            job_id: unique_object_id(&[SYNC_INSCRIPTION_MESSAGE, message_id]).to_string(),
            action: SYNC_INSCRIPTION_MESSAGE.to_string(),
            blaze_message: Some(message_id.to_string()),
            ..Self::new()
        }
    }
}

impl JobDao {
    pub async fn insert_job(&self, job: &Job) -> Result<(), Error> {
        let statement = if is_deduplicated_resource_action(&job.action) {
            r#"INSERT OR IGNORE INTO jobs (job_id, action, created_at, order_id, priority, user_id,
             conversation_id, resend_message_id, run_count, blaze_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        } else {
            r#"INSERT OR REPLACE INTO jobs (job_id, action, created_at, order_id, priority, user_id,
             conversation_id, resend_message_id, run_count, blaze_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        };
        sqlx::query(statement)
            .bind(&job.job_id)
            .bind(&job.action)
            .bind(job.created_at.and_utc().timestamp_millis())
            .bind(job.order_id)
            .bind(job.priority)
            .bind(job.user_id.as_ref())
            .bind(job.conversation_id.as_ref())
            .bind(job.resend_message_id.as_ref())
            .bind(job.run_count)
            .bind(&job.blaze_message)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn insert_all(&self, jobs: &[Job]) -> Result<(), Error> {
        const COLUMN_COUNT: usize = 10;
        for (deduplicated, statement) in [
            (true, "INSERT OR IGNORE INTO jobs"),
            (false, "INSERT OR REPLACE INTO jobs"),
        ] {
            let jobs = jobs
                .iter()
                .filter(|job| is_deduplicated_resource_action(&job.action) == deduplicated)
                .collect::<Vec<_>>();
            for jobs in jobs.chunks(MARK_LIMIT / COLUMN_COUNT) {
                let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
                    "{statement} \
                     (job_id, action, created_at, order_id, priority, user_id, \
                     conversation_id, resend_message_id, run_count, blaze_message) "
                ));
                query_builder.push_values(jobs, |mut builder, job| {
                    builder
                        .push_bind(&job.job_id)
                        .push_bind(&job.action)
                        .push_bind(job.created_at.and_utc().timestamp_millis())
                        .push_bind(job.order_id)
                        .push_bind(job.priority)
                        .push_bind(job.user_id.as_ref())
                        .push_bind(job.conversation_id.as_ref())
                        .push_bind(job.resend_message_id.as_ref())
                        .push_bind(job.run_count)
                        .push_bind(&job.blaze_message);
                });

                query_builder.build().execute(&self.0).await?;
            }
        }
        Ok(())
    }

    pub async fn delete_job_by_id(&self, job_id: &str) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM jobs WHERE job_id = ?")
            .bind(job_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_jobs_by_action(&self, action: &str) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM jobs WHERE action = ?")
            .bind(action)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn reschedule_job(
        &self,
        job_id: &str,
        created_at: NaiveDateTime,
        run_count: i32,
    ) -> Result<u64, Error> {
        let result = sqlx::query("UPDATE jobs SET created_at = ?, run_count = ? WHERE job_id = ?")
            .bind(created_at.and_utc().timestamp_millis())
            .bind(run_count)
            .bind(job_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_jobs(&self, ids: &[String]) -> Result<u64, Error> {
        let chunks = ids.chunks(MARK_LIMIT);
        let mut rows_affected: u64 = 0;
        for chunk in chunks {
            let query = format!(
                "DELETE FROM jobs WHERE job_id in ({})",
                expand_var(chunk.len())
            );
            let affected = sqlx::query(sqlx::AssertSqlSafe(query))
                .bind_list(chunk)
                .execute(&self.0)
                .await?
                .rows_affected();
            rows_affected += affected;
        }
        Ok(rows_affected)
    }

    pub async fn ack_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action = ? AND blaze_message IS NOT NULL LIMIT 100",
        )
        .bind(ACKNOWLEDGE_MESSAGE_RECEIPTS)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn session_ack_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action = ? AND blaze_message IS NOT NULL \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(CREATE_MESSAGE)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn sending_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action IN (?, ?, ?) AND blaze_message IS NOT NULL \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(SENDING_MESSAGE)
        .bind(PIN_MESSAGE)
        .bind(RECALL_MESSAGE)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn update_asset_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action = ? AND blaze_message IS NOT NULL \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(UPDATE_ASSET)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn update_token_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action = ? AND blaze_message IS NOT NULL \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(UPDATE_TOKEN)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn update_sticker_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action = ? AND blaze_message IS NOT NULL \
             AND created_at <= ? ORDER BY created_at ASC LIMIT 100",
        )
        .bind(UPDATE_STICKER)
        .bind(Utc::now().timestamp_millis())
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn sync_inscription_message_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE action = ? AND blaze_message IS NOT NULL \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(SYNC_INSCRIPTION_MESSAGE)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MixinDatabase;

    async fn test_database() -> (tempfile::TempDir, MixinDatabase) {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        (directory, database)
    }

    #[tokio::test]
    async fn inserts_empty_and_large_job_batches() {
        let (_directory, database) = test_database().await;
        let jobs = (0..105)
            .map(|index| Job::create_update_asset_job(&format!("asset-{index}")))
            .collect::<Vec<_>>();

        database.job_dao.insert_all(&[]).await.unwrap();
        database.job_dao.insert_all(&jobs).await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.job_dao.0)
            .await
            .unwrap();
        assert_eq!(count, jobs.len() as i64);
    }

    #[tokio::test]
    async fn deduplicates_resource_refresh_jobs() {
        let (_directory, database) = test_database().await;
        let mut first_asset = Job::create_update_asset_job("asset");
        first_asset.created_at -= chrono::Duration::hours(1);
        first_asset.run_count = 3;
        let original_created_at = first_asset.created_at;
        let asset_job_id = first_asset.job_id.clone();
        let jobs = [
            first_asset,
            Job::create_update_asset_job("asset"),
            Job::create_update_token_job("token"),
            Job::create_update_token_job("token"),
            Job::create_update_sticker_job("sticker"),
            Job::create_update_sticker_job("sticker"),
            Job::create_sync_inscription_message_job("message"),
            Job::create_sync_inscription_message_job("message"),
        ];
        for job in &jobs {
            database.job_dao.insert_job(job).await.unwrap();
        }
        database
            .job_dao
            .insert_all(&[Job::create_update_asset_job("asset")])
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.job_dao.0)
            .await
            .unwrap();
        assert_eq!(count, 4);
        let (created_at, run_count): (i64, i32) =
            sqlx::query_as("SELECT created_at, run_count FROM jobs WHERE job_id = ?")
                .bind(asset_job_id)
                .fetch_one(&database.job_dao.0)
                .await
                .unwrap();
        assert_eq!(created_at, original_created_at.and_utc().timestamp_millis());
        assert_eq!(run_count, 3);
    }

    #[tokio::test]
    async fn sticker_jobs_only_return_when_due() {
        let (_directory, database) = test_database().await;
        let job = Job::create_update_sticker_job("sticker");
        database.job_dao.insert_job(&job).await.unwrap();
        database
            .job_dao
            .reschedule_job(
                &job.job_id,
                Utc::now().naive_utc() + chrono::Duration::minutes(5),
                2,
            )
            .await
            .unwrap();
        assert!(database
            .job_dao
            .update_sticker_jobs()
            .await
            .unwrap()
            .is_empty());

        database
            .job_dao
            .reschedule_job(
                &job.job_id,
                Utc::now().naive_utc() - chrono::Duration::seconds(1),
                2,
            )
            .await
            .unwrap();
        let due = database.job_dao.update_sticker_jobs().await.unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].run_count, 2);
    }
}
