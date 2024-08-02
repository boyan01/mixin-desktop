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
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
pub struct SafeSnapshotShot {
    pub snapshot_id: String,
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
    pub inscription_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::Type)]
pub struct SafeWithdrawal {
    pub amount: String,
    pub address: String,
    pub transaction_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::Type)]
pub struct SafeDeposit {
    pub amount: String,
    pub transaction_hash: String,
}
