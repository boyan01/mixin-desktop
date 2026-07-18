use crate::db::Error;

#[derive(Clone, sqlx::FromRow)]
pub struct SnapshotDetail {
    pub snapshot_id: String,
    pub trace_id: Option<String>,
    pub type_field: String,
    pub asset_id: String,
    pub amount: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub opponent_id: Option<String>,
    pub transaction_hash: Option<String>,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub memo: Option<String>,
    pub confirmations: Option<i32>,
    pub snapshot_hash: Option<String>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
    pub symbol: Option<String>,
    pub asset_name: Option<String>,
    pub asset_icon_url: Option<String>,
    pub chain_icon_url: Option<String>,
    pub asset_confirmations: Option<i64>,
    pub asset_tag: Option<String>,
    pub opponent_name: Option<String>,
    pub price_usd: Option<String>,
    pub fiat_rate: Option<f64>,
}

#[derive(Clone)]
pub struct SnapshotDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl SnapshotDao {
    pub async fn insert(&self, snapshot: &sdk::SnapshotMessage) -> Result<(), Error> {
        let _ = sqlx::query(r#"
INSERT OR REPLACE INTO snapshots (snapshot_id, trace_id, type, asset_id, amount, created_at, opponent_id, transaction_hash, sender, receiver, memo, confirmations, snapshot_hash, opening_balance, closing_balance)
VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
        "#).bind(&snapshot.snapshot_id)
            .bind(&snapshot.trace_id)
            .bind(&snapshot.type_field)
            .bind(&snapshot.asset_id)
            .bind(&snapshot.amount)
            .bind(snapshot.created_at)
            .bind(&snapshot.opponent_id)
            .bind(&snapshot.transaction_hash)
            .bind(&snapshot.sender)
            .bind(&snapshot.receiver)
            .bind(&snapshot.memo)
            .bind(snapshot.confirmations)
            .bind(&snapshot.snapshot_hash)
            .bind(&snapshot.opening_balance)
            .bind(&snapshot.closing_balance)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn find_by_trace_id(
        &self,
        trace_id: &str,
        fiat_currency: &str,
    ) -> Result<Option<SnapshotDetail>, Error> {
        Ok(sqlx::query_as::<_, SnapshotDetail>(
            r#"
SELECT s.snapshot_id, s.trace_id, s.type AS type_field, s.asset_id, s.amount,
       s.created_at, s.opponent_id, s.transaction_hash, s.sender, s.receiver,
       s.memo, s.confirmations, s.snapshot_hash, s.opening_balance, s.closing_balance,
       a.symbol, a.name AS asset_name, a.icon_url AS asset_icon_url,
       c.icon_url AS chain_icon_url, a.confirmations AS asset_confirmations,
       a.tag AS asset_tag,
       u.full_name AS opponent_name, a.price_usd,
       (SELECT rate FROM fiats WHERE code = ?) AS fiat_rate
  FROM snapshots s
  LEFT JOIN assets a ON a.asset_id = s.asset_id
  LEFT JOIN chains c ON c.chain_id = a.chain_id
  LEFT JOIN users u ON u.user_id = s.opponent_id
 WHERE s.trace_id = ?
 LIMIT 1
        "#,
        )
        .bind(fiat_currency)
        .bind(trace_id)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn find_by_id(
        &self,
        snapshot_id: &str,
        fiat_currency: &str,
    ) -> Result<Option<SnapshotDetail>, Error> {
        Ok(sqlx::query_as::<_, SnapshotDetail>(
            r#"
SELECT s.snapshot_id, s.trace_id, s.type AS type_field, s.asset_id, s.amount,
       s.created_at, s.opponent_id, s.transaction_hash, s.sender, s.receiver,
       s.memo, s.confirmations, s.snapshot_hash, s.opening_balance, s.closing_balance,
       a.symbol, a.name AS asset_name, a.icon_url AS asset_icon_url,
       c.icon_url AS chain_icon_url, a.confirmations AS asset_confirmations,
       a.tag AS asset_tag,
       u.full_name AS opponent_name, a.price_usd,
       (SELECT rate FROM fiats WHERE code = ?) AS fiat_rate
  FROM snapshots s
  LEFT JOIN assets a ON a.asset_id = s.asset_id
  LEFT JOIN chains c ON c.chain_id = a.chain_id
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
