use anyhow::Context;

use crate::db::Error;

#[derive(Clone)]
pub struct SafeSnapshotDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl SafeSnapshotDao {
    pub async fn insert(&self, snapshot: &sdk::SafeSnapshotShot) -> Result<(), Error> {
        let _ = sqlx::query(r#"
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
        let _ = sqlx::query(
            "DELETE FROM safe_snapshots WHERE type = 'pending' AND transaction_hash = ?",
        )
        .bind(tx_hash)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
