use mixin_desktop_core::runtime::ConversationAccess as CoreConversationAccess;

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
    pub async fn can_send(&self, conversation_id: String) -> crate::ClientResult<bool> {
        Ok(self.inner.can_send(conversation_id).await?)
    }

    pub async fn resolve_code(&self, code: String) -> Result<model::CodeResult, ClientError> {
        Ok(self.inner.resolve_code(code).await?)
    }

    pub async fn join_group(&self, code: String) -> Result<String, ClientError> {
        Ok(self.inner.join_group(code).await?)
    }

    pub async fn resolve_conversation_link(
        &self,
        conversation_id: String,
        user_id: Option<String>,
    ) -> crate::ClientResult<model::ConversationListData> {
        Ok(self
            .inner
            .resolve_conversation_link(conversation_id, user_id)
            .await?)
    }

    pub async fn open_user_conversation(&self, user_id: String) -> Result<String, ClientError> {
        Ok(self.inner.open_user_conversation(user_id).await?)
    }

    pub async fn conversation_items(
        &self,
    ) -> Result<Vec<model::ConversationListData>, ClientError> {
        Ok(self.inner.conversation_items().await?)
    }

    pub async fn conversation_items_by_ids(
        &self,
        conversation_ids: Vec<String>,
    ) -> Result<Vec<model::ConversationListData>, ClientError> {
        Ok(self
            .inner
            .conversation_items_by_ids(conversation_ids)
            .await?)
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
            .await?)
    }

    pub async fn conversation_position(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        conversation_id: String,
    ) -> crate::ClientResult<Option<i64>> {
        Ok(self
            .inner
            .conversation_position(category, circle_id, keyword, unseen_only, conversation_id)
            .await?)
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
            .await?)
    }

    pub async fn search_bot_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<model::ConversationParticipantItem>, ClientError> {
        Ok(self
            .inner
            .search_bot_group_users(conversation_id, keyword)
            .await?)
    }

    pub async fn search_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<model::ConversationParticipantItem>, ClientError> {
        Ok(self
            .inner
            .search_group_users(conversation_id, keyword)
            .await?)
    }

    pub async fn conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<model::ConversationDetailItem, ClientError> {
        Ok(self.inner.conversation_detail(conversation_id).await?)
    }

    pub async fn local_conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<model::ConversationDetailItem, ClientError> {
        Ok(self
            .inner
            .local_conversation_detail(conversation_id)
            .await?)
    }

    pub async fn groups_in_common(
        &self,
        user_id: String,
    ) -> Result<Vec<model::GroupConversationItem>, ClientError> {
        Ok(self.inner.groups_in_common(user_id).await?)
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
        Ok(self.inner.create_circle(name).await?)
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

    /// Updates the group avatar and refreshes the authoritative conversation.
    pub async fn update_conversation_avatar(
        &self,
        conversation_id: String,
        avatar_base64: String,
    ) -> crate::ClientResult<()> {
        Ok(self
            .inner
            .update_conversation_avatar(conversation_id, avatar_base64)
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
        Ok(self.inner.circles().await?)
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

    /// Applies membership edits. On failure, reload membership to reconcile any
    /// successful edits; they remain durable and have already emitted events.
    pub async fn edit_circle_conversations(
        &self,
        circle_id: String,
        edits: Vec<model::CircleConversationEdit>,
    ) -> crate::ClientResult<()> {
        Ok(self
            .inner
            .edit_circle_conversations(circle_id, edits)
            .await?)
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
