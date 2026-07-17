use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use log::warn;
use sdk::Client;

use crate::db::MixinDatabase;

use super::{JobCategory, JobTrigger};

pub(super) struct UpdateStickerJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) client: Arc<Client>,
}

impl JobTrigger for UpdateStickerJobRunner {
    async fn trigger(&self) -> Result<bool> {
        self.run_update_sticker_jobs().await?;
        Ok(false)
    }

    fn category(&self) -> JobCategory {
        JobCategory::UpdateSticker
    }
}

impl UpdateStickerJobRunner {
    async fn run_update_sticker_jobs(&self) -> Result<()> {
        for job in self.database.job_dao.update_sticker_jobs().await? {
            let Some(sticker_id) = job.blaze_message.as_deref() else {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                continue;
            };
            let result: Result<()> =
                match self.client.account_api.get_sticker_by_id(sticker_id).await {
                    Ok(sticker) => self
                        .database
                        .sticker_dao
                        .insert(&sticker)
                        .await
                        .map_err(Into::into),
                    Err(error) => Err(error.into()),
                };
            match result {
                Ok(()) => {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                Err(error)
                    if error.downcast_ref::<sdk::ApiError>().is_some_and(
                        |error| matches!(error, sdk::ApiError::Server(error) if error.status == 404 || error.code == 404),
                    ) =>
                {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                Err(error) => {
                    self.database
                        .job_dao
                        .reschedule_job(
                            &job.job_id,
                            Utc::now().naive_utc() + sticker_backoff(job.run_count),
                            job.run_count.saturating_add(1),
                        )
                        .await?;
                    warn!("failed to update sticker {sticker_id}: {error}");
                }
            }
        }
        Ok(())
    }
}

fn sticker_backoff(run_count: i32) -> chrono::Duration {
    match run_count {
        i32::MIN..=0 => chrono::Duration::minutes(1),
        1 => chrono::Duration::minutes(5),
        2 => chrono::Duration::minutes(15),
        3 => chrono::Duration::hours(1),
        _ => chrono::Duration::hours(6),
    }
}

#[cfg(test)]
mod tests {
    use super::sticker_backoff;

    #[test]
    fn uses_flutter_sticker_retry_backoff() {
        assert_eq!(sticker_backoff(0), chrono::Duration::minutes(1));
        assert_eq!(sticker_backoff(1), chrono::Duration::minutes(5));
        assert_eq!(sticker_backoff(2), chrono::Duration::minutes(15));
        assert_eq!(sticker_backoff(3), chrono::Duration::hours(1));
        assert_eq!(sticker_backoff(4), chrono::Duration::hours(6));
        assert_eq!(sticker_backoff(99), chrono::Duration::hours(6));
    }
}
