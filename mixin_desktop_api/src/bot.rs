//! Account-scoped bot metadata and signing. Secret key material never crosses this facade.
use crate::{BotContext, BotSignature, ClientResult, WebShareContent};
use mixin_desktop_core::runtime::BotAccess;

pub struct BotClient {
    inner: BotAccess,
}

impl BotClient {
    pub(crate) fn new(inner: BotAccess) -> Self {
        Self { inner }
    }

    pub async fn open_context(
        &self,
        app_id: String,
        source_conversation: Option<String>,
    ) -> ClientResult<BotContext> {
        Ok(self.inner.open_context(app_id, source_conversation).await?)
    }

    pub async fn resolve_context(
        &self,
        sender_id: String,
        source_conversation: Option<String>,
    ) -> ClientResult<Option<BotContext>> {
        Ok(self
            .inner
            .resolve_context(sender_id, source_conversation)
            .await?)
    }

    /// Resolves current bot resources into a shareable card, or a plain URL when unrelated.
    pub async fn prepare_web_share(
        &self,
        app_id: Option<String>,
        url: String,
        title: String,
        shareable: bool,
    ) -> ClientResult<WebShareContent> {
        Ok(self
            .inner
            .prepare_web_share(app_id, url, title, shareable)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn sign_request(
        &self,
        app_id: String,
        page_url: String,
        method: String,
        path: String,
        body: Option<String>,
        reload_public_key: bool,
    ) -> ClientResult<Option<BotSignature>> {
        Ok(self
            .inner
            .sign_request(app_id, page_url, method, path, body, reload_public_key)
            .await?)
    }
}
