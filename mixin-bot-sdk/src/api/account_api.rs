use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::user_api::UserRelationship;
use crate::client::ClientRef;
use crate::ApiError;

pub struct AccountApi {
    client: Arc<ClientRef>,
}

impl AccountApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        AccountApi { client }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct App {
    pub app_id: String,
    pub app_number: String,
    pub app_secret: String,
    pub capabilities: Vec<String>,
    pub category: String,
    pub creator_id: String,
    pub description: String,
    pub has_safe: bool,
    pub home_uri: String,
    pub icon_url: String,
    pub is_verified: bool,
    pub name: String,
    pub redirect_uri: String,
    pub resource_patterns: Vec<String>,
    pub safe_created_at: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub user_id: String,
    pub app: Option<App>,
    pub avatar_url: Option<String>,
    pub biography: String,
    pub code_id: String,
    pub code_url: String,
    pub created_at: String,
    pub device_status: String,
    pub fiat_currency: String,
    pub full_name: Option<String>,
    pub has_emergency_contact: bool,
    pub accept_search_source: String,
    pub accept_conversation_source: String,
    pub receive_message_source: String,
    pub has_pin: bool,
    pub has_safe: bool,
    pub identity_number: String,
    pub is_deactivated: bool,
    pub is_scam: bool,
    pub is_verified: bool,
    pub mute_until: String,
    pub phone: String,
    pub pin_token: String,
    pub pin_token_base64: String,
    #[serde(default)]
    pub relationship: UserRelationship,
    pub salt_base64: String,
    pub session_id: String,
    pub spend_public_key: String,
    pub tip_counter: i64,
    pub tip_key_base64: String,
    pub transfer_confirmation_threshold: i64,
    pub transfer_notification_threshold: i64,
}

impl AccountApi {
    pub async fn get_me(&self) -> Result<Account, ApiError> {
        let account: Account = self.client.get("me").await?;
        Ok(account)
    }
}

#[cfg(test)]
mod test {
    use crate::client::tests::new_test_client;

    #[tokio::test]
    async fn test() {
        let client = new_test_client().await;
        let result = client.account_api.get_me().await;
        println!("account: {:?}", result);
    }
}
