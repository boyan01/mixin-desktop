use std::sync::Arc;

use anyhow::Result;
use log::warn;
use sdk::Client;

use crate::db::MixinDatabase;

use super::{JobCategory, JobTrigger};

pub(super) struct UpdateTokenJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) client: Arc<Client>,
}

impl JobTrigger for UpdateTokenJobRunner {
    async fn trigger(&self) -> Result<bool> {
        self.run_update_token_jobs().await
    }

    fn category(&self) -> JobCategory {
        JobCategory::UpdateToken
    }
}

impl UpdateTokenJobRunner {
    async fn run_update_token_jobs(&self) -> Result<bool> {
        loop {
            let jobs = self.database.job_dao.update_token_jobs().await?;
            if jobs.is_empty() {
                return Ok(false);
            }
            let mut retry = false;
            for job in jobs {
                let Some(asset_id) = job.blaze_message.as_deref() else {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                    continue;
                };
                let result: Result<()> = async {
                    let token = self.client.token_api.get_asset_by_id(asset_id).await?;
                    let chain = self.client.asset_api.get_chain(&token.chain_id).await?;
                    self.database.asset_dao.insert_token(&token).await?;
                    self.database.asset_dao.insert_chain(&chain).await?;
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                    Ok(())
                }
                .await;
                if let Err(err) = result {
                    warn!("failed to update token {asset_id}: {err}");
                    retry = true;
                }
            }
            if retry {
                return Ok(true);
            }
        }
    }
}
