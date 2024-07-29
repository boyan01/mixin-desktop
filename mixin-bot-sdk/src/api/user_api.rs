use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::account_api::App;
use crate::client::ClientRef;
use crate::ApiError;

pub struct UserApi {
    client: Arc<ClientRef>,
}

impl UserApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        UserApi { client }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(sqlx::Type)]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRelationship {
    Friend,
    Me,
    #[default]
    Stranger,
    Blocked,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub user_id: String,
    pub identity_number: String,
    #[serde(default)]
    pub relationship: UserRelationship,
    pub biography: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub phone: String,
    pub is_verified: bool,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub mute_until: DateTime<Utc>,
    #[serde(default)]
    pub has_pin: bool,
    pub app: Option<App>,
    pub is_scam: bool,
    #[serde(default)]
    pub code_id: String,
    #[serde(default)]
    pub code_url: String,
    #[serde(default)]
    pub is_deactivated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "action")]
pub enum RelationshipAction {
    Add { user_id: String, full_name: String },
    Update { user_id: String },
    Remove { user_id: String },
    Block { user_id: String },
    Unblock { user_id: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSession {
    pub user_id: String,
    pub session_id: String,
    pub platform: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteApp {
    pub app_id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
}

impl UserApi {
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User, ApiError> {
        self.client.get(&format!("users/{user_id}")).await
    }

    pub async fn get_users<T: AsRef<str> + Serialize>(
        &self,
        ids: &[T],
    ) -> Result<Vec<User>, ApiError> {
        self.client.post("users/fetch", ids).await
    }

    pub async fn update_relationship(&self, action: &RelationshipAction) -> Result<User, ApiError> {
        self.client.post("relationships", action).await
    }

    pub async fn report_and_block(&self, user_id: &str) -> Result<User, ApiError> {
        self.client
            .post(
                "/reports",
                &RelationshipAction::Block {
                    user_id: user_id.to_string(),
                },
            )
            .await
    }

    pub async fn blocking_users(&self) -> Result<Vec<User>, ApiError> {
        self.client.get("blocking_users").await
    }

    pub async fn get_sessions(&self, ids: &[String]) -> Result<Vec<UserSession>, ApiError> {
        self.client.post("sessions/fetch", &ids).await
    }

    pub async fn get_favorite_apps(&self, user_id: &str) -> Result<Vec<FavoriteApp>, ApiError> {
        self.client
            .get(&format!("users/{user_id}/apps/favorite"))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::tests::new_test_client;

    #[tokio::test]
    async fn test_get_user_by_id() {
        let client = new_test_client().await;
        let result = client
            .user_api
            .get_user_by_id("cfb018b0-eaf7-40ec-9e07-28a5158f1269")
            .await;
        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_get_users() {
        let client = new_test_client().await;
        let result = client
            .user_api
            .get_users(&["cfb018b0-eaf7-40ec-9e07-28a5158f1269"])
            .await;
        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_get_sessions() {
        let client = new_test_client().await;
        let result = client
            .user_api
            .get_sessions(vec!["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string()])
            .await;
        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_get_favorite_apps() {
        let client = new_test_client().await;
        let result = client
            .user_api
            .get_favorite_apps("cfb018b0-eaf7-40ec-9e07-28a5158f1269")
            .await;
        println!("result: {:?}", result);
    }
}
