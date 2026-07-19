use std::sync::Arc;

use anyhow::Result;
use log::info;

use crate::db::mixin::job::{Job, MIGRATE_FTS};
use crate::db::MixinDatabase;

use super::{JobCategory, JobTrigger};

pub(super) struct MigrateFtsJobRunner {
    pub(super) database: Arc<MixinDatabase>,
}

impl JobTrigger for MigrateFtsJobRunner {
    async fn trigger(&self) -> Result<bool> {
        loop {
            let job = sqlx::query_as::<_, Job>(
                "SELECT * FROM jobs WHERE action = ? ORDER BY created_at LIMIT 1",
            )
            .bind(MIGRATE_FTS)
            .fetch_optional(&self.database.job_dao.0)
            .await?;
            let Some(job) = job else {
                return Ok(false);
            };
            let anchor = job
                .blaze_message
                .as_deref()
                .and_then(|value| value.parse::<i64>().ok());
            let Some(next_anchor) = self.database.message_fts_dao.migrate_batch(anchor).await?
            else {
                sqlx::query("DELETE FROM jobs WHERE action = ?")
                    .bind(MIGRATE_FTS)
                    .execute(&self.database.job_dao.0)
                    .await?;
                info!("message FTS migration completed");
                return Ok(false);
            };
            sqlx::query("UPDATE jobs SET blaze_message = ? WHERE job_id = ?")
                .bind(next_anchor.to_string())
                .bind(&job.job_id)
                .execute(&self.database.job_dao.0)
                .await?;
            tokio::task::yield_now().await;
        }
    }

    fn category(&self) -> JobCategory {
        JobCategory::MigrateFts
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sdk::blaze_message::MessageStatus;

    use super::*;
    use crate::db::mixin::message::Message;

    #[tokio::test]
    async fn migrates_messages_in_batches_and_removes_job() {
        let directory = tempfile::tempdir().unwrap();
        let database = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        sqlx::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('conversation', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "message".into(),
                conversation_id: "conversation".into(),
                user_id: "user".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("hello migration".into()),
                status: MessageStatus::Sent,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO jobs (job_id, action, created_at, priority, run_count) \
             VALUES ('migration', ?, 0, 5, 0)",
        )
        .bind(MIGRATE_FTS)
        .execute(&database.job_dao.0)
        .await
        .unwrap();

        MigrateFtsJobRunner {
            database: database.clone(),
        }
        .trigger()
        .await
        .unwrap();

        let indexed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM fts.messages_metas WHERE message_id = 'message'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE action = ?")
            .bind(MIGRATE_FTS)
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!((indexed, jobs), (1, 0));
    }
}
