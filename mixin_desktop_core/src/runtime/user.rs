use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use sdk::RelationshipAction;

use super::{model, AccountState};

pub struct UserAccess {
    state: Arc<AccountState>,
}

impl UserAccess {
    pub async fn search_mao_user(&self, query: String) -> Result<Option<model::UserProfileItem>> {
        let query = query.trim();
        let candidate = query.trim_end_matches('.');
        if candidate.is_empty()
            || candidate.chars().count() > 128
            || candidate
                .chars()
                .all(|character| character.is_ascii_digit())
            || candidate
                .chars()
                .any(|character| character.is_whitespace() || character.is_ascii_uppercase())
        {
            return Ok(None);
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user = self.client.user_api.search(query).await?;
        let profile = self
            .database
            .user_dao
            .insert_sdk_users(vec![user])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("searched MAO user was not persisted"))?;
        self.notify_conversation_changed();
        Ok(Some(profile.into()))
    }

    pub async fn selectable_users(&self) -> Result<Vec<model::UserProfileItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .user_dao
            .selectable_users(&self.account_id)
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
    ) -> Result<Vec<model::UserProfileItem>> {
        let query = query.trim();
        if query.is_empty() || limit <= 0 {
            return Ok(Vec::new());
        }
        let users = self
            .database
            .user_dao
            .fuzzy_search_users(&self.account_id, query, limit.min(100))
            .await?;
        Ok(users
            .into_iter()
            .filter(|user| match category.as_str() {
                "contacts" => {
                    matches!(user.relationship, Some(sdk::UserRelationship::Friend))
                        && user.app_id.is_none()
                }
                "bots" => user.app_id.is_some(),
                "strangers" => {
                    matches!(user.relationship, Some(sdk::UserRelationship::Stranger))
                        && user.app_id.is_none()
                }
                "groups" => false,
                _ => true,
            })
            .map(Into::into)
            .collect())
    }

    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }
}

