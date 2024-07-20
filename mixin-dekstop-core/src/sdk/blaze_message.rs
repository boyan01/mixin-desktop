use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{db, sdk};

pub(crate) const ACKNOWLEDGE_MESSAGE_RECEIPT: &str = "ACKNOWLEDGE_MESSAGE_RECEIPT";
pub(crate) const ACKNOWLEDGE_MESSAGE_RECEIPTS: &str = "ACKNOWLEDGE_MESSAGE_RECEIPTS";
pub(crate) const DEVICE_TRANSFER: &str = "DEVICE_TRANSFER";
pub(crate) const SENDING_MESSAGE: &str = "SENDING_MESSAGE";
pub(crate) const RECALL_MESSAGE: &str = "RECALL_MESSAGE";
pub(crate) const PIN_MESSAGE: &str = "PIN_MESSAGE";
pub(crate) const RESEND_MESSAGES: &str = "RESEND_MESSAGES";
pub(crate) const CREATE_MESSAGE: &str = "CREATE_MESSAGE";
pub(crate) const CREATE_CALL: &str = "CREATE_CALL";
pub(crate) const CREATE_KRAKEN: &str = "CREATE_KRAKEN";
pub(crate) const LIST_PENDING_MESSAGE: &str = "LIST_PENDING_MESSAGES";
pub(crate) const RESEND_KEY: &str = "RESEND_KEY";
pub(crate) const NO_KEY: &str = "NO_KEY";
pub(crate) const ERROR_ACTION: &str = "ERROR";
pub(crate) const CONSUME_SESSION_SIGNAL_KEYS: &str = "CONSUME_SESSION_SIGNAL_KEYS";
pub(crate) const CREATE_SIGNAL_KEY_MESSAGES: &str = "CREATE_SIGNAL_KEY_MESSAGES";
pub(crate) const COUNT_SIGNAL_KEYS: &str = "COUNT_SIGNAL_KEYS";
pub(crate) const SYNC_SIGNAL_KEYS: &str = "SYNC_SIGNAL_KEYS";

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub(crate) struct BlazeMessage {
    pub id: String,
    pub action: String,
    pub params: Option<Value>,
    pub data: Option<Value>,
    pub error: Option<sdk::Error>,
}

impl BlazeMessage {
    pub(crate) fn new_list_pending_blaze(offset: Option<String>) -> Self {
        BlazeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            action: LIST_PENDING_MESSAGE.to_string(),
            params: offset.map(|v| json!({"offset": v})),
            data: None,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum MessageStatus {
    Failed,
    Unknown,
    Sending,
    Sent,
    Delivered,
    Read,
}

impl Into<String> for MessageStatus {
    fn into(self) -> String {
        match self {
            MessageStatus::Failed => "FAILED",
            MessageStatus::Unknown => "UNKNOWN",
            MessageStatus::Sending => "SENDING",
            MessageStatus::Sent => "SENT",
            MessageStatus::Delivered => "DELIVERED",
            MessageStatus::Read => "READ",
        }.to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BlazeMessageData {
    pub conversation_id: String,
    pub user_id: String,
    pub message_id: String,
    pub category: Option<String>,
    pub data: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
    pub representative_id: Option<String>,
    pub quote_message_id: Option<String>,
    pub session_id: String,
    pub silent: Option<bool>,
    pub expire_in: Option<i32>,
}
