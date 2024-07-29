use std::sync::Arc;

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use simplelog::{Config, TestLogger};

use crate::sdk::api::account_api::App;
use crate::sdk::client::ClientRef;
use crate::sdk::ApiError;

pub struct UserApi {
    client: Arc<ClientRef>,
}

impl UserApi {
    pub fn new(client: Arc<ClientRef>) -> Self {
        UserApi { client }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum UserRelationship {
    Friend,
    Me,
    Stranger,
    Blocked,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    user_id: String,
    identity_number: String,
    relationship: Option<UserRelationship>,
    biography: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
    phone: Option<String>,
    is_verified: bool,
    created_at: Option<String>,
    mute_until: String,
    has_pin: Option<bool>,
    app: Option<App>,
    is_scam: bool,
    code_id: Option<String>,
    code_url: Option<String>,
    is_deactivated: Option<bool>,
}

impl UserApi {
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<(), ApiError> {
        let _ = self
            .client
            .get::<User>(&format!("users/{}", user_id))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sdk::client::tests::new_test_client;

    #[tokio::test]
    async fn test() {
        let client = new_test_client().await;
        let result = client.user_api.get_user_by_id("1").await;
        println!("result: {:?}", result);
    }
}
