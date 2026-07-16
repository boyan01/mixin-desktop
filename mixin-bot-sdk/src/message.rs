use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod ack_message_status {
    pub const READ: &str = "READ";
    pub const MENTION_READ: &str = "MENTION_READ";
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlazeAckMessage {
    pub message_id: String,
    pub status: String,
    pub expire_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct RecallMessage {
    pub message_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttachmentMessage {
    #[serde(default, with = "optional_base64_bytes")]
    pub key: Option<Vec<u8>>,
    #[serde(default, with = "optional_base64_bytes")]
    pub digest: Option<Vec<u8>>,
    pub attachment_id: String,
    pub mime_type: String,
    pub size: i64,
    pub name: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub thumbnail: Option<String>,
    pub duration: Option<i64>,
    #[serde(default, with = "optional_base64_bytes")]
    pub waveform: Option<Vec<u8>>,
    pub caption: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub shareable: Option<bool>,
}

mod optional_base64_bytes {
    use base64ct::{Base64, Encoding};
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Bytes {
        Base64(String),
        Array(Vec<u8>),
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<Bytes>::deserialize(deserializer)? {
            Some(Bytes::Base64(value)) if !value.is_empty() => Base64::decode_vec(&value)
                .map(Some)
                .map_err(D::Error::custom),
            Some(Bytes::Base64(_)) | None => Ok(None),
            Some(Bytes::Array(value)) => Ok(Some(value)),
        }
    }

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&Base64::encode_string(value)),
            None => serializer.serialize_none(),
        }
    }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppButton {
    pub label: String,
    pub color: String,
    pub action: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppCard {
    pub app_id: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub cover_url: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub actions: Vec<AppButton>,

    #[serde(default)]
    pub updated_at: DateTime<Utc>,

    #[serde(default)]
    pub shareable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(
    tag = "action",
    content = "message_ids",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum PinMessagePayload {
    Pin(Vec<String>),
    Unpin(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_message_accepts_flutter_base64_keys() {
        let message: AttachmentMessage = serde_json::from_str(
            r#"{
                "key":"AAECAw==",
                "digest":[4,5,6],
                "waveform":"BwgJ",
                "attachment_id":"attachment-id",
                "mime_type":"application/octet-stream",
                "size":3
            }"#,
        )
        .unwrap();

        assert_eq!(message.key, Some(vec![0, 1, 2, 3]));
        assert_eq!(message.digest, Some(vec![4, 5, 6]));
        assert_eq!(message.waveform, Some(vec![7, 8, 9]));
        assert_eq!(serde_json::to_value(message).unwrap()["key"], "AAECAw==");
    }
}
