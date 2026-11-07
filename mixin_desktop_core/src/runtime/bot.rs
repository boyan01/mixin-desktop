//! Account-owned bot context and signatures. Signing keys never cross this boundary.
mod signature;
#[cfg(test)]
mod tests;

use std::{ops::Deref, sync::Arc};

use anyhow::{anyhow, Result};
use base64ct::{Base64Url, Base64UrlUnpadded, Encoding};
use chrono::Utc;
use sdk::{generate_conversation_id, ConversationCategory};
use url::Url;
use zeroize::Zeroizing;

use super::{model, AccountState};
use crate::db::mixin::conversation::ConversationStatus;

const MAX_CACHED_BOT_KEYS: usize = 128;

pub struct BotAccess {
    state: Arc<AccountState>,
}

impl BotAccess {
    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }

    pub async fn open_context(
        &self,
        app_id: String,
        source_conversation: Option<String>,
    ) -> Result<model::BotContext> {
        self.resolve_context(app_id, source_conversation)
            .await?
            .ok_or_else(|| anyhow!("bot not found"))
    }

    /// A fresh non-bot sender has no bot context. Transport failures remain errors.
    pub async fn resolve_context(
        &self,
        app_id: String,
        source_conversation: Option<String>,
    ) -> Result<Option<model::BotContext>> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        validate_source(
            &self.database,
            &self.account_id,
            source_conversation.as_deref(),
        )
        .await?;
        let Some(user) = self.fetch_app(&app_id).await? else {
            return Ok(None);
        };
        self.database
            .user_dao
            .insert_sdk_users(vec![user.clone()])
            .await?;
        self.notify_all_conversations_changed();
        let app = user.app.ok_or_else(|| anyhow!("bot not found"))?;
        if web_url(&app.home_uri).is_none() {
            return Err(anyhow!("bot home must be an HTTP or HTTPS URL"));
        }
        Ok(Some(model::BotContext {
            app_id,
            title: app.name,
            home_url: app.home_uri,
            icon_url: app.icon_url,
            immersive: app
                .capabilities
                .iter()
                .any(|capability| capability == "IMMERSIVE"),
            currency: self.profile.borrow().fiat_currency.clone(),
            source_conversation,
        }))
    }

    /// Resolves only server-owned bot metadata; a navigated external page shares as text.
    pub async fn prepare_web_share(
        &self,
        app_id: Option<String>,
        url: String,
        title: String,
        shareable: bool,
    ) -> Result<model::WebShareContent> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if !shareable {
            return Err(anyhow!("app card sharing is disabled"));
        }
        if web_url(&url).is_none() {
            return Err(anyhow!("invalid page URL"));
        }
        let Some(app_id) = app_id else {
            return Ok(model::WebShareContent::Text(url));
        };
        let user = self
            .fetch_app(&app_id)
            .await?
            .ok_or_else(|| anyhow!("bot not found"))?;
        let app = user.app.ok_or_else(|| anyhow!("bot not found"))?;
        Ok(shared_page_card(&app, &url, &title).map_or_else(
            || model::WebShareContent::Text(url),
            |card| model::WebShareContent::AppCard(card.to_string()),
        ))
    }

    /// The presentation adapter must supply the native message frame's URL,
    /// never a URL reported by JavaScript. Rust rechecks bot identity/resources.
    #[allow(clippy::too_many_arguments)]
    pub async fn sign_request(
        &self,
        app_id: String,
        page_url: String,
        method: String,
        path: String,
        body: Option<String>,
        reload_public_key: bool,
    ) -> Result<Option<model::BotSignature>> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if web_url(&page_url).is_none() {
            return Ok(None);
        }
        let Some(user) = self.fetch_app(&app_id).await? else {
            return Ok(None);
        };
        if !signing_allowed(&user, &app_id, &page_url) {
            return Ok(None);
        }
        let Some(public_key) = self.public_key(&app_id, reload_public_key).await? else {
            return Ok(None);
        };
        let mut auth = self
            .auth_service
            .get_auth()
            .ok_or_else(|| anyhow!("account authorization unavailable"))?;
        if auth.user_id != self.account_id {
            return Err(anyhow!("account authorization changed"));
        }
        let private_key = Zeroizing::new(std::mem::take(&mut auth.private_key));
        let timestamp = Utc::now().timestamp();
        let result = signature::sign(
            &self.account_id,
            &private_key,
            &public_key,
            timestamp,
            &method,
            &path,
            body.as_deref(),
        );
        if result.is_err() {
            let mut keys = self.bot_public_keys.lock().await;
            if keys.get(&app_id) == Some(&public_key) {
                keys.remove(&app_id);
            }
        }
        let signature = result?;
        Ok(Some(model::BotSignature {
            timestamp,
            signature,
        }))
    }

    async fn fetch_app(&self, app_id: &str) -> Result<Option<sdk::User>> {
        uuid::Uuid::parse_str(app_id).map_err(|_| anyhow!("invalid bot identifier"))?;
        let users = self.client.user_api.get_users(&[app_id.to_owned()]).await?;
        let user = users
            .into_iter()
            .find(|user| app_identity_matches(user, app_id));
        Ok(user)
    }

    async fn public_key(&self, app_id: &str, reload: bool) -> Result<Option<[u8; 32]>> {
        // Serialize key refreshes so an older fetch cannot replace a forced reload.
        // This private account cache stores public material only and is bounded.
        let mut keys = self.bot_public_keys.lock().await;
        if reload {
            keys.remove(app_id);
        }
        if let Some(key) = keys.get(app_id) {
            return Ok(Some(*key));
        }
        let sessions = self
            .client
            .user_api
            .get_sessions(&[app_id.to_owned()])
            .await?;
        let key = sessions
            .into_iter()
            .filter(|session| session.user_id == app_id)
            .find_map(|session| session.public_key)
            .as_deref()
            .map(decode_public_key)
            .transpose()?;
        if let Some(key) = key {
            if keys.len() >= MAX_CACHED_BOT_KEYS {
                keys.clear();
            }
            keys.insert(app_id.to_owned(), key);
        }
        Ok(key)
    }
}

