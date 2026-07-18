use crate::client::ClientRef;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct TokenApi {
    client: Arc<ClientRef>,
}

impl TokenApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        TokenApi { client }
    }

    pub async fn get_asset_by_id(&self, asset_id: &str) -> Result<Token, crate::ApiError> {
        self.client.get(&format!("safe/assets/{asset_id}")).await
    }

    pub async fn get_snapshot_by_id(
        &self,
        snapshot_id: &str,
    ) -> Result<SafeSnapshotShot, crate::ApiError> {
        self.client
            .get(&format!("safe/snapshots/{snapshot_id}"))
            .await
    }

    pub async fn get_inscription_item(
        &self,
        hash: &str,
    ) -> Result<InscriptionItem, crate::ApiError> {
        self.client
            .get(&format!("safe/inscriptions/items/{hash}"))
            .await
    }

    pub async fn get_inscription_collection(
        &self,
        hash: &str,
    ) -> Result<InscriptionCollection, crate::ApiError> {
        self.client
            .get(&format!("safe/inscriptions/collections/{hash}"))
            .await
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Token {
    pub asset_id: String,
    pub kernel_asset_id: String,
    pub symbol: String,
    pub name: String,
    pub icon_url: String,
    pub price_btc: String,
    pub price_usd: String,
    pub chain_id: String,
    pub change_usd: String,
    pub change_btc: String,
    pub confirmations: i64,
    pub asset_key: String,
    pub dust: String,
    pub collection_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
pub struct InscriptionItem {
    pub inscription_hash: String,
    pub collection_hash: String,
    pub sequence: i64,
    pub content_type: String,
    pub content_url: String,
    pub occupied_by: Option<String>,
    pub occupied_at: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
pub struct InscriptionCollection {
    pub collection_hash: String,
    pub supply: String,
    pub unit: String,
    pub symbol: String,
    pub name: String,
    pub icon_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
pub struct SafeSnapshotShot {
    pub snapshot_id: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub type_field: String,
    pub asset_id: String,
    pub amount: String,
    pub user_id: String,
    pub opponent_id: String,
    pub memo: String,
    pub transaction_hash: String,
    pub created_at: DateTime<Utc>,
    pub trace_id: Option<String>,
    pub confirmations: Option<i32>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
    pub withdrawal: Option<SafeWithdrawal>,
    pub deposit: Option<SafeDeposit>,
    #[serde(default)]
    pub deposit_hash: Option<String>,
    pub inscription_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::Type)]
pub struct SafeWithdrawal {
    pub withdrawal_hash: String,
    pub receiver: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::Type)]
pub struct SafeDeposit {
    pub deposit_hash: String,
    #[serde(default)]
    pub sender: String,
}

#[cfg(test)]
mod tests {
    use super::SafeSnapshotShot;

    #[test]
    fn parses_flutter_safe_snapshot_fields() {
        let snapshot: SafeSnapshotShot = serde_json::from_str(
            r#"{
                "snapshot_id":"snapshot",
                "type":"deposit",
                "asset_id":"asset",
                "amount":"1",
                "user_id":"user",
                "opponent_id":"opponent",
                "memo":"",
                "transaction_hash":"transaction",
                "created_at":"2024-01-02T03:04:05Z",
                "deposit_hash":"deposit-transaction",
                "withdrawal":{"withdrawal_hash":"withdrawal-transaction","receiver":"receiver"}
            }"#,
        )
        .unwrap();

        assert_eq!(snapshot.type_field, "deposit");
        assert_eq!(
            snapshot.deposit_hash.as_deref(),
            Some("deposit-transaction")
        );
        assert_eq!(
            snapshot
                .withdrawal
                .as_ref()
                .map(|withdrawal| withdrawal.withdrawal_hash.as_str()),
            Some("withdrawal-transaction")
        );
    }
}
