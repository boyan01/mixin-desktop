use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BlazeAckMessage {
    pub message_id: String,
    pub status: String,
    pub expire_at: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct RecallMessage {
    pub message_id: String,
}
