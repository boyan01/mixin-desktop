use std::sync::Arc;

use anyhow::Result;
use log::{error, info};
use sdk::{BlazeAckMessage, Client};

use crate::db::MixinDatabase;

use super::{JobCategory, JobTrigger};

pub(super) struct AckJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) client: Arc<Client>,
}

impl JobTrigger for AckJobRunner {
    async fn trigger(&self) -> Result<bool> {
        loop {
            let jobs = self.database.job_dao.ack_jobs().await?;
            info!("trigger ack job runner: {:?}", jobs.len());
            if jobs.is_empty() {
                return Ok(false);
            }
            let mut job_ids = vec![];
            let mut acks = vec![];
            for job in jobs {
                if let Some(m) = job
                    .blaze_message
                    .and_then(|m| serde_json::from_str::<BlazeAckMessage>(&m).ok())
                {
                    acks.push(m);
                } else {
                    error!("AckJobRunner: failed to parse message: {:?}", job.job_id);
                }
                job_ids.push(job.job_id);
            }

            let result = self.client.message_api.acknowledgements(&acks).await;
            if let Err(err) = result {
                error!("failed to ack messages: {:?}", err);
                return Ok(true);
            } else {
                info!("ack messages success");
                self.database.job_dao.delete_jobs(&job_ids).await?;
            }
        }
    }

    fn category(&self) -> JobCategory {
        JobCategory::Ack
    }
}
