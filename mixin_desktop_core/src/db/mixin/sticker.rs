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

    pub async fn insert(&self, sticker: &sdk::Sticker) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO stickers
               (sticker_id, album_id, name, asset_url, asset_type, asset_width,
                asset_height, created_at, last_use_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&sticker.sticker_id)
        .bind(&sticker.album_id)
        .bind(&sticker.name)
        .bind(&sticker.asset_url)
        .bind(&sticker.asset_type)
        .bind(sticker.asset_width)
        .bind(sticker.asset_height)
        .bind(sticker.created_at)
        .bind(sticker.last_use_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
