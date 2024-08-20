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
    pub created_at: NaiveDateTime,
    pub order_id: Option<i32>,
    pub priority: i32,
    pub user_id: Option<String>,
    pub blaze_message: Option<String>,
    pub conversation_id: Option<String>,
    pub resend_message_id: Option<String>,
    pub run_count: i32,
}

const UPDATE_STICKER: &str = "LOCAL_UPDATE_STICKER";
const UPDATE_ASSET: &str = "LOCAL_UPDATE_ASSET";
const UPDATE_TOKEN: &str = "LOCAL_UPDATE_TOKEN";
const SYNC_INSCRIPTION_MESSAGE: &str = "LOCAL_SYNC_INSCRIPTION_MESSAGE";

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

    pub fn create_update_sticker_job(sticker_id: &str) -> Job {
        Job {
            action: UPDATE_STICKER.to_string(),
            blaze_message: Some(sticker_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_asset_job(asset_id: &str) -> Job {
        Job {
            action: UPDATE_ASSET.to_string(),
            blaze_message: Some(asset_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_token_job(asset_id: &str) -> Job {
        Job {
            action: UPDATE_TOKEN.to_string(),
            blaze_message: Some(asset_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_sync_inscription_message_job(message_id: &str) -> Job {
        Job {
            action: SYNC_INSCRIPTION_MESSAGE.to_string(),
            blaze_message: Some(message_id.to_string()),
            ..Self::new()
        }
    }
}

impl JobDao {
    pub async fn insert_job(&self, job: &Job) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO jobs (job_id, action, created_at, order_id, priority, user_id,
             conversation_id, resend_message_id, run_count, blaze_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&job.job_id)
        .bind(&job.action)
        .bind(job.created_at)
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
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO jobs\
         (job_id, action, created_at, order_id, priority, user_id, \
         conversation_id, resend_message_id, run_count, blaze_message) ",
        );
        query_builder.push_values(jobs, |mut builder, job| {
            builder
                .push_bind(&job.job_id)
                .push_bind(&job.action)
                .push_bind(job.created_at)
                .push_bind(job.order_id)
                .push_bind(job.priority)
                .push_bind(job.user_id.as_ref())
                .push_bind(job.conversation_id.as_ref())
                .push_bind(job.resend_message_id.as_ref())
                .push_bind(job.run_count)
                .push_bind(&job.blaze_message);
        });

        query_builder.build().execute(&self.0).await?;
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

    pub async fn delete_jobs(&self, ids: &[String]) -> Result<u64, Error> {
        let chunks = ids.chunks(MARK_LIMIT);
        let mut rows_affected: u64 = 0;
        for chunk in chunks {
            let affected = sqlx::query(&format!(
                "DELETE FROM jobs WHERE job_id in ({})",
                expand_var(chunk.len())
            ))
            .bind_list(chunk)
            .execute(&self.0)
            .await?
            .rows_affected();
            rows_affected += affected;
        }
        Ok(rows_affected)
    }

    pub async fn ack_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(&format!(
            "SELECT * FROM jobs WHERE action = '{}' AND blaze_message IS NOT NULL LIMIT 100",
            ACKNOWLEDGE_MESSAGE_RECEIPTS
        ))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn session_ack_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(&format!(
            "SELECT * FROM jobs WHERE action = '{}' AND blaze_message IS NOT NULL ORDER BY created_at ASC  LIMIT 100",
            CREATE_MESSAGE
        ))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn sending_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(&format!(
            "SELECT * FROM jobs WHERE action IN ('{}', '{}', '{}') AND blaze_message IS NOT NULL ORDER BY created_at ASC  LIMIT 100",
            SENDING_MESSAGE, PIN_MESSAGE, RECALL_MESSAGE,
        ))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn update_asset_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(&format!(
            "SELECT * FROM jobs WHERE action = '{}' AND blaze_message IS NOT NULL ORDER BY created_at ASC  LIMIT 100",
            UPDATE_ASSET
        ))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn update_token_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(&format!(
            "SELECT * FROM jobs WHERE action = '{}' AND blaze_message IS NOT NULL ORDER BY created_at ASC  LIMIT 100",
            UPDATE_TOKEN
        ))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn update_sticker_jobs(&self) -> Result<Vec<Job>, Error> {
        let result = sqlx::query_as::<_, Job>(&format!(
            "SELECT * FROM jobs WHERE action = '{}' AND blaze_message IS NOT NULL ORDER BY created_at ASC  LIMIT 100",
            UPDATE_STICKER,
        ))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }
}