impl Deref for UserAccess {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl UserAccess {
    pub async fn search_user(&self, query: String) -> Result<model::UserProfileItem> {
        let query = query.trim();
        if query.chars().count() < 4
            || !query.chars().enumerate().all(|(index, character)| {
                character.is_ascii_digit() || (index == 0 && character == '+')
            })
        {
            return Err(anyhow!("user search requires a Mixin ID or phone number"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user = self.client.user_api.search(query).await?;
        let profile = self
            .database
            .user_dao
            .insert_sdk_users(vec![user])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("searched user was not persisted"))?;
        self.notify_conversation_changed();
        Ok(profile.into())
    }

    pub async fn local_shared_apps(&self, user_id: String) -> Result<Vec<model::SharedAppItem>> {
        let user_id = user_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .favorite_app_dao
            .find_apps_by_user_id(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn shared_apps(&self, user_id: String) -> Result<Vec<model::SharedAppItem>> {
        let user_id = user_id.as_str();
        self.ensure_active()?;
        let favorite_apps = self.client.user_api.get_favorite_apps(user_id).await?;
        let ids = favorite_apps
            .iter()
            .map(|favorite| favorite.app_id.clone())
            .collect::<Vec<_>>();
        self.database
            .favorite_app_dao
            .replace_for_user(user_id, &favorite_apps)
            .await?;

        let mut missing_ids = Vec::new();
        for app_id in &ids {
            if self
                .database
                .app_dao
                .find_app_by_id(app_id)
                .await?
                .is_none()
                || !self.database.user_dao.has_user(app_id).await?
            {
                missing_ids.push(app_id.clone());
            }
        }
        if !missing_ids.is_empty() {
            let users = self.client.user_api.get_users(&missing_ids).await?;
            let apps = users
                .iter()
                .filter_map(|user| user.app.clone())
                .collect::<Vec<_>>();
            self.database.app_dao.insert_sdk_apps(&apps).await?;
            self.database.user_dao.insert_sdk_users(users).await?;
        }

        Ok(self
            .database
            .favorite_app_dao
            .find_apps_by_user_id(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn bot_creator_id(&self, user_id: String) -> Result<Option<String>> {
        let user_id = user_id.as_str();
        self.ensure_active()?;
        let Some(app_id) = self
            .database
            .user_dao
            .find_user_by_id(user_id)
            .await?
            .and_then(|user| user.app_id)
        else {
            return Ok(None);
        };
        Ok(self
            .database
            .app_dao
            .find_app_by_id(&app_id)
            .await?
            .map(|app| app.creator_id)
            .filter(|creator_id| !creator_id.is_empty()))
    }

    pub async fn user_profile(
        &self,
        user_id: Option<String>,
        identity_number: Option<String>,
    ) -> Result<Option<model::UserProfileItem>> {
        let user_id = user_id.as_deref();
        let identity_number = identity_number.as_deref();
        if user_id.is_none() && identity_number.is_none() {
            return Err(anyhow!("user id or identity number is required"));
        }
        if let Some(user_id) = user_id {
            if let Some(user) = self.database.user_dao.find_user_by_id(user_id).await? {
                return Ok(Some(user.into()));
            }
        }
        if let Some(identity_number) = identity_number {
            return Ok(self
                .database
                .user_dao
                .find_user_by_identity_number(identity_number)
                .await?
                .map(Into::into));
        }
        Ok(None)
    }

    pub async fn refresh_user_profile(
        &self,
        user_id: String,
    ) -> Result<Option<model::UserProfileItem>> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let users = self.client.user_api.get_users(&[user_id]).await?;
        let profile = self
            .database
            .user_dao
            .insert_sdk_users(users)
            .await?
            .into_iter()
            .next()
            .map(Into::into);
        self.notify_conversation_changed();
        Ok(profile)
    }

    pub async fn users_by_identity_numbers(
        &self,
        identity_numbers: Vec<String>,
    ) -> Result<Vec<model::UserProfileItem>> {
        let identity_numbers = identity_numbers.as_slice();
        Ok(self
            .database
            .user_dao
            .find_users_by_identity_numbers(identity_numbers)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn add_contact(&self, user_id: String, full_name: String) -> Result<()> {
        let user_id = user_id.as_str();
        let full_name = full_name.as_str();
        self.update_relationship(RelationshipAction::Add {
            user_id: user_id.to_string(),
            full_name: full_name.to_string(),
        })
        .await
    }

    pub async fn block_user(&self, user_id: String) -> Result<()> {
        let user_id = user_id.as_str();
        self.update_relationship(RelationshipAction::Block {
            user_id: user_id.to_string(),
        })
        .await
    }

    pub async fn remove_contact(&self, user_id: String) -> Result<()> {
        let user_id = user_id.as_str();
        self.update_relationship(RelationshipAction::Remove {
            user_id: user_id.to_string(),
        })
        .await
    }

    pub async fn unblock_user(&self, user_id: String) -> Result<()> {
        let user_id = user_id.as_str();
        self.update_relationship(RelationshipAction::Unblock {
            user_id: user_id.to_string(),
        })
        .await
    }

    pub async fn report_user(&self, user_id: String) -> Result<()> {
        let user_id = user_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user = self.client.user_api.report_and_block(user_id).await?;
        self.database.user_dao.insert_sdk_users(vec![user]).await?;
        self.notify_conversation_changed();
        Ok(())
    }

    async fn update_relationship(&self, action: RelationshipAction) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user = self.client.user_api.update_relationship(&action).await?;
        self.database.user_dao.insert_sdk_users(vec![user]).await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn bot_home_uri(&self, app_id: String) -> Result<Option<String>> {
        let app_id = app_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .app_dao
            .find_app_by_id(app_id)
            .await?
            .map(|app| app.home_uri)
            .filter(|uri| !uri.trim().is_empty()))
    }
}
