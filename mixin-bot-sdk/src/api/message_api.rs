use std::sync::Arc;

use crate::client::ClientRef;
use crate::{ApiError, BlazeAckMessage, BlazeMessageData};

pub struct MessageApi {
    pub(crate) client: Arc<ClientRef>,
}

impl MessageApi {
    pub async fn message_status_offset(
        &self,
        offset: i64,
    ) -> Result<Vec<BlazeMessageData>, ApiError> {
        self.client.get(&format!("messages/status/{offset}")).await
    }

    pub async fn acknowledgements(&self, acks: &[BlazeAckMessage]) -> Result<(), ApiError> {
        let request = self
            .client
            .client
            .post(format!("{}/acknowledgements", self.client.base_url))
            .body(serde_json::to_string(&acks)?)
            .build()?;
        let _: serde_json::Value = self.client.request(request).await?;
        Ok(())
    }
}
