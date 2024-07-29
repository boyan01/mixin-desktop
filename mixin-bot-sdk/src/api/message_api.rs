use std::sync::Arc;

use crate::client::ClientRef;
use crate::{ApiError, BlazeAckMessage};

pub struct MessageApi {
    pub(crate) client: Arc<ClientRef>,
}

impl MessageApi {
    pub async fn acknowledgements(&self, acks: &[BlazeAckMessage]) -> Result<(), ApiError> {
        let request = self
            .client
            .client
            .post(format!("{}/acknowledgements", self.client.base_url))
            .body(serde_json::to_string(&acks)?)
            .build()?;
        let _ = self.client.raw_request(request).await?;
        Ok(())
    }
}
