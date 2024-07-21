use serde::{Deserialize, Serialize};

use crate::sdk::client::Client;
use crate::sdk::ApiError;

#[derive(Serialize, Deserialize, Debug)]
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
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
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
    pub relationship: String,
    pub salt_base64: String,
    pub session_id: String,
    pub spend_public_key: String,
    pub tip_counter: i64,
    pub tip_key_base64: String,
    pub transfer_confirmation_threshold: i64,
    pub transfer_notification_threshold: i64,
}

impl Client {
    pub async fn get_me(&self) -> Result<Account, ApiError> {
        let request = self.client.get(format!("{}/me", self.base_url)).build()?;
        Ok(self.request(request).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sdk::*;
    use std::fs;

    #[tokio::test]
    async fn test() {
        let file = fs::read("./keystore.json").expect("no keystore file");
        let keystore: KeyStore = serde_json::from_slice(&file).expect("failed to read keystore");
        let a = Client::new(Credential::KeyStore(keystore));
        let result = a.get_me().await;
        println!("account: {:?}", result);
    }
}
