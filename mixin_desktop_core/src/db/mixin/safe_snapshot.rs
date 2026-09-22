use anyhow::Context;

use crate::db::Error;

#[derive(Clone)]
pub struct SafeSnapshotDetail {
    pub snapshot_id: String,
    pub trace_id: Option<String>,
    pub type_field: String,
    pub asset_id: String,
    pub amount: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub opponent_id: String,
    pub transaction_hash: String,
    pub memo: String,
    pub confirmations: Option<i32>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
    pub withdrawal: Option<String>,
    pub deposit: Option<String>,
    pub symbol: Option<String>,
    pub asset_name: Option<String>,
    pub asset_icon_url: Option<String>,
    pub chain_icon_url: Option<String>,
    pub asset_confirmations: Option<i64>,
    pub opponent_name: Option<String>,
    pub price_usd: Option<String>,
    pub fiat_rate: Option<f64>,
}

#[derive(Clone)]
pub struct SafeSnapshotDao(pub(crate) crate::db::Pool);

impl SafeSnapshotDao {
    pub async fn insert(&self, snapshot: &sdk::SafeSnapshotShot) -> Result<(), Error> {
        let _ = crate::db::query(r#"
INSERT OR REPLACE INTO safe_snapshots
(snapshot_id, type, asset_id, amount, user_id, opponent_id, memo, transaction_hash, created_at, trace_id, confirmations, opening_balance, closing_balance, withdrawal, deposit, inscription_hash)
VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
        "#).bind(&snapshot.snapshot_id)
            .bind(&snapshot.type_field)
            .bind(&snapshot.asset_id)
            .bind(&snapshot.amount)
            .bind(&snapshot.user_id)
            .bind(&snapshot.opponent_id)
            .bind(&snapshot.memo)
            .bind(&snapshot.transaction_hash)
            .bind(snapshot.created_at)
            .bind(&snapshot.trace_id)
            .bind(snapshot.confirmations)
            .bind(&snapshot.opening_balance)
            .bind(&snapshot.closing_balance)
            .bind(serde_json::to_string(&snapshot.withdrawal).with_context(|| "failed to serialize withdrawal")?)
            .bind(serde_json::to_string(&snapshot.deposit).with_context(|| "failed to serialize deposit")?)
            .bind(&snapshot.inscription_hash)
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn delete_pending_snapshot_by_hash(&self, tx_hash: &str) -> Result<(), Error> {
        let _ = crate::db::query(
            "DELETE FROM safe_snapshots WHERE type = 'pending' AND transaction_hash = ?",
        )
        .bind(tx_hash)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(
        &self,
        snapshot_id: &str,
        fiat_currency: &str,
    ) -> Result<Option<SafeSnapshotDetail>, Error> {
        Ok(crate::db::query_as::<SafeSnapshotDetail>(
            r#"
SELECT s.snapshot_id, s.trace_id, s.type AS type_field, s.asset_id, s.amount,
       s.created_at, s.opponent_id, s.transaction_hash, s.memo, s.confirmations,
       s.opening_balance, s.closing_balance, s.withdrawal, s.deposit,
       t.symbol, t.name AS asset_name, t.icon_url AS asset_icon_url,
       c.icon_url AS chain_icon_url, t.confirmations AS asset_confirmations,
       u.full_name AS opponent_name, t.price_usd,
       (SELECT rate FROM fiats WHERE code = ?) AS fiat_rate
  FROM safe_snapshots s
  LEFT JOIN tokens t ON t.asset_id = s.asset_id
  LEFT JOIN chains c ON c.chain_id = t.chain_id
  LEFT JOIN users u ON u.user_id = s.opponent_id
 WHERE s.snapshot_id = ?
 LIMIT 1
            "#,
        )
        .bind(fiat_currency)
        .bind(snapshot_id)
        .fetch_optional(&self.0)
        .await?)
    }
}

crate::db::storage::row_mapping!(SafeSnapshotDetail {
    snapshot_id: "snapshot_id",
    trace_id: "trace_id",
    type_field: "type_field",
    asset_id: "asset_id",
    amount: "amount",
    created_at: "created_at",
    opponent_id: "opponent_id",
    transaction_hash: "transaction_hash",
    memo: "memo",
    confirmations: "confirmations",
    opening_balance: "opening_balance",
    closing_balance: "closing_balance",
    withdrawal: "withdrawal",
    deposit: "deposit",
    symbol: "symbol",
    asset_name: "asset_name",
    asset_icon_url: "asset_icon_url",
    chain_icon_url: "chain_icon_url",
    asset_confirmations: "asset_confirmations",
    opponent_name: "opponent_name",
    price_usd: "price_usd",
    fiat_rate: "fiat_rate",
});
