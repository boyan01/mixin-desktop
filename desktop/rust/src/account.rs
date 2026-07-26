use std::sync::Arc;

use mixin_desktop_api::AccountClient;

use crate::{error::SwiftClientError, model::SwiftConversationListItem};

#[derive(uniffi::Object)]
pub struct SwiftAccountHandle {
    client: Arc<AccountClient>,
}

impl SwiftAccountHandle {
    pub(crate) fn new(client: AccountClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftAccountHandle {
    pub async fn conversations(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SwiftConversationListItem>, SwiftClientError> {
        Ok(self
            .client
            .conversations(category, circle_id, keyword, unseen_only, limit, offset)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn shutdown(&self) {
        self.client.shutdown().await;
    }
}
