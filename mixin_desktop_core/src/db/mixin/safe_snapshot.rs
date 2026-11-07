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
    pub inscription_hash: Option<String>,
    pub inscription_collection_hash: Option<String>,
    pub inscription_sequence: Option<i64>,
    pub inscription_content_type: Option<String>,
    pub inscription_content_url: Option<String>,
    pub inscription_name: Option<String>,
    pub inscription_icon_url: Option<String>,
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
       (SELECT rate FROM fiats WHERE code = ?) AS fiat_rate,
       s.inscription_hash AS inscription_hash,
       inscription.collection_hash AS inscription_collection_hash, inscription.sequence AS inscription_sequence,
       inscription.content_type AS inscription_content_type, inscription.content_url AS inscription_content_url,
       collection.name AS inscription_name, collection.icon_url AS inscription_icon_url
  FROM safe_snapshots s
  LEFT JOIN tokens t ON t.asset_id = s.asset_id
  LEFT JOIN chains c ON c.chain_id = t.chain_id
  LEFT JOIN users u ON u.user_id = s.opponent_id
  LEFT JOIN inscription_items inscription ON inscription.inscription_hash = s.inscription_hash
  LEFT JOIN inscription_collections collection ON collection.collection_hash = inscription.collection_hash
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
    inscription_hash: "inscription_hash",
    inscription_collection_hash: "inscription_collection_hash",
    inscription_sequence: "inscription_sequence",
    inscription_content_type: "inscription_content_type",
    inscription_content_url: "inscription_content_url",
    inscription_name: "inscription_name",
    inscription_icon_url: "inscription_icon_url",
});

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn safe_snapshot_detail_retains_inscription_identity_with_optional_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query("INSERT INTO inscription_collections (collection_hash,supply,unit,symbol,name,icon_url,created_at,updated_at) VALUES ('collection','100','piece','NFT','Collection','https://example.com/icon.png',0,0)")
            .execute(&database.safe_snapshot_dao.0).await.unwrap();
        crate::db::query("INSERT INTO inscription_items (inscription_hash,collection_hash,sequence,content_type,content_url,created_at,updated_at) VALUES ('known','collection',42,'text/plain','https://example.com/item',0,0)")
            .execute(&database.safe_snapshot_dao.0).await.unwrap();
        for hash in ["known", "missing"] {
            crate::db::query("INSERT INTO safe_snapshots (snapshot_id,type,asset_id,amount,user_id,opponent_id,memo,transaction_hash,created_at,inscription_hash) VALUES (?,'transfer','asset','1','me','peer','','tx',1700000000000,?)")
                .bind(hash).bind(hash).execute(&database.safe_snapshot_dao.0).await.unwrap();
            let detail = database
                .safe_snapshot_dao
                .find_by_id(hash, "USD")
                .await
                .unwrap()
                .unwrap();
            let view = crate::runtime::model::SnapshotDetailItem::from_safe_detail(
                detail,
                "Me".into(),
                None,
            );
            assert_eq!(view.inscription_hash.as_deref(), Some(hash));
            assert!(view.is_safe);
            assert_eq!(view.amount, "1");
            if hash == "known" {
                assert_eq!(
                    view.inscription_collection_hash.as_deref(),
                    Some("collection")
                );
                assert_eq!(view.inscription_sequence, Some(42));
                assert_eq!(view.inscription_content_type.as_deref(), Some("text/plain"));
                assert_eq!(
                    view.inscription_content_url.as_deref(),
                    Some("https://example.com/item")
                );
                assert_eq!(view.inscription_name.as_deref(), Some("Collection"));
                assert_eq!(
                    view.inscription_icon_url.as_deref(),
                    Some("https://example.com/icon.png")
                );
            } else {
                assert!(view.inscription_collection_hash.is_none());
                assert!(view.inscription_sequence.is_none());
                assert!(view.inscription_content_url.is_none());
                assert!(view.inscription_name.is_none());
            }
        }
        database.close().await;
    }
}
