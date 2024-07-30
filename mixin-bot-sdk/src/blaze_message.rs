use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::Error;

pub const ACKNOWLEDGE_MESSAGE_RECEIPT: &str = "ACKNOWLEDGE_MESSAGE_RECEIPT";
pub const ACKNOWLEDGE_MESSAGE_RECEIPTS: &str = "ACKNOWLEDGE_MESSAGE_RECEIPTS";
pub const DEVICE_TRANSFER: &str = "DEVICE_TRANSFER";
pub const SENDING_MESSAGE: &str = "SENDING_MESSAGE";
pub const RECALL_MESSAGE: &str = "RECALL_MESSAGE";
pub const PIN_MESSAGE: &str = "PIN_MESSAGE";
pub const RESEND_MESSAGES: &str = "RESEND_MESSAGES";
pub const CREATE_MESSAGE: &str = "CREATE_MESSAGE";
pub const CREATE_CALL: &str = "CREATE_CALL";
pub const CREATE_KRAKEN: &str = "CREATE_KRAKEN";
pub const LIST_PENDING_MESSAGE: &str = "LIST_PENDING_MESSAGES";
pub const RESEND_KEY: &str = "RESEND_KEY";
pub const NO_KEY: &str = "NO_KEY";
pub const ERROR_ACTION: &str = "ERROR";
pub const CONSUME_SESSION_SIGNAL_KEYS: &str = "CONSUME_SESSION_SIGNAL_KEYS";
pub const CREATE_SIGNAL_KEY_MESSAGES: &str = "CREATE_SIGNAL_KEY_MESSAGES";
pub const COUNT_SIGNAL_KEYS: &str = "COUNT_SIGNAL_KEYS";
pub const SYNC_SIGNAL_KEYS: &str = "SYNC_SIGNAL_KEYS";

#[derive(Serialize, Deserialize, Debug)]
pub struct BlazeMessage {
    pub id: String,
    pub action: String,
    pub params: Option<Value>,
    pub data: Option<Value>,
    pub error: Option<Error>,
}

impl BlazeMessage {
    pub fn new_list_pending_blaze(offset: Option<String>) -> Self {
        BlazeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            action: LIST_PENDING_MESSAGE.to_string(),
            params: offset.map(|v| json!({"offset": v})),
            data: None,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Debug, Eq, Default, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
#[derive(sqlx::Type)]
#[sqlx(rename_all = "UPPERCASE")]
pub enum MessageStatus {
    Failed,
    #[default]
    Unknown,
    Sending,
    Sent,
    Delivered,
    Read,
}

impl From<MessageStatus> for &str {
    fn from(value: MessageStatus) -> Self {
        match value {
            MessageStatus::Failed => "FAILED",
            MessageStatus::Unknown => "UNKNOWN",
            MessageStatus::Sending => "SENDING",
            MessageStatus::Sent => "SENT",
            MessageStatus::Delivered => "DELIVERED",
            MessageStatus::Read => "READ",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlazeMessageData {
    pub conversation_id: String,
    pub user_id: String,
    pub message_id: String,
    #[serde(default)]
    pub category: String,
    pub data: String,
    pub status: MessageStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
    pub representative_id: Option<String>,
    pub quote_message_id: Option<String>,
    pub session_id: String,
    pub silent: Option<bool>,
    pub expire_in: Option<i32>,
}
