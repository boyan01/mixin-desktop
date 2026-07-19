use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::Error;

#[derive(Clone)]
pub struct StickerDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Sticker {
    pub sticker_id: String,
    pub album_id: Option<String>,
    pub name: String,
    pub asset_url: String,
    pub asset_width: i32,
    pub asset_height: i32,
    pub asset_type: String,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
    #[sqlx(try_from = "crate::db::datetime::OptionalDatabaseDateTime")]
    pub last_use_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StickerAlbum {
    pub album_id: String,
    pub name: String,
    pub icon_url: String,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub update_at: DateTime<Utc>,
    pub ordered_at: i64,
    pub user_id: String,
    pub category: String,
    pub description: String,
    pub banner: Option<String>,
    pub added: bool,
    pub is_verified: bool,
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
        .bind(sticker.created_at.timestamp_millis())
        .bind(sticker.last_use_at.map(|value| value.timestamp_millis()))
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn insert_album(&self, album: &sdk::StickerAlbum) -> Result<(), Error> {
        let ordered_at = match sqlx::query_scalar::<_, i64>(
            "SELECT ordered_at FROM sticker_albums WHERE album_id = ?",
        )
        .bind(&album.album_id)
        .fetch_optional(&self.0)
        .await?
        {
            Some(ordered_at) => ordered_at,
            None => {
                sqlx::query_scalar::<_, i64>(
                    "SELECT COALESCE(MAX(ordered_at), 0) + 1 FROM sticker_albums",
                )
                .fetch_one(&self.0)
                .await?
            }
        };
        sqlx::query(
            r#"INSERT INTO sticker_albums
               (album_id, name, icon_url, created_at, update_at, ordered_at, user_id,
                category, description, banner, added, is_verified)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(album_id) DO UPDATE SET
                 name = excluded.name, icon_url = excluded.icon_url,
                 created_at = excluded.created_at, update_at = excluded.update_at,
                 user_id = excluded.user_id, category = excluded.category,
                 description = excluded.description, banner = excluded.banner,
                 is_verified = excluded.is_verified"#,
        )
        .bind(&album.album_id)
        .bind(&album.name)
        .bind(&album.icon_url)
        .bind(album.created_at.timestamp_millis())
        .bind(album.update_at.timestamp_millis())
        .bind(ordered_at)
        .bind(&album.user_id)
        .bind(&album.category)
        .bind(&album.description)
        .bind(&album.banner)
        .bind(
            album
                .banner
                .as_ref()
                .is_some_and(|banner| !banner.is_empty()),
        )
        .bind(album.is_verified)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn recent(&self) -> Result<Vec<Sticker>, Error> {
        Ok(sqlx::query_as::<_, Sticker>(
            "SELECT * FROM stickers WHERE last_use_at IS NOT NULL ORDER BY last_use_at DESC LIMIT 20",
        )
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn personal(&self) -> Result<Vec<Sticker>, Error> {
        self.by_album_category("PERSONAL").await
    }

    async fn by_album_category(&self, category: &str) -> Result<Vec<Sticker>, Error> {
        Ok(sqlx::query_as::<_, Sticker>(
            r#"SELECT s.* FROM sticker_albums a
               INNER JOIN sticker_relationships r ON r.album_id = a.album_id
               INNER JOIN stickers s ON s.sticker_id = r.sticker_id
               WHERE a.category = ? ORDER BY s.created_at DESC"#,
        )
        .bind(category)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn system_albums(&self) -> Result<Vec<StickerAlbum>, Error> {
        Ok(sqlx::query_as::<_, StickerAlbum>(
            r#"SELECT * FROM sticker_albums
               WHERE category = 'SYSTEM' AND added = TRUE
               ORDER BY ordered_at, created_at DESC"#,
        )
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn system_store_albums(&self) -> Result<Vec<StickerAlbum>, Error> {
        Ok(sqlx::query_as::<_, StickerAlbum>(
            r#"SELECT * FROM sticker_albums
               WHERE category = 'SYSTEM' AND is_verified = TRUE
               ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn find_album_by_id(&self, album_id: &str) -> Result<Option<StickerAlbum>, Error> {
        Ok(sqlx::query_as::<_, StickerAlbum>(
            "SELECT * FROM sticker_albums WHERE album_id = ? LIMIT 1",
        )
        .bind(album_id)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn find_system_album_for_sticker(
        &self,
        sticker_id: &str,
    ) -> Result<Option<StickerAlbum>, Error> {
        Ok(sqlx::query_as::<_, StickerAlbum>(
            r#"SELECT a.* FROM sticker_albums a
               INNER JOIN sticker_relationships r ON r.album_id = a.album_id
               WHERE r.sticker_id = ? AND a.category = 'SYSTEM' LIMIT 1"#,
        )
        .bind(sticker_id)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn by_album(&self, album_id: &str) -> Result<Vec<Sticker>, Error> {
        Ok(sqlx::query_as::<_, Sticker>(
            r#"SELECT s.* FROM sticker_relationships r
               INNER JOIN stickers s ON s.sticker_id = r.sticker_id
               WHERE r.album_id = ? ORDER BY s.created_at DESC"#,
        )
        .bind(album_id)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn update_last_used(&self, sticker_id: &str) -> Result<(), Error> {
        sqlx::query("UPDATE stickers SET last_use_at = ? WHERE sticker_id = ?")
            .bind(Utc::now().timestamp_millis())
            .bind(sticker_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn is_personal(&self, sticker_id: &str) -> Result<bool, Error> {
        Ok(sqlx::query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM sticker_relationships r
               INNER JOIN sticker_albums a ON a.album_id = r.album_id
               WHERE r.sticker_id = ? AND a.category = 'PERSONAL')"#,
        )
        .bind(sticker_id)
        .fetch_one(&self.0)
        .await?)
    }

    pub async fn set_album_added(&self, album_id: &str, added: bool) -> Result<(), Error> {
        sqlx::query("UPDATE sticker_albums SET added = ? WHERE album_id = ?")
            .bind(added)
            .bind(album_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn set_album_order(&self, album_ids: &[String]) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        for (ordered_at, album_id) in album_ids.iter().enumerate() {
            sqlx::query("UPDATE sticker_albums SET ordered_at = ? WHERE album_id = ?")
                .bind(ordered_at as i64)
                .bind(album_id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn remove_personal_relationship(&self, sticker_id: &str) -> Result<(), Error> {
        sqlx::query(
            r#"DELETE FROM sticker_relationships WHERE sticker_id = ? AND album_id IN
               (SELECT album_id FROM sticker_albums WHERE category = 'PERSONAL')"#,
        )
        .bind(sticker_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn insert_relationship(&self, album_id: &str, sticker_id: &str) -> Result<(), Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO sticker_relationships (album_id, sticker_id) VALUES (?, ?)",
        )
        .bind(album_id)
        .bind(sticker_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn find_personal_album_id(&self) -> Result<Option<String>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT album_id FROM sticker_albums WHERE category = 'PERSONAL' LIMIT 1",
        )
        .fetch_optional(&self.0)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::db::MixinDatabase;

    fn album(id: &str, category: &str, banner: Option<&str>) -> sdk::StickerAlbum {
        sdk::StickerAlbum {
            album_id: id.to_string(),
            name: format!("{category} album"),
            icon_url: "https://example.com/icon.png".to_string(),
            created_at: Utc::now(),
            update_at: Utc::now(),
            user_id: "user".to_string(),
            category: category.to_string(),
            description: String::new(),
            banner: banner.map(str::to_string),
            is_verified: true,
        }
    }

    fn sticker(id: &str, album_id: &str, created_at: DateTime<Utc>) -> sdk::Sticker {
        sdk::Sticker {
            sticker_id: id.to_string(),
            album_id: Some(album_id.to_string()),
            name: id.to_string(),
            asset_url: format!("https://example.com/{id}.webp"),
            asset_type: "image/webp".to_string(),
            asset_width: 512,
            asset_height: 512,
            created_at,
            last_use_at: None,
        }
    }

    #[tokio::test]
    async fn queries_picker_groups_and_sticker_detail_data() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = &database.sticker_dao;
        let now = Utc::now();

        dao.insert_album(&album("personal", "PERSONAL", None))
            .await
            .unwrap();
        dao.insert_album(&album("system", "SYSTEM", Some("banner")))
            .await
            .unwrap();
        dao.insert(&sticker("favorite", "personal", now))
            .await
            .unwrap();
        dao.insert(&sticker("system-old", "system", now - Duration::days(1)))
            .await
            .unwrap();
        dao.insert(&sticker("system-new", "system", now))
            .await
            .unwrap();
        dao.insert_relationship("personal", "favorite")
            .await
            .unwrap();
        dao.insert_relationship("system", "system-old")
            .await
            .unwrap();
        dao.insert_relationship("system", "system-new")
            .await
            .unwrap();
        dao.update_last_used("system-old").await.unwrap();

        assert_eq!(dao.personal().await.unwrap()[0].sticker_id, "favorite");
        assert_eq!(dao.recent().await.unwrap()[0].sticker_id, "system-old");
        assert_eq!(dao.system_albums().await.unwrap()[0].album_id, "system");
        assert_eq!(
            dao.system_store_albums().await.unwrap()[0].album_id,
            "system"
        );
        let album_stickers = dao.by_album("system").await.unwrap();
        assert_eq!(album_stickers[0].sticker_id, "system-new");
        assert_eq!(album_stickers[1].sticker_id, "system-old");
        assert_eq!(
            dao.find_system_album_for_sticker("system-new")
                .await
                .unwrap()
                .unwrap()
                .album_id,
            "system"
        );
        assert!(dao.is_personal("favorite").await.unwrap());

        dao.insert_album(&album("system-2", "SYSTEM", Some("banner")))
            .await
            .unwrap();
        assert_eq!(dao.system_albums().await.unwrap()[0].album_id, "system");
        dao.set_album_order(&["system-2".to_string(), "system".to_string()])
            .await
            .unwrap();
        assert_eq!(dao.system_albums().await.unwrap()[0].album_id, "system-2");

        dao.remove_personal_relationship("favorite").await.unwrap();
        assert!(!dao.is_personal("favorite").await.unwrap());
        dao.set_album_added("system", false).await.unwrap();
        dao.set_album_added("system-2", false).await.unwrap();
        assert!(dao.system_albums().await.unwrap().is_empty());
        assert_eq!(dao.system_store_albums().await.unwrap().len(), 2);
    }
}
