use std::collections::HashMap;

use mixin_desktop_core::runtime::{
    AttachmentAccess as CoreAttachmentAccess, ConversationAccess as CoreConversationAccess,
    MessageAccess as CoreMessageAccess, StickerAccess as CoreStickerAccess,
    UserAccess as CoreUserAccess,
};

pub use crate::dto::*;
use crate::{dto as model, ClientError};

pub struct ConversationAccess {
    inner: CoreConversationAccess,
}

impl From<CoreConversationAccess> for ConversationAccess {
    fn from(inner: CoreConversationAccess) -> Self {
        Self { inner }
    }
}

impl ConversationAccess {
    pub async fn resolve_code(&self, code: String) -> Result<model::CodeResult, ClientError> {
        Ok(self.inner.resolve_code(code).await?.into())
    }

    pub async fn join_group(&self, code: String) -> Result<String, ClientError> {
        Ok(self.inner.join_group(code).await?)
    }

    pub async fn open_user_conversation(&self, user_id: String) -> Result<String, ClientError> {
        Ok(self.inner.open_user_conversation(user_id).await?)
    }

    pub async fn conversation_items(
        &self,
    ) -> Result<Vec<model::ConversationListData>, ClientError> {
        Ok(self
            .inner
            .conversation_items()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn conversation_items_by_ids(
        &self,
        conversation_ids: Vec<String>,
    ) -> Result<Vec<model::ConversationListData>, ClientError> {
        Ok(self
            .inner
            .conversation_items_by_ids(conversation_ids)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn is_bot_group(&self, conversation_id: String) -> Result<bool, ClientError> {
        Ok(self.inner.is_bot_group(conversation_id).await?)
    }

    pub async fn conversation_count(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
    ) -> Result<i64, ClientError> {
        Ok(self
            .inner
            .conversation_count(category, circle_id, keyword, unseen_only)
            .await?)
    }

    pub async fn conversations(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<model::ConversationListData>, ClientError> {
        Ok(self
            .inner
            .conversations(category, circle_id, keyword, unseen_only, limit, offset)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn current_user_role(
        &self,
        conversation_id: String,
    ) -> Result<Option<String>, ClientError> {
        Ok(self.inner.current_user_role(conversation_id).await?)
    }

    pub async fn conversation_participants(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::ConversationParticipantItem>, ClientError> {
        Ok(self
            .inner
            .conversation_participants(conversation_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_bot_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<model::ConversationParticipantItem>, ClientError> {
        Ok(self
            .inner
            .search_bot_group_users(conversation_id, keyword)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<model::ConversationParticipantItem>, ClientError> {
        Ok(self
            .inner
            .search_group_users(conversation_id, keyword)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<model::ConversationDetailItem, ClientError> {
        Ok(self
            .inner
            .conversation_detail(conversation_id)
            .await?
            .into())
    }

    pub async fn local_conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<model::ConversationDetailItem, ClientError> {
        Ok(self
            .inner
            .local_conversation_detail(conversation_id)
            .await?
            .into())
    }

    pub async fn groups_in_common(
        &self,
        user_id: String,
    ) -> Result<Vec<model::GroupConversationItem>, ClientError> {
        Ok(self
            .inner
            .groups_in_common(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn update_participants(
        &self,
        conversation_id: String,
        action: String,
        user_ids: Vec<String>,
        role: Option<String>,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .update_participants(conversation_id, action, user_ids, role)
            .await?)
    }

    pub async fn set_disappearing_messages(
        &self,
        conversation_id: String,
        duration: i64,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .set_disappearing_messages(conversation_id, duration)
            .await?)
    }

    pub async fn update_draft(
        &self,
        conversation_id: String,
        draft: String,
    ) -> Result<(), ClientError> {
        Ok(self.inner.update_draft(conversation_id, draft).await?)
    }

    pub async fn create_circle(&self, name: String) -> Result<model::CircleItem, ClientError> {
        Ok(self.inner.create_circle(name).await?.into())
    }

    pub async fn update_circle(&self, circle_id: String, name: String) -> Result<(), ClientError> {
        Ok(self.inner.update_circle(circle_id, name).await?)
    }

    pub async fn delete_circle(&self, circle_id: String) -> Result<(), ClientError> {
        Ok(self.inner.delete_circle(circle_id).await?)
    }

    pub async fn reorder_circles(&self, circle_ids: Vec<String>) -> Result<(), ClientError> {
        Ok(self.inner.reorder_circles(circle_ids).await?)
    }

    pub async fn create_group(
        &self,
        name: String,
        user_ids: Vec<String>,
    ) -> Result<String, ClientError> {
        Ok(self.inner.create_group(name, user_ids).await?)
    }

    pub async fn edit_conversation(
        &self,
        conversation_id: String,
        name: Option<String>,
        announcement: Option<String>,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .edit_conversation(conversation_id, name, announcement)
            .await?)
    }

    pub async fn exit_group(&self, conversation_id: String) -> Result<(), ClientError> {
        Ok(self.inner.exit_group(conversation_id).await?)
    }

    pub async fn rotate_group_invite(&self, conversation_id: String) -> Result<(), ClientError> {
        Ok(self.inner.rotate_group_invite(conversation_id).await?)
    }

    pub async fn clear_conversation(&self, conversation_id: String) -> Result<(), ClientError> {
        Ok(self.inner.clear_conversation(conversation_id).await?)
    }

    pub async fn circles(&self) -> Result<Vec<model::CircleItem>, ClientError> {
        Ok(self
            .inner
            .circles()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn set_pinned(
        &self,
        conversation_id: String,
        pinned: bool,
    ) -> Result<(), ClientError> {
        Ok(self.inner.set_pinned(conversation_id, pinned).await?)
    }

    pub async fn set_muted(
        &self,
        conversation_id: String,
        owner_id: String,
        category: String,
        duration_seconds: i64,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .set_muted(conversation_id, owner_id, category, duration_seconds)
            .await?)
    }

    pub async fn delete_conversation(&self, conversation_id: String) -> Result<(), ClientError> {
        Ok(self.inner.delete_conversation(conversation_id).await?)
    }

    pub async fn edit_circle_conversation(
        &self,
        circle_id: String,
        conversation_id: String,
        owner_id: String,
        is_group: bool,
        add: bool,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .edit_circle_conversation(circle_id, conversation_id, owner_id, is_group, add)
            .await?)
    }
}

pub struct MessageAccess {
    inner: CoreMessageAccess,
}

impl From<CoreMessageAccess> for MessageAccess {
    fn from(inner: CoreMessageAccess) -> Self {
        Self { inner }
    }
}

impl MessageAccess {
    #[allow(clippy::too_many_arguments)]
    pub async fn send_remote_image(
        &self,
        conversation_id: String,
        url: String,
        preview_url: String,
        width: Option<i32>,
        height: Option<i32>,
        mime_type: String,
        silent: bool,
    ) -> Result<String, ClientError> {
        Ok(self
            .inner
            .send_remote_image(
                conversation_id,
                url,
                preview_url,
                width,
                height,
                mime_type,
                silent,
            )
            .await?)
    }

    pub async fn messages(
        &self,
        conversation_id: String,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<String>,
        limit: i64,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .messages(
                conversation_id,
                before_created_at_micros,
                before_message_id,
                limit,
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn message_items_by_ids(
        &self,
        message_ids: Vec<String>,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .message_items_by_ids(message_ids)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn message_order_info(
        &self,
        message_id: String,
    ) -> Result<Option<model::MessageOrderInfoView>, ClientError> {
        Ok(self
            .inner
            .message_order_info(message_id)
            .await?
            .map(Into::into))
    }

    pub async fn message_ids_before(
        &self,
        conversation_id: String,
        anchor_row_id: i64,
        anchor_created_at_micros: i64,
        limit: i64,
    ) -> Result<Vec<String>, ClientError> {
        Ok(self
            .inner
            .message_ids_before(
                conversation_id,
                anchor_row_id,
                anchor_created_at_micros,
                limit,
            )
            .await?)
    }

    pub async fn message_ids_after(
        &self,
        conversation_id: String,
        anchor_row_id: i64,
        anchor_created_at_micros: i64,
        limit: i64,
    ) -> Result<Vec<String>, ClientError> {
        Ok(self
            .inner
            .message_ids_after(
                conversation_id,
                anchor_row_id,
                anchor_created_at_micros,
                limit,
            )
            .await?)
    }

    pub async fn search_messages(
        &self,
        conversation_id: String,
        query: String,
        sender_id: Option<String>,
        categories: Vec<String>,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .search_messages(
                conversation_id,
                query,
                sender_id,
                categories,
                anchor_message_id,
                limit,
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_global_messages(
        &self,
        query: String,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .search_global_messages(query, anchor_message_id, limit)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn shared_messages(
        &self,
        conversation_id: String,
        kind: String,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .shared_messages(conversation_id, kind, offset, limit)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .messages_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn image_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::ImageMessageView>, ClientError> {
        Ok(self
            .inner
            .image_messages_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn pinned_messages(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .pinned_messages(conversation_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn pinned_message_ids(
        &self,
        conversation_id: String,
    ) -> Result<Vec<String>, ClientError> {
        Ok(self.inner.pinned_message_ids(conversation_id).await?)
    }

    pub async fn pin_message_preview(
        &self,
        conversation_id: String,
    ) -> Result<Option<model::PinMessagePreviewItem>, ClientError> {
        Ok(self
            .inner
            .pin_message_preview(conversation_id)
            .await?
            .map(Into::into))
    }

    pub async fn transcript_messages(
        &self,
        transcript_id: String,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self
            .inner
            .transcript_messages(transcript_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn send_text(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String, ClientError> {
        Ok(self
            .inner
            .send_text(conversation_id, content, quote_message_id, silent)
            .await?)
    }

    pub async fn conversation_is_encrypted(
        &self,
        conversation_id: String,
    ) -> Result<bool, ClientError> {
        Ok(self
            .inner
            .conversation_is_encrypted(conversation_id)
            .await?)
    }

    pub async fn send_post(
        &self,
        conversation_id: String,
        content: String,
    ) -> Result<String, ClientError> {
        Ok(self.inner.send_post(conversation_id, content).await?)
    }

    pub async fn send_app_card(
        &self,
        conversation_id: String,
        content: String,
    ) -> Result<String, ClientError> {
        Ok(self.inner.send_app_card(conversation_id, content).await?)
    }

    pub async fn send_contact(
        &self,
        conversation_id: String,
        shared_user_id: String,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String, ClientError> {
        Ok(self
            .inner
            .send_contact(conversation_id, shared_user_id, quote_message_id, silent)
            .await?)
    }

    pub async fn send_sticker(
        &self,
        conversation_id: String,
        sticker_id: String,
    ) -> Result<String, ClientError> {
        Ok(self.inner.send_sticker(conversation_id, sticker_id).await?)
    }

    pub async fn send_audio(
        &self,
        conversation_id: String,
        path: String,
        duration_millis: i64,
        waveform: Vec<u8>,
        quote_message_id: Option<String>,
    ) -> Result<String, ClientError> {
        Ok(self
            .inner
            .send_audio(
                conversation_id,
                path,
                duration_millis,
                waveform,
                quote_message_id,
            )
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_attachment(
        &self,
        conversation_id: String,
        path: String,
        kind: String,
        mime_type: String,
        name: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
        duration_millis: Option<i64>,
        thumbnail: Option<String>,
        caption: Option<String>,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String, ClientError> {
        Ok(self
            .inner
            .send_attachment(
                conversation_id,
                path,
                kind,
                mime_type,
                name,
                width,
                height,
                duration_millis,
                thumbnail,
                caption,
                quote_message_id,
                silent,
            )
            .await?)
    }

    pub async fn forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<Vec<String>, ClientError> {
        Ok(self
            .inner
            .forward_messages(target_conversation_id, source_message_ids)
            .await?)
    }

    pub async fn combine_forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<String, ClientError> {
        Ok(self
            .inner
            .combine_forward_messages(target_conversation_id, source_message_ids)
            .await?)
    }

    pub async fn delete_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .delete_messages(conversation_id, message_ids)
            .await?)
    }

    pub async fn recall_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .recall_messages(conversation_id, message_ids)
            .await?)
    }

    pub async fn set_message_pinned(
        &self,
        conversation_id: String,
        message_id: String,
        pinned: bool,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .set_message_pinned(conversation_id, message_id, pinned)
            .await?)
    }

    pub async fn mark_mention_read(
        &self,
        conversation_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .mark_mention_read(conversation_id, message_id)
            .await?)
    }

    pub async fn unread_mention_message_ids(
        &self,
        conversation_id: String,
    ) -> Result<Vec<String>, ClientError> {
        Ok(self
            .inner
            .unread_mention_message_ids(conversation_id)
            .await?)
    }

    pub async fn mark_conversation_read(&self, conversation_id: String) -> Result<(), ClientError> {
        Ok(self.inner.mark_conversation_read(conversation_id).await?)
    }
}

pub struct AttachmentAccess {
    inner: CoreAttachmentAccess,
}

impl From<CoreAttachmentAccess> for AttachmentAccess {
    fn from(inner: CoreAttachmentAccess) -> Self {
        Self { inner }
    }
}

impl AttachmentAccess {
    pub async fn retry_attachment(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.retry_attachment(message_id).await?)
    }

    pub async fn download_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .download_transcript_attachment(transcript_id, message_id)
            .await?)
    }

    pub async fn retry_transcript_attachment(
        &self,
        transcript_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .retry_transcript_attachment(transcript_id)
            .await?)
    }

    pub async fn cancel_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .cancel_transcript_attachment(transcript_id, message_id)
            .await?)
    }

    pub async fn mark_transcript_audio_read(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .mark_transcript_audio_read(transcript_id, message_id)
            .await?)
    }

    pub async fn download_attachment(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.download_attachment(message_id).await?)
    }

    pub async fn cancel_attachment(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.cancel_attachment(message_id).await?)
    }

    pub async fn mark_audio_read(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.mark_audio_read(message_id).await?)
    }
}

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
        Ok(self
            .inner
            .recent_stickers()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn personal_stickers(&self) -> Result<Vec<model::StickerItem>, ClientError> {
        Ok(self
            .inner
            .personal_stickers()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn sticker_albums(&self) -> Result<Vec<model::StickerAlbumItem>, ClientError> {
        Ok(self
            .inner
            .sticker_albums()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn sticker_store_albums(&self) -> Result<Vec<model::StickerAlbumItem>, ClientError> {
        Ok(self
            .inner
            .sticker_store_albums()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn album_stickers(
        &self,
        album_id: String,
    ) -> Result<Vec<model::StickerItem>, ClientError> {
        Ok(self
            .inner
            .album_stickers(album_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
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
        Ok(self.inner.sticker_detail(sticker_id).await?.into())
    }

    pub async fn add_sticker_from_file(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.add_sticker_from_file(message_id).await?)
    }

    pub async fn add_sticker_from_path(&self, path: String) -> Result<(), ClientError> {
        Ok(self.inner.add_sticker_from_path(path).await?)
    }
}

pub struct UserAccess {
    inner: CoreUserAccess,
}

impl From<CoreUserAccess> for UserAccess {
    fn from(inner: CoreUserAccess) -> Self {
        Self { inner }
    }
}

impl UserAccess {
    pub async fn search_mao_user(
        &self,
        query: String,
    ) -> Result<Option<model::UserProfileItem>, ClientError> {
        Ok(self.inner.search_mao_user(query).await?.map(Into::into))
    }

    pub async fn selectable_users(&self) -> Result<Vec<model::UserProfileItem>, ClientError> {
        Ok(self
            .inner
            .selectable_users()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_local_users(
        &self,
        query: String,
        category: String,
        limit: i64,
    ) -> Result<Vec<model::UserProfileItem>, ClientError> {
        Ok(self
            .inner
            .search_local_users(query, category, limit)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_user(&self, query: String) -> Result<model::UserProfileItem, ClientError> {
        Ok(self.inner.search_user(query).await?.into())
    }

    pub async fn local_shared_apps(
        &self,
        user_id: String,
    ) -> Result<Vec<model::SharedAppItem>, ClientError> {
        Ok(self
            .inner
            .local_shared_apps(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn shared_apps(
        &self,
        user_id: String,
    ) -> Result<Vec<model::SharedAppItem>, ClientError> {
        Ok(self
            .inner
            .shared_apps(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn bot_creator_id(&self, user_id: String) -> Result<Option<String>, ClientError> {
        Ok(self.inner.bot_creator_id(user_id).await?)
    }

    pub async fn user_profile(
        &self,
        user_id: Option<String>,
        identity_number: Option<String>,
    ) -> Result<Option<model::UserProfileItem>, ClientError> {
        Ok(self
            .inner
            .user_profile(user_id, identity_number)
            .await?
            .map(Into::into))
    }

    pub async fn refresh_user_profile(
        &self,
        user_id: String,
    ) -> Result<Option<model::UserProfileItem>, ClientError> {
        Ok(self
            .inner
            .refresh_user_profile(user_id)
            .await?
            .map(Into::into))
    }

    pub async fn users_by_identity_numbers(
        &self,
        identity_numbers: Vec<String>,
    ) -> Result<Vec<model::UserProfileItem>, ClientError> {
        Ok(self
            .inner
            .users_by_identity_numbers(identity_numbers)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn replace_mentions(
        &self,
        contents: Vec<String>,
    ) -> Result<Vec<String>, ClientError> {
        Ok(self.inner.replace_mentions(contents).await?)
    }

    pub async fn mention_names(
        &self,
        contents: Vec<String>,
    ) -> Result<HashMap<String, String>, ClientError> {
        Ok(self.inner.mention_names(contents).await?)
    }

    pub async fn add_contact(&self, user_id: String, full_name: String) -> Result<(), ClientError> {
        Ok(self.inner.add_contact(user_id, full_name).await?)
    }

    pub async fn block_user(&self, user_id: String) -> Result<(), ClientError> {
        Ok(self.inner.block_user(user_id).await?)
    }

    pub async fn remove_contact(&self, user_id: String) -> Result<(), ClientError> {
        Ok(self.inner.remove_contact(user_id).await?)
    }

    pub async fn unblock_user(&self, user_id: String) -> Result<(), ClientError> {
        Ok(self.inner.unblock_user(user_id).await?)
    }

    pub async fn report_user(&self, user_id: String) -> Result<(), ClientError> {
        Ok(self.inner.report_user(user_id).await?)
    }

    pub async fn bot_home_uri(&self, app_id: String) -> Result<Option<String>, ClientError> {
        Ok(self.inner.bot_home_uri(app_id).await?)
    }
}
