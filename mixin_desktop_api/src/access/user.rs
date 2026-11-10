use std::collections::HashMap;

use mixin_desktop_core::runtime::UserAccess as CoreUserAccess;

use crate::{dto as model, ClientError};

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
        Ok(self.inner.search_mao_user(query).await?)
    }

    pub async fn selectable_users(&self) -> Result<Vec<model::UserProfileItem>, ClientError> {
        Ok(self.inner.selectable_users().await?)
    }

    /// Loads a bounded, searchable selection of eligible contacts and bots.
    pub async fn selectable_users_page(
        &self,
        query: String,
        offset: i64,
        limit: i64,
    ) -> crate::ClientResult<Vec<model::UserProfileItem>> {
        Ok(self
            .inner
            .selectable_users_page(query, offset, limit)
            .await?)
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
            .await?)
    }

    /// Searches local users after applying the category, in stable pages.
    pub async fn search_local_users_page(
        &self,
        query: String,
        category: String,
        circle_id: Option<String>,
        offset: i64,
        limit: i64,
    ) -> crate::ClientResult<Vec<model::UserProfileItem>> {
        Ok(self
            .inner
            .search_local_users_page(query, category, circle_id, offset, limit)
            .await?)
    }

    pub async fn invitable_users_page(
        &self,
        conversation_id: String,
        query: String,
        offset: i64,
        limit: i64,
    ) -> crate::ClientResult<Vec<model::UserProfileItem>> {
        Ok(self
            .inner
            .invitable_users_page(conversation_id, query, offset, limit)
            .await?)
    }

    pub async fn blocked_users(&self) -> Result<Vec<model::UserProfileItem>, ClientError> {
        Ok(self.inner.blocked_users().await?)
    }

    pub async fn search_user(&self, query: String) -> Result<model::UserProfileItem, ClientError> {
        Ok(self.inner.search_user(query).await?)
    }

    pub async fn local_shared_apps(
        &self,
        user_id: String,
    ) -> Result<Vec<model::SharedAppItem>, ClientError> {
        Ok(self.inner.local_shared_apps(user_id).await?)
    }

    pub async fn shared_apps(
        &self,
        user_id: String,
    ) -> Result<Vec<model::SharedAppItem>, ClientError> {
        Ok(self.inner.shared_apps(user_id).await?)
    }

    pub async fn bot_creator_id(&self, user_id: String) -> Result<Option<String>, ClientError> {
        Ok(self.inner.bot_creator_id(user_id).await?)
    }

    pub async fn user_profile(
        &self,
        user_id: Option<String>,
        identity_number: Option<String>,
    ) -> Result<Option<model::UserProfileItem>, ClientError> {
        Ok(self.inner.user_profile(user_id, identity_number).await?)
    }

    pub async fn refresh_user_profile(
        &self,
        user_id: String,
    ) -> Result<Option<model::UserProfileItem>, ClientError> {
        Ok(self.inner.refresh_user_profile(user_id).await?)
    }

    pub async fn users_by_identity_numbers(
        &self,
        identity_numbers: Vec<String>,
    ) -> Result<Vec<model::UserProfileItem>, ClientError> {
        Ok(self
            .inner
            .users_by_identity_numbers(identity_numbers)
            .await?)
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

    /// Changes the contact name while preserving the user's relationship.
    pub async fn rename_contact(
        &self,
        user_id: String,
        full_name: String,
    ) -> crate::ClientResult<()> {
        Ok(self.inner.rename_contact(user_id, full_name).await?)
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
