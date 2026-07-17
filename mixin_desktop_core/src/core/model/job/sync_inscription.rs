use std::sync::Arc;

use anyhow::{Context, Result};
use log::warn;
use sdk::Client;

use crate::db::MixinDatabase;

use super::{JobCategory, JobTrigger};

pub(super) struct SyncInscriptionJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) client: Arc<Client>,
}

impl JobTrigger for SyncInscriptionJobRunner {
    async fn trigger(&self) -> Result<bool> {
        self.run_sync_inscription_jobs().await?;
        Ok(false)
    }

    fn category(&self) -> JobCategory {
        JobCategory::SyncInscription
    }
}

impl SyncInscriptionJobRunner {
    async fn run_sync_inscription_jobs(&self) -> Result<()> {
        for job in self
            .database
            .job_dao
            .sync_inscription_message_jobs()
            .await?
        {
            let Some(message_id) = job.blaze_message.as_deref() else {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                continue;
            };
            let result = self.sync_inscription_message(message_id).await;
            match result {
                Ok(()) => {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                Err(err) => warn!("failed to sync inscription message {message_id}: {err}"),
            }
        }
        Ok(())
    }

    async fn sync_inscription_message(&self, message_id: &str) -> Result<()> {
        let Some(message) = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
        else {
            warn!("inscription message not found: {message_id}");
            return Ok(());
        };
        let Some(hash) = message.content.as_deref() else {
            warn!("inscription message has no hash: {message_id}");
            return Ok(());
        };
        if let Err(error) = hex::decode(hash).context("invalid inscription hash") {
            warn!("failed to sync inscription message {message_id}: {error}");
            return Ok(());
        }

        let item = match self.database.inscription_dao.find_item(hash).await? {
            Some(item) => item,
            None => {
                let item = match self.client.token_api.get_inscription_item(hash).await {
                    Ok(item) => item,
                    Err(error) => {
                        warn!("failed to get inscription {hash}: {error}");
                        return Ok(());
                    }
                };
                self.database.inscription_dao.insert_item(&item).await?;
                item
            }
        };
        let collection = match self
            .database
            .inscription_dao
            .find_collection(&item.collection_hash)
            .await?
        {
            Some(collection) => collection,
            None => {
                let collection = match self
                    .client
                    .token_api
                    .get_inscription_collection(&item.collection_hash)
                    .await
                {
                    Ok(collection) => collection,
                    Err(error) => {
                        warn!(
                            "failed to get inscription collection {}: {error}",
                            item.collection_hash
                        );
                        return Ok(());
                    }
                };
                self.database
                    .inscription_dao
                    .insert_collection(&collection)
                    .await?;
                collection
            }
        };
        let content = serde_json::json!({
            "collection_hash": collection.collection_hash,
            "inscription_hash": item.inscription_hash,
            "sequence": item.sequence,
            "content_type": item.content_type,
            "content_url": item.content_url,
            "name": collection.name,
            "icon_url": collection.icon_url,
        })
        .to_string();
        self.database
            .message_dao
            .update_message_content_and_status(message_id, &content, message.status)
            .await?;
        Ok(())
    }
}
