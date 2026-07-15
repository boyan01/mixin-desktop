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
        let body = self.client.raw_request(request).await?;
        if !is_empty_success_body(&body) {
            let _: serde_json::Value = self.client.parse_response(&body)?;
        }
        Ok(())
    }
}

fn is_empty_success_body(body: &[u8]) -> bool {
    body.is_empty()
        || serde_json::from_slice::<serde_json::Value>(body)
            .is_ok_and(|value| value.as_object().is_some_and(serde_json::Map::is_empty))
}

#[cfg(test)]
mod tests {
    use super::is_empty_success_body;

    #[test]
    fn accepts_empty_acknowledgement_response() {
        assert!(is_empty_success_body(b""));
        assert!(is_empty_success_body(b"{}"));
        assert!(!is_empty_success_body(br#"{"data":{}}"#));
    }
}
