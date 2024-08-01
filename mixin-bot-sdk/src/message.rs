use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlazeAckMessage {
    pub message_id: String,
    pub status: String,
    pub expire_at: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct RecallMessage {
    pub message_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttachmentMessage {
    pub key: Option<Vec<u8>>,
    pub digest: Option<Vec<u8>>,
    pub attachment_id: String,
    pub mime_type: String,
    pub size: i64,
    pub name: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub thumbnail: Option<String>,
    pub duration: Option<i64>,
    pub waveform: Option<Vec<u8>>,
    pub caption: Option<String>,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    pub shareable: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StickerMessage {
    pub sticker_id: String,
    pub album_id: Option<String>,
    #[serde(default)]
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContactMessage {
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveMessage {
    pub width: i32,
    pub height: i32,
    pub thumb_url: String,
    pub url: String,
    pub shareable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocationMessage {
    pub latitude: f64,
    pub longitude: f64,
    pub name: Option<String>,
    pub address: Option<String>,
    pub venue_type: Option<String>,
}


