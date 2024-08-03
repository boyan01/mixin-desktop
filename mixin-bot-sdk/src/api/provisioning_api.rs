use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::account_api::Account;
use crate::client::ClientRef;
use crate::ApiError;

pub struct ProvisioningApi {
    client: Arc<ClientRef>,
}

impl ProvisioningApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        ProvisioningApi { client }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvisioningId {
    pub device_id: String,
    pub expired_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provisioning {
    pub device_id: String,
    #[serde(default)]
    pub expired_at: DateTime<Utc>,
    pub secret: String,
    #[serde(default)]
    pub platform: String,
    pub provisioning_code: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvisioningRequest {
    pub user_id: String,
    pub session_id: String,
    pub session_secret: String,
    pub code: String,
    pub platform: String,
    pub platform_version: String,
    pub app_version: String,
    pub purpose: String,
    pub registration_id: u32,
}

impl ProvisioningApi {
    pub async fn get_provisioning_id(&self, device_id: &str) -> Result<ProvisioningId, ApiError> {
        self.client
            .post("provisionings", &json!({"device_id": device_id,}))
            .await
    }

    pub async fn get_provisioning(&self, device_id: &str) -> Result<Provisioning, ApiError> {
        self.client.get(&format!("provisionings/{device_id}")).await
    }

    pub async fn verify_provisioning(
        &self,
        request: &ProvisioningRequest,
    ) -> Result<Account, ApiError> {
        self.client.post("provisionings/verify", request).await
    }
}