impl Deref for BotAccess {
    type Target = AccountState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

fn app_identity_matches(user: &sdk::User, app_id: &str) -> bool {
    user.user_id == app_id && user.app.as_ref().is_some_and(|app| app.app_id == app_id)
}

fn decode_public_key(value: &str) -> Result<[u8; 32]> {
    let decoded = Base64UrlUnpadded::decode_vec(value)
        .or_else(|_| Base64Url::decode_vec(value))
        .map_err(|_| anyhow!("invalid bot public key encoding"))?;
    decoded
        .try_into()
        .map_err(|_| anyhow!("invalid bot public key length"))
}

fn web_url(raw: &str) -> Option<Url> {
    Url::parse(raw).ok().filter(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}

async fn validate_source(
    database: &crate::db::mixin::MixinDatabase,
    account_id: &str,
    conversation_id: Option<&str>,
) -> Result<()> {
    let Some(id) = conversation_id else {
        return Ok(());
    };
    let conversation = database
        .conversation_dao
        .find_conversation_by_id(id)
        .await?
        .ok_or_else(|| anyhow!("source conversation not found"))?;
    let permitted = match conversation.category {
        Some(ConversationCategory::Contact) => {
            conversation.owner_id.as_deref().is_some_and(|owner| {
                owner != account_id && generate_conversation_id(account_id, owner).to_string() == id
            })
        }
        Some(ConversationCategory::Group) => {
            conversation.status == ConversationStatus::SUCCESS
                && database
                    .participant_dao
                    .find_participant_by_id(id, account_id)
                    .await?
                    .is_some()
        }
        _ => false,
    };
    if !permitted {
        return Err(anyhow!("source conversation membership required"));
    }
    Ok(())
}

fn signing_allowed(user: &sdk::User, app_id: &str, page_url: &str) -> bool {
    app_identity_matches(user, app_id)
        && user.is_verified
        && user
            .app
            .as_ref()
            .is_some_and(|app| signature::matches_resource(page_url, &app.resource_patterns))
}

fn shared_page_card(app: &sdk::App, url: &str, title: &str) -> Option<serde_json::Value> {
    signature::matches_resource(url, &app.resource_patterns).then(|| {
        serde_json::json!({
            "app_id": app.app_id,
            "icon_url": app.icon_url,
            "title": app.name,
            "description": if title.trim().is_empty() { app.name.as_str() } else { title.trim() },
            "action": url,
            "updated_at": app.updated_at.to_rfc3339(),
            "shareable": true,
            "actions": [],
            "cover_url": "",
            "cover": null,
        })
    })
}
