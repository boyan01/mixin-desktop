use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::client::ClientRef;
use crate::ApiError;

pub struct AssetApi {
    client: Arc<ClientRef>,
}

impl AssetApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        Self { client }
    }

    pub async fn get_asset_by_id(&self, asset_id: &str) -> Result<Asset, ApiError> {
        self.client.get(&format!("assets/{asset_id}")).await
    }

    pub async fn get_chain(&self, chain_id: &str) -> Result<Chain, ApiError> {
        self.client.get(&format!("network/chains/{chain_id}")).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset_id: String,
    pub symbol: String,
    pub name: String,
    pub icon_url: String,
    pub balance: String,
    pub tag: Option<String>,
    pub price_btc: String,
    pub price_usd: String,
    pub chain_id: String,
    pub change_usd: String,
    pub change_btc: String,
    pub confirmations: i64,
    pub asset_key: Option<String>,
    pub reserve: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub chain_id: String,
    pub name: String,
    pub symbol: String,
    pub icon_url: String,
    pub threshold: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sticker {
    pub sticker_id: String,
    pub album_id: Option<String>,
    pub name: String,
    pub asset_url: String,
    pub asset_type: String,
    pub asset_width: i32,
    pub asset_height: i32,
    pub created_at: DateTime<Utc>,
    pub last_use_at: Option<DateTime<Utc>>,
}
