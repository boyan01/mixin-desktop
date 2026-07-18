use std::collections::HashSet;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use log::warn;
use sdk::message_category::MessageCategory as _;

use crate::db::mixin::job::Job;
use crate::db::mixin::message::MediaStatus;

use super::{model, validate_sticker_image, AccountState, StickerDetail, MAX_STICKER_FILE_SIZE};

pub struct StickerAccess {
    state: Arc<AccountState>,
}

impl StickerAccess {
    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }
}

impl Deref for StickerAccess {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl StickerAccess {
    pub async fn add_sticker(&self, sticker_id: String) -> Result<()> {
        let sticker_id = sticker_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if sticker_id.trim().is_empty() {
            return Err(anyhow!("sticker id is required"));
        }
        let sticker = self.client.account_api.add_sticker(sticker_id).await?;
        self.database.sticker_dao.insert(&sticker).await?;
        if let Some(album_id) = sticker.album_id.as_deref().filter(|id| !id.is_empty()) {
            self.database
                .sticker_dao
                .insert_relationship(album_id, &sticker.sticker_id)
                .await?;
        }
        Ok(())
    }

    pub async fn remove_sticker(&self, sticker_id: String) -> Result<()> {
        let sticker_id = sticker_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if sticker_id.trim().is_empty() {
            return Err(anyhow!("sticker id is required"));
        }
        self.client
            .account_api
            .remove_stickers(&[sticker_id.to_string()])
            .await?;
        self.database
            .sticker_dao
            .remove_personal_relationship(sticker_id)
            .await?;
        Ok(())
    }

    pub async fn refresh_stickers(&self) -> Result<bool> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let existing_album_ids = self
            .database
            .sticker_dao
            .system_store_albums()
            .await?
            .into_iter()
            .map(|album| album.album_id)
            .collect::<HashSet<_>>();
        let albums = self.client.account_api.get_sticker_albums().await?;
        let has_new_album = albums
            .iter()
            .any(|album| !existing_album_ids.contains(&album.album_id));
        for album in albums {
            self.database.sticker_dao.insert_album(&album).await?;
            let stickers = match self
                .client
                .account_api
                .get_stickers_by_album_id(&album.album_id)
                .await
            {
                Ok(stickers) => stickers,
                Err(error) => {
                    warn!(
                        "failed to refresh stickers for album {}: {error}",
                        album.album_id
                    );
                    continue;
                }
            };
            for mut sticker in stickers {
                sticker.album_id = Some(album.album_id.clone());
                self.database.sticker_dao.insert(&sticker).await?;
                self.database
                    .sticker_dao
                    .insert_relationship(&album.album_id, &sticker.sticker_id)
                    .await?;
            }
        }
        Ok(has_new_album)
    }

    pub async fn refresh_sticker(&self, sticker_id: String) -> Result<()> {
        let sticker_id = sticker_id.as_str();
        self.ensure_active()?;
        if sticker_id.is_empty() {
            return Ok(());
        }
        self.app_service
            .job
            .add(&Job::create_update_sticker_job(sticker_id))
            .await
    }

