use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, message_category};

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

pub const SYSTEM_USER: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlazeMessage {
    pub id: String,
    pub action: String,
    pub params: Option<BlazeMessageParam>,
    pub data: Option<Value>,
    pub error: Option<Error>,
}

impl BlazeMessage {
    pub fn new_list_pending_blaze(offset: Option<String>) -> Self {
        BlazeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            action: LIST_PENDING_MESSAGE.to_string(),
            params: Some(BlazeMessageParam {
                offset,
                ..Default::default()
            }),
            data: None,
            error: None,
        }
    }

    pub fn new_param_blaze(params: BlazeMessageParam) -> Self {
        BlazeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            action: CREATE_MESSAGE.to_string(),
            params: Some(params),
            data: None,
            error: None,
        }
    }

    pub fn new_signal_key_message(
        conversation_id: String,
        messages: Vec<BlazeSignalKeyMessage>,
        conversation_checksum: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action: CREATE_SIGNAL_KEY_MESSAGES.to_string(),
            params: Some(BlazeMessageParam {
                conversation_id: Some(conversation_id),
                conversation_checksum: Some(conversation_checksum),
                messages: Some(messages),
                ..Default::default()
            }),
            data: None,
            error: None,
        }
    }

    pub fn new_count_signal_keys() -> Self {
        BlazeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            action: COUNT_SIGNAL_KEYS.to_string(),
            params: None,
            data: None,
            error: None,
        }
    }

    pub fn new_sync_signal_keys(request: SignalKeyRequest) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action: SYNC_SIGNAL_KEYS.to_string(),
            params: Some(BlazeMessageParam {
                keys: Some(request),
                ..Default::default()
            }),
            data: None,
            error: None,
        }
    }

    pub fn new_consume_session_signal_keys(recipients: Vec<BlazeMessageParamSession>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action: CONSUME_SESSION_SIGNAL_KEYS.to_string(),
            params: Some(BlazeMessageParam {
                recipients: Some(recipients),
                ..BlazeMessageParam::default()
            }),
            data: None,
            error: None,
        }
    }

    pub fn new_plain_json(
        conversation_id: &str,
        conversation_checksum: String,
        user_id: &str,
        encoded: String,
        session_id: impl Into<Option<String>>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action: CREATE_MESSAGE.to_string(),
            params: Some(BlazeMessageParam {
                conversation_id: Some(conversation_id.to_string()),
                conversation_checksum: Some(conversation_checksum),
                recipient_id: Some(user_id.to_string()),
                message_id: Some(uuid::Uuid::new_v4().to_string()),
                category: Some(message_category::PLAIN_JSON.to_string()),
                data: Some(encoded),
                status: Some(MessageStatus::Sending.into()),
                session_id: session_id.into(),
                ..Default::default()
            }),
            data: None,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BlazeMessageParam {
    pub conversation_id: Option<String>,
    pub recipient_id: Option<String>,
    pub message_id: Option<String>,
    pub category: Option<String>,
    pub data: Option<String>,
    pub status: Option<String>,
    pub recipients: Option<Vec<BlazeMessageParamSession>>,
    pub keys: Option<SignalKeyRequest>,
    pub messages: Option<Vec<BlazeSignalKeyMessage>>,
    pub quote_message_id: Option<String>,
    pub session_id: Option<String>,
    pub representative_id: Option<String>,
    pub conversation_checksum: Option<String>,
    pub mentions: Option<Vec<String>>,
    pub jsep: Option<String>,
    pub candidate: Option<String>,
    pub track_id: Option<String>,
    pub recipient_ids: Option<Vec<String>>,
    pub offset: Option<String>,
    pub silent: Option<bool>,
    pub expire_in: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlazeSignalKeyMessage {
    pub message_id: String,
    pub recipient_id: String,
    pub data: String,
    pub session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlazeMessageParamSession {
    pub user_id: String,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalKeyRequest {
    pub identity_key: String,
    pub signed_pre_key: SignedPreKey,
    pub one_time_pre_keys: Vec<OneTimePreKey>,
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

impl From<MessageStatus> for String {
    fn from(value: MessageStatus) -> Self {
        match value {
            MessageStatus::Failed => "FAILED".to_string(),
            MessageStatus::Unknown => "UNKNOWN".to_string(),
            MessageStatus::Sending => "SENDING".to_string(),
            MessageStatus::Sent => "SENT".to_string(),
            MessageStatus::Delivered => "DELIVERED".to_string(),
            MessageStatus::Read => "READ".to_string(),
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
    pub expire_in: Option<i64>,
}

impl BlazeMessageData {
    pub fn sender_id(&self) -> &String {
        if let Some(representative_id) = self.representative_id.as_ref() {
            if !representative_id.is_empty() {
                return representative_id;
            }
        }
        &self.user_id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlainJsonMessage {
    pub action: String,
    pub messages: Option<Vec<String>>,
    pub user_id: Option<String>,
    pub message_id: Option<String>,
    pub session_id: Option<String>,
    pub ack_messages: Option<Vec<crate::BlazeAckMessage>>,
    pub content: Option<String>,
}

pub mod participant_role {
    pub const OWNER: &str = "OWNER";
    pub const ADMIN: &str = "ADMIN";
}

pub mod message_action {
    pub const JOIN: &str = "JOIN";
    pub const EXIT: &str = "EXIT";
    pub const ADD: &str = "ADD";
    pub const REMOVE: &str = "REMOVE";
    pub const CREATE: &str = "CREATE";
    pub const UPDATE: &str = "UPDATE";
    pub const ROLE: &str = "ROLE";
    pub const EXPIRE: &str = "EXPIRE";
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemConversationMessage {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub participant_id: String,
    pub user_id: Option<String>,
    pub role: Option<String>,
    pub expire_in: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemUserMessage {
    pub action: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemCircleAction {
    Create,
    Delete,
    Update,
    Add,
    Remove,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemCircleMessage {
    pub action: SystemCircleAction,
    pub circle_id: String,
    pub conversation_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnapshotMessage {
    #[serde(rename = "type")]
    pub type_field: String,
    pub snapshot_id: String,
    pub asset_id: String,
    pub amount: String,
    pub created_at: DateTime<Utc>,
    pub opponent_id: Option<String>,
    pub trace_id: Option<String>,
    pub transaction_hash: Option<String>,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub memo: Option<String>,
    pub confirmations: Option<i32>,
    pub snapshot_hash: Option<String>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignalKey {
    pub identity_key: String,
    pub signed_pre_key: SignedPreKey,
    pub ont_time_pre_key: OneTimePreKey,
    pub registration_id: u32,
    pub user_id: String,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignedPreKey {
    pub key_id: u32,
    pub pub_key: Option<String>,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OneTimePreKey {
    pub key_id: u32,
    pub pub_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignalKeyCount {
    pub one_time_pre_keys_count: u32,
}
