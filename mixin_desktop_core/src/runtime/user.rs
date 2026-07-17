use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use sdk::RelationshipAction;

use super::{model, AccountState};

pub struct UserAccess {
    state: Arc<AccountState>,
}

impl UserAccess {
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
    pub async fn shared_apps(&self, user_id: String) -> Result<Vec<model::SharedAppItem>> {
        let user_id = user_id.as_str();
        self.ensure_active()?;
        let ids = self
            .client
            .user_api
            .get_favorite_apps(user_id)
            .await?
            .into_iter()
            .map(|favorite| favorite.app_id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .client
            .user_api
            .get_users(&ids)
            .await?
            .into_iter()
            .filter_map(|user| user.app)
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
