use std::sync::Arc;

use crate::client::ClientRef;
use serde::Deserialize;

use crate::{ApiError, SnapshotMessage};

pub struct SnapshotApi {
    client: Arc<ClientRef>,
}

impl SnapshotApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        Self { client }
    }

    pub async fn get_snapshot_by_trace_id(
        &self,
        trace_id: &str,
    ) -> Result<SnapshotMessage, ApiError> {
        self.client
            .get(&format!("snapshots/trace/{trace_id}"))
            .await
    }

    pub async fn get_snapshot_by_id(&self, snapshot_id: &str) -> Result<SnapshotMessage, ApiError> {
        self.client.get(&format!("snapshots/{snapshot_id}")).await
    }

    pub async fn get_ticker(
        &self,
        asset_id: &str,
        offset: Option<&str>,
    ) -> Result<Ticker, ApiError> {
        let mut query = vec![("asset", asset_id)];
        if let Some(offset) = offset {
            query.push(("offset", offset));
        }
        self.client.get_with_query("network/ticker", &query).await
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Ticker {
    pub price_usd: String,
}
