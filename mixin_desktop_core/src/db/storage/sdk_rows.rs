crate::db::storage::row_mapping!(sdk::InscriptionItem {
    inscription_hash: "inscription_hash",
    collection_hash: "collection_hash",
    sequence: "sequence",
    content_type: "content_type",
    content_url: "content_url",
    occupied_by: "occupied_by",
    occupied_at: "occupied_at",
    created_at: "created_at",
    updated_at: "updated_at",
});

crate::db::storage::row_mapping!(sdk::InscriptionCollection {
    collection_hash: "collection_hash",
    supply: "supply",
    unit: "unit",
    symbol: "symbol",
    name: "name",
    icon_url: "icon_url",
    created_at: "created_at",
    updated_at: "updated_at",
});

crate::db::storage::row_mapping!(sdk::SafeSnapshotShot {
    snapshot_id: "snapshot_id",
    type_field: "type",
    asset_id: "asset_id",
    amount: "amount",
    user_id: "user_id",
    opponent_id: "opponent_id",
    memo: "memo",
    transaction_hash: "transaction_hash",
    created_at: "created_at",
    trace_id: "trace_id",
    confirmations: "confirmations",
    opening_balance: "opening_balance",
    closing_balance: "closing_balance",
    withdrawal: "withdrawal",
    deposit: "deposit",
    deposit_hash: "deposit_hash",
    inscription_hash: "inscription_hash",
});
