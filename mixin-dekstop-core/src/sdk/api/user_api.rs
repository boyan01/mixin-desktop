use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::sdk::api::account_api::App;
use crate::sdk::client::ClientRef;
use crate::sdk::ApiError;

pub struct UserApi {
    client: Arc<ClientRef>,
}

impl UserApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        UserApi { client }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum UserRelationship {
    Friend,
    Me,
    #[default]
    Stranger,
    Blocked,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub identity_number: String,
    #[serde(default)]
    pub relationship: UserRelationship,
    pub biography: String,
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
    pub phone: Option<String>,
    pub is_verified: bool,
    pub created_at: Option<String>,
    pub mute_until: String,
    pub has_pin: Option<bool>,
    pub app: Option<App>,
    pub is_scam: bool,
    pub code_id: Option<String>,
    pub code_url: Option<String>,
    pub is_deactivated: Option<bool>,
}

impl UserApi {
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User, ApiError> {
        let user: User = self.client.get(&format!("users/{}", user_id)).await?;
        Ok(user)
    }

    pub async fn get_users(&self, ids: Vec<String>) -> Result<Vec<User>, ApiError> {
        let users: Vec<User> = self.client.post("users/fetch", &ids).await?;
        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use crate::sdk::client::tests::new_test_client;

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
            .get_users(vec!["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string()])
            .await;
        println!("result: {:?}", result);
    }
}
