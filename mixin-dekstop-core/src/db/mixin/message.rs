use chrono::NaiveDateTime;

use crate::db::Error;
use crate::db::mixin::MixinDatabase;

#[derive(Default)]
pub struct Message {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub category: String,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i32>,
    pub media_duration: Option<String>,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub media_hash: Option<String>,
    pub thumb_image: Option<String>,
    pub media_key: Option<String>,
    pub media_digest: Option<String>,
    pub media_status: Option<String>,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub action: Option<String>,
    pub participant_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub hyperlink: Option<String>,
    pub name: Option<String>,
    pub album_id: Option<String>,
    pub sticker_id: Option<String>,
    pub shared_user_id: Option<String>,
    pub media_waveform: Option<String>,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub thumb_url: Option<String>,
    pub caption: Option<String>,
}

impl MixinDatabase {
    pub fn is_message_exits(&self, message_id: &String) -> Result<bool, Error> {
        todo!()
    }

    pub fn insert_message(&self, message: &Message) -> Result<(), Error> {
        todo!()
    }
}