use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachmentExtra {
    pub attachment_id: String,
    pub message_id: String,
    pub shareable: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}
