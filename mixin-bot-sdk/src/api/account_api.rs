use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::api::user_api::{Membership, UserRelationship};
use crate::client::ClientRef;
use crate::{ApiError, SignalKeyCount, SignalKeyRequest, Sticker};

pub struct AccountApi {
    client: Arc<ClientRef>,
}

impl AccountApi {
    pub async fn code(&self, code: &str) -> Result<Value, ApiError> {
        self.client.get(&format!("codes/{code}")).await
    }

    pub async fn get_fiats(&self) -> Result<Vec<Fiat>, ApiError> {
        self.client.get("fiats").await
    }

    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        AccountApi { client }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Fiat {
    pub code: String,
    pub rate: f64,
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
    pub relationship: Option<UserRelationship>,
    pub salt_base64: String,
    pub session_id: String,
    pub spend_public_key: String,
    pub tip_counter: i64,
    pub tip_key_base64: String,
    pub transfer_confirmation_threshold: i64,
    pub transfer_notification_threshold: i64,
    #[serde(default)]
    pub membership: Option<Membership>,
}

impl AccountApi {
    pub async fn get_me(&self) -> Result<Account, ApiError> {
        let account: Account = self.client.get("me").await?;
        Ok(account)
    }

    pub async fn get_signal_key_count(&self) -> Result<SignalKeyCount, ApiError> {
        self.client.get("signal/keys/count").await
    }

    pub async fn push_signal_keys(&self, request: &SignalKeyRequest) -> Result<(), ApiError> {
        let _: Value = self.client.post("signal/keys", request).await?;
        Ok(())
    }

    pub async fn update(&self, request: &AccountUpdateRequest<'_>) -> Result<Account, ApiError> {
        self.client.post("me", request).await
    }

    pub async fn get_sticker_by_id(&self, sticker_id: &str) -> Result<Sticker, ApiError> {
        self.client.get(&format!("stickers/{sticker_id}")).await
    }

    pub async fn get_sticker_albums(&self) -> Result<Vec<StickerAlbum>, ApiError> {
        self.client.get("stickers/albums").await
    }

    pub async fn get_sticker_album(&self, album_id: &str) -> Result<StickerAlbum, ApiError> {
        self.client.get(&format!("albums/{album_id}")).await
    }

    pub async fn get_stickers_by_album_id(&self, album_id: &str) -> Result<Vec<Sticker>, ApiError> {
        self.client
            .get(&format!("stickers/albums/{album_id}"))
            .await
    }

    pub async fn add_sticker(&self, sticker_id: &str) -> Result<Sticker, ApiError> {
        self.client
            .post(
                "stickers/favorite/add",
                &AddStickerRequest {
                    sticker_id: Some(sticker_id),
                    data_base64: None,
                },
            )
            .await
    }

    pub async fn add_sticker_data(&self, data_base64: &str) -> Result<Sticker, ApiError> {
        self.client
            .post(
                "stickers/favorite/add",
                &AddStickerRequest {
                    sticker_id: None,
                    data_base64: Some(data_base64),
                },
            )
            .await
    }

    pub async fn remove_stickers(&self, sticker_ids: &[String]) -> Result<(), ApiError> {
        self.client
            .post("stickers/favorite/remove", sticker_ids)
            .await
    }

    pub async fn logout(&self, session_id: &str) -> Result<(), ApiError> {
        #[derive(Serialize)]
        struct LogoutRequest<'a> {
            session_id: &'a str,
        }

        let _: Value = self
            .client
            .post("logout", &LogoutRequest { session_id })
            .await?;
        Ok(())
    }
}

#[derive(Serialize, Default)]
pub struct AccountUpdateRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biography: Option<&'a str>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StickerAlbum {
    pub album_id: String,
    pub name: String,
    pub icon_url: String,
    pub created_at: DateTime<Utc>,
    pub update_at: DateTime<Utc>,
    pub user_id: String,
    pub category: String,
    pub description: String,
    pub banner: Option<String>,
    #[serde(default)]
    pub is_verified: bool,
}

#[derive(Serialize)]
struct AddStickerRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    sticker_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_base64: Option<&'a str>,
}

#[cfg(test)]
mod test {
    use super::{AccountUpdateRequest, AddStickerRequest};
    use crate::client::tests::new_test_client;

    #[test]
    fn serializes_sticker_id_or_data() {
        assert_eq!(
            serde_json::to_value(AddStickerRequest {
                sticker_id: Some("sticker"),
                data_base64: None,
            })
            .unwrap(),
            serde_json::json!({"sticker_id": "sticker"})
        );
        assert_eq!(
            serde_json::to_value(AddStickerRequest {
                sticker_id: None,
                data_base64: Some("aW1hZ2U="),
            })
            .unwrap(),
            serde_json::json!({"data_base64": "aW1hZ2U="})
        );
    }

    #[test]
    fn serializes_account_profile_update() {
        assert_eq!(
            serde_json::to_value(AccountUpdateRequest {
                full_name: Some("Mixin"),
                biography: Some("Messenger"),
            })
            .unwrap(),
            serde_json::json!({"full_name": "Mixin", "biography": "Messenger"})
        );
    }

    #[tokio::test]
    #[ignore = "requires ../keystore.json and the live Mixin API"]
    async fn test() {
        let client = new_test_client().await;
        let result = client.account_api.get_me().await;
        println!("account: {:?}", result);
    }
}
