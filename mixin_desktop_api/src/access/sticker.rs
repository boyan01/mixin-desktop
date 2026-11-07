use mixin_desktop_core::runtime::StickerAccess as CoreStickerAccess;

use crate::{dto as model, ClientError};

pub struct StickerAccess {
    inner: CoreStickerAccess,
}

impl From<CoreStickerAccess> for StickerAccess {
    fn from(inner: CoreStickerAccess) -> Self {
        Self { inner }
    }
}

impl StickerAccess {
    pub async fn add_sticker(&self, sticker_id: String) -> Result<(), ClientError> {
        Ok(self.inner.add_sticker(sticker_id).await?)
    }

    pub async fn remove_sticker(&self, sticker_id: String) -> Result<(), ClientError> {
        Ok(self.inner.remove_sticker(sticker_id).await?)
    }

    pub async fn refresh_stickers(&self) -> Result<bool, ClientError> {
        Ok(self.inner.refresh_stickers().await?)
    }

    pub async fn refresh_sticker(&self, sticker_id: String) -> Result<(), ClientError> {
        Ok(self.inner.refresh_sticker(sticker_id).await?)
    }

    pub async fn recent_stickers(&self) -> Result<Vec<model::StickerItem>, ClientError> {
        Ok(self.inner.recent_stickers().await?)
    }

    pub async fn personal_stickers(&self) -> Result<Vec<model::StickerItem>, ClientError> {
        Ok(self.inner.personal_stickers().await?)
    }

    pub async fn sticker_albums(&self) -> Result<Vec<model::StickerAlbumItem>, ClientError> {
        Ok(self.inner.sticker_albums().await?)
    }

    pub async fn sticker_store_albums(&self) -> Result<Vec<model::StickerAlbumItem>, ClientError> {
        Ok(self.inner.sticker_store_albums().await?)
    }

    pub async fn album_stickers(
        &self,
        album_id: String,
    ) -> Result<Vec<model::StickerItem>, ClientError> {
        Ok(self.inner.album_stickers(album_id).await?)
    }

    pub async fn set_sticker_album_added(
        &self,
        album_id: String,
        added: bool,
    ) -> Result<(), ClientError> {
        Ok(self.inner.set_sticker_album_added(album_id, added).await?)
    }

    pub async fn set_sticker_album_order(&self, album_ids: Vec<String>) -> Result<(), ClientError> {
        Ok(self.inner.set_sticker_album_order(album_ids).await?)
    }

    pub async fn sticker_detail(
        &self,
        sticker_id: String,
    ) -> Result<model::StickerDetailItem, ClientError> {
        Ok(self.inner.sticker_detail(sticker_id).await?)
    }

    pub async fn add_sticker_from_file(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.add_sticker_from_file(message_id).await?)
    }

    pub async fn add_sticker_from_path(&self, path: String) -> Result<(), ClientError> {
        Ok(self.inner.add_sticker_from_path(path).await?)
    }
}
