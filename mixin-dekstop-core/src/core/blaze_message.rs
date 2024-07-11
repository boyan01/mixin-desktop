use diesel::serialize::IsNull::No;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::sdk;

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub(crate) struct BlazeMessage {
    id: String,
    action: String,
    params: Option<Value>,
    data: Option<Value>,
    error: Option<sdk::Error>,
}

impl BlazeMessage {
    pub(crate) fn new_list_pending_blaze(offset: Option<String>) -> Self {
        BlazeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            action: "LIST_PENDING_MESSAGES".to_string(),
            params: offset.map(|v| json!({"offset": v})),
            data: None,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum MessageStatus {
    Failed,
    Unknown,
    Sending,
    Sent,
    Delivered,
    Read,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BlazeMessageData {
    conversation_id: String,
    user_id: String,
    message_id: String,
    category: Option<String>,
    data: String,
    status: MessageStatus,
    created_at: String,
    updated_at: String,
    source: String,
    representative_id: Option<String>,
    quote_message_id: Option<String>,
    session_id: String,
    silent: Option<bool>,
    expire_in: Option<i32>,
}
