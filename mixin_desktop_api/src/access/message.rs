use mixin_desktop_core::runtime::MessageAccess as CoreMessageAccess;

use crate::{dto as model, ClientError};

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
            .await?)
    }

    pub async fn message_items_by_ids(
        &self,
        message_ids: Vec<String>,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self.inner.message_items_by_ids(message_ids).await?)
    }

    pub async fn message_order_info(
        &self,
        message_id: String,
    ) -> Result<Option<model::MessageOrderInfoView>, ClientError> {
        Ok(self.inner.message_order_info(message_id).await?)
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
            .await?)
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
            .await?)
    }

    /// Applies conversation category, circle and unread scope before message pagination.
    pub async fn search_scoped_messages(
        &self,
        query: String,
        category: String,
        circle_id: Option<String>,
        unseen_only: bool,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .search_scoped_messages(
                query,
                category,
                circle_id,
                unseen_only,
                anchor_message_id,
                limit,
            )
            .await?)
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
            .await?)
    }

    /// Adjacent messages, newest first, excluding the supplied anchor (at most 100).
    pub async fn messages_relative(
        &self,
        conversation_id: String,
        anchor_message_id: String,
        older: bool,
        limit: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .messages_relative(conversation_id, anchor_message_id, older, limit)
            .await?)
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
            .await?)
    }

    pub async fn first_unread_message_id(
        &self,
        conversation_id: String,
    ) -> crate::ClientResult<Option<String>> {
        Ok(self.inner.first_unread_message_id(conversation_id).await?)
    }

    pub async fn pinned_audio_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .pinned_audio_messages_around(conversation_id, target_message_id, before, after)
            .await?)
    }

    pub async fn pinned_image_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> crate::ClientResult<Vec<model::ImageMessageView>> {
        Ok(self
            .inner
            .pinned_image_messages_around(conversation_id, target_message_id, before, after)
            .await?)
    }

    pub async fn audio_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .audio_messages_around(conversation_id, target_message_id, before, after)
            .await?)
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
            .await?)
    }

    pub async fn pinned_messages_page(
        &self,
        conversation_id: String,
        offset: i64,
        limit: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .pinned_messages_page(conversation_id, offset, limit)
            .await?)
    }

    pub async fn pinned_message_count(&self, conversation_id: String) -> crate::ClientResult<i64> {
        Ok(self.inner.pinned_message_count(conversation_id).await?)
    }

    pub async fn pinned_message_offset(
        &self,
        conversation_id: String,
        message_id: String,
    ) -> crate::ClientResult<Option<usize>> {
        Ok(self
            .inner
            .pinned_message_offset(conversation_id, message_id)
            .await?)
    }

    pub async fn unpin_all_messages(&self, conversation_id: String) -> crate::ClientResult<()> {
        Ok(self.inner.unpin_all_messages(conversation_id).await?)
    }

    pub async fn can_unpin_messages(&self, conversation_id: String) -> crate::ClientResult<bool> {
        Ok(self.inner.can_unpin_messages(conversation_id).await?)
    }

    pub async fn pinned_messages(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self.inner.pinned_messages(conversation_id).await?)
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
        Ok(self.inner.pin_message_preview(conversation_id).await?)
    }

    pub async fn transcript_messages(
        &self,
        transcript_id: String,
    ) -> Result<Vec<model::MessageListView>, ClientError> {
        Ok(self.inner.transcript_messages(transcript_id).await?)
    }

    /// Loads a transcript page in persisted order; the business layer caps pages at 200.
    pub async fn transcript_messages_page(
        &self,
        transcript_id: String,
        offset: i64,
        limit: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .transcript_messages_page(transcript_id, offset, limit)
            .await?)
    }

    /// Loads playable audio neighbors from the same persisted forwarded transcript.
    pub async fn transcript_audio_messages_around(
        &self,
        transcript_id: String,
        anchor_message_id: String,
        before: i64,
        after: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .transcript_audio_messages_around(transcript_id, anchor_message_id, before, after)
            .await?)
    }

    /// Loads image neighbors from the same persisted forwarded transcript.
    pub async fn transcript_image_messages_around(
        &self,
        transcript_id: String,
        anchor_message_id: String,
        before: i64,
        after: i64,
    ) -> crate::ClientResult<Vec<model::MessageListView>> {
        Ok(self
            .inner
            .transcript_image_messages_around(transcript_id, anchor_message_id, before, after)
            .await?)
    }

    /// Reads a nested summary without inventing unavailable original messages.
    pub async fn transcript_summary(
        &self,
        transcript_id: String,
        child_message_id: String,
    ) -> crate::ClientResult<Vec<model::TranscriptSummaryItem>> {
        Ok(self
            .inner
            .transcript_summary(transcript_id, child_message_id)
            .await?)
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

    pub async fn send_audio_receipt(
        &self,
        conversation_id: String,
        path: String,
        duration_millis: i64,
        waveform: Vec<u8>,
        quote_message_id: Option<String>,
    ) -> crate::ClientResult<model::SendReceipt> {
        Ok(self
            .inner
            .send_audio_receipt(
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

    #[allow(clippy::too_many_arguments)]
    pub async fn send_attachment_receipt(
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
    ) -> crate::ClientResult<model::SendReceipt> {
        Ok(self
            .inner
            .send_attachment_receipt(
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

    /// Resolves message permissions in the account business layer.
    pub async fn message_actions(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> crate::ClientResult<Vec<model::MessageActions>> {
        Ok(self
            .inner
            .message_actions(conversation_id, message_ids)
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

    pub async fn clear_unread_mentions(&self, conversation_id: String) -> crate::ClientResult<()> {
        Ok(self.inner.clear_unread_mentions(conversation_id).await?)
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