    pub async fn recent_stickers(&self) -> Result<Vec<model::StickerItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .sticker_dao
            .recent()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn personal_stickers(&self) -> Result<Vec<model::StickerItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .sticker_dao
            .personal()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn sticker_albums(&self) -> Result<Vec<model::StickerAlbumItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .sticker_dao
            .system_albums()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn sticker_store_albums(&self) -> Result<Vec<model::StickerAlbumItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .sticker_dao
            .system_store_albums()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn album_stickers(&self, album_id: String) -> Result<Vec<model::StickerItem>> {
        let album_id = album_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .sticker_dao
            .by_album(album_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn set_sticker_album_added(&self, album_id: String, added: bool) -> Result<()> {
        let album_id = album_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .sticker_dao
            .set_album_added(album_id, added)
            .await?;
        Ok(())
    }

    pub async fn set_sticker_album_order(&self, album_ids: Vec<String>) -> Result<()> {
        let album_ids = album_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if album_ids.iter().any(|album_id| album_id.trim().is_empty()) {
            return Err(anyhow!("sticker album id is required"));
        }
        let unique: HashSet<_> = album_ids.iter().collect();
        if unique.len() != album_ids.len() {
            return Err(anyhow!("sticker album order contains duplicate ids"));
        }
        self.database.sticker_dao.set_album_order(album_ids).await?;
        Ok(())
    }

    pub async fn sticker_detail(&self, sticker_id: String) -> Result<model::StickerDetailItem> {
        let sticker_id = sticker_id.as_str();
        self.ensure_active()?;
        let sticker = match self
            .database
            .sticker_dao
            .find_sticker_by_id(sticker_id)
            .await?
        {
            Some(sticker) => sticker,
            None => {
                let sticker = self
                    .client
                    .account_api
                    .get_sticker_by_id(sticker_id)
                    .await?;
                self.database.sticker_dao.insert(&sticker).await?;
                self.database
                    .sticker_dao
                    .find_sticker_by_id(sticker_id)
                    .await?
                    .ok_or_else(|| anyhow!("failed to persist sticker: {sticker_id}"))?
            }
        };
        let mut album = self
            .database
            .sticker_dao
            .find_system_album_for_sticker(sticker_id)
            .await?
            .or(match sticker.album_id.as_deref() {
                Some(album_id) => self.database.sticker_dao.find_album_by_id(album_id).await?,
                None => None,
            })
            .filter(|album| album.category == "SYSTEM");
        if album.is_none() {
            if let Some(album_id) = sticker.album_id.as_deref() {
                match self.client.account_api.get_sticker_album(album_id).await {
                    Ok(fetched) => {
                        self.database.sticker_dao.insert_album(&fetched).await?;
                        match self
                            .client
                            .account_api
                            .get_stickers_by_album_id(album_id)
                            .await
                        {
                            Ok(stickers) => {
                                for mut album_sticker in stickers {
                                    album_sticker.album_id = Some(album_id.to_string());
                                    self.database.sticker_dao.insert(&album_sticker).await?;
                                    self.database
                                        .sticker_dao
                                        .insert_relationship(album_id, &album_sticker.sticker_id)
                                        .await?;
                                }
                            }
                            Err(error) => {
                                warn!("failed to fetch stickers for album {album_id}: {error}");
                                self.database
                                    .sticker_dao
                                    .insert_relationship(album_id, sticker_id)
                                    .await?;
                            }
                        }
                        album = self
                            .database
                            .sticker_dao
                            .find_album_by_id(album_id)
                            .await?
                            .filter(|album| album.category == "SYSTEM");
                    }
                    Err(error) => warn!("failed to fetch sticker album {album_id}: {error}"),
                }
            }
        }
        let album_stickers = match album.as_ref() {
            Some(album) => self.database.sticker_dao.by_album(&album.album_id).await?,
            None => Vec::new(),
        };
        let is_personal = self.database.sticker_dao.is_personal(sticker_id).await?;
        Ok(StickerDetail {
            sticker,
            album,
            album_stickers,
            is_personal,
        }
        .into())
    }

    pub async fn add_sticker_from_file(&self, message_id: String) -> Result<()> {
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("image message not found: {message_id}"))?;
        if !message.category.is_image() {
            return Err(anyhow!("message is not an image: {message_id}"));
        }
        if !matches!(message.media_status, MediaStatus::Done | MediaStatus::Read) {
            return Err(anyhow!("image is not available locally: {message_id}"));
        }
        let path = Path::new(
            message
                .media_url
                .as_deref()
                .filter(|path| !path.is_empty())
                .ok_or_else(|| anyhow!("image has no local file: {message_id}"))?,
        );
        let bytes = self
            .app_service
            .attachment
            .read_account_file(path, MAX_STICKER_FILE_SIZE as u64)
            .await?;
        self.add_sticker_image(path, &bytes).await
    }

    pub async fn add_sticker_from_path(&self, path: String) -> Result<()> {
        let path = path.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let path = tokio::fs::canonicalize(path)
            .await
            .with_context(|| format!("resolve sticker image {path}"))?;
        let metadata = tokio::fs::metadata(&path).await?;
        if !metadata.is_file() {
            return Err(anyhow!("sticker image path is not a file"));
        }
        if metadata.len() > MAX_STICKER_FILE_SIZE as u64 {
            return Err(anyhow!("sticker image exceeds the 1MB size limit"));
        }
        let bytes = tokio::fs::read(&path).await?;
        self.add_sticker_image(&path, &bytes).await
    }

    async fn add_sticker_image(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        validate_sticker_image(path, bytes)?;
        let mut personal_album_id = self.database.sticker_dao.find_personal_album_id().await?;
        let encoded = Base64::encode_string(bytes);
        let sticker = self.client.account_api.add_sticker_data(&encoded).await?;
        if personal_album_id.is_none() {
            match self.client.account_api.get_sticker_albums().await {
                Ok(albums) => {
                    for album in albums {
                        self.database.sticker_dao.insert_album(&album).await?;
                        if album.category == "PERSONAL" {
                            personal_album_id = Some(album.album_id);
                        }
                    }
                }
                Err(error) => warn!("failed to refresh personal sticker album: {error}"),
            }
        }
        let album_id = sticker
            .album_id
            .clone()
            .filter(|album_id| !album_id.is_empty())
            .or(personal_album_id);
        self.database.sticker_dao.insert(&sticker).await?;
        if let Some(album_id) = album_id {
            self.database
                .sticker_dao
                .insert_relationship(&album_id, &sticker.sticker_id)
                .await?;
        }
        Ok(())
    }
}
