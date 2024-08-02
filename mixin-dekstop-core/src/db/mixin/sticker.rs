use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::Error;

#[derive(Clone)]
pub struct StickerDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Sticker {
    pub sticker_id: String,
    pub album_id: Option<String>,
    pub name: String,
    pub asset_url: String,
    pub asset_width: i32,
    pub asset_height: i32,
    pub asset_name: String,
    pub asset_type: String,
    pub created_at: DateTime<Utc>,
    pub last_use_at: Option<DateTime<Utc>>,
}

impl StickerDao {
    pub async fn find_sticker_by_id(&self, sticker_id: &str) -> Result<Option<Sticker>, Error> {
        let result = sqlx::query_as::<_, Sticker>("SELECT * FROM stickers WHERE sticker_id = ?")
            .bind(sticker_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }
}
