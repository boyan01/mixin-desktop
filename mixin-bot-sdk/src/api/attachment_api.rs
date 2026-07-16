use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::client::ClientRef;
use crate::ApiError;

pub struct AttachmentApi {
    client: Arc<ClientRef>,
}

impl AttachmentApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        Self { client }
    }

    pub async fn get_attachment(&self, attachment_id: &str) -> Result<Attachment, ApiError> {
        self.client
            .get(&format!("attachments/{attachment_id}"))
            .await
    }

    pub async fn create_attachment(&self) -> Result<Attachment, ApiError> {
        self.client
            .post("attachments", &serde_json::json!({}))
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    pub attachment_id: String,
    pub created_at: DateTime<Utc>,
    pub upload_url: Option<String>,
    pub view_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attachment_response() {
        let attachment: Attachment = serde_json::from_str(
            r#"{
                "attachment_id":"attachment-id",
                "created_at":"2024-08-20T08:00:00Z",
                "upload_url":null,
                "view_url":"https://example.com/file"
            }"#,
        )
        .unwrap();

        assert_eq!(attachment.attachment_id, "attachment-id");
        assert_eq!(
            attachment.view_url.as_deref(),
            Some("https://example.com/file")
        );
    }
}
