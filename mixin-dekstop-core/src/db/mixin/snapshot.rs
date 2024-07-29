use crate::db::Error;

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
}
