//! Local MCP services for a running account and a read-only database sidecar.
//!
//! The services deliberately expose no message-send operation. The database sidecar uses local HTTP
//! and exposes no write operation.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::task::JoinHandle;
use anyhow::{anyhow, Result};
use axum::extract::Request;
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Router;
use futures::{Stream, StreamExt as _};
use rmcp::handler::server::{
    router::tool::ToolRouter,
    wrapper::{Json, Parameters},
};
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use sdk::message_category::MessageCategory as _;
use serde_json::{json, Map, Value};
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::app::SettingDao;
use crate::db::mixin::message_fts::MessageFtsSearch;
use crate::db::MixinDatabase;

use super::message::normalize_message_media_paths;
use super::{model, AccountRuntime};

pub const DEFAULT_PORT: u16 = 55001;
const MCP_ENABLED_KEY: &str = "enable_mcp_server";
const MCP_TOKEN_KEY: &str = "mcp_server_token";
const MCP_DRAFT_TOOLS_ENABLED_KEY: &str = "enable_mcp_draft_tools";
const MCP_CIRCLE_MANAGEMENT_ENABLED_KEY: &str = "enable_mcp_circle_management";
const MCP_SETTING_KEYS: [&str; 4] = [
    MCP_ENABLED_KEY,
    MCP_TOKEN_KEY,
    MCP_DRAFT_TOOLS_ENABLED_KEY,
    MCP_CIRCLE_MANAGEMENT_ENABLED_KEY,
];
type ToolOutput = Map<String, Value>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpSettings {
    pub enabled: bool,
    pub token: String,
    pub draft_tools_enabled: bool,
    pub circle_management_enabled: bool,
}

impl McpSettings {
    pub fn disabled(token: String) -> Self {
        Self {
            enabled: false,
            token,
            draft_tools_enabled: false,
            circle_management_enabled: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpServerStatus {
    pub running: bool,
    pub endpoint: Option<String>,
    pub last_error: Option<String>,
}

fn generate_access_token() -> String {
    Uuid::new_v4().simple().to_string()
}

pub struct McpServer {
    settings: SettingDao,
    state: Mutex<McpServerState>,
    settings_task: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Default)]
struct McpServerState {
    runtime: Option<Arc<AccountRuntime>>,
    running: Option<RunningMcpServer>,
    applied_settings: Option<McpSettings>,
    last_error: Option<String>,
    /// Process-scoped overrides for headless hosts; never persisted, so the
    /// desktop apps cannot enable sending.
    port: Option<u16>,
    message_send: bool,
}

impl McpServerState {
    fn status(&self) -> McpServerStatus {
        let running = self.running.as_ref().filter(|server| {
            !server.cancellation.is_cancelled()
                && self
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.is_running())
        });
        McpServerStatus {
            running: running.is_some(),
            endpoint: running.map(|server| server.endpoint().to_owned()),
            last_error: self.last_error.clone(),
        }
    }

    async fn apply(&mut self, settings: McpSettings, force: bool) -> Result<McpServerStatus> {
        if !force && self.applied_settings.as_ref() == Some(&settings) {
            return Ok(self.status());
        }
        if let Some(server) = self.running.take() {
            server.stop().await;
        }
        self.applied_settings = Some(settings.clone());
        self.last_error = None;
        if settings.enabled {
            let runtime = self
                .runtime
                .clone()
                .ok_or_else(|| anyhow!("MCP requires a signed-in account"))?;
            let port = self.port.unwrap_or(DEFAULT_PORT);
            match RunningMcpServer::start(runtime, settings, port, self.message_send).await {
                Ok(server) => self.running = Some(server),
                Err(error) => {
                    self.last_error = Some(error.to_string());
                    return Ok(self.status());
                }
            }
        }
        Ok(self.status())
    }
}

struct RunningMcpServer {
    cancellation: CancellationToken,
    task: Option<JoinHandle<()>>,
    endpoint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyMcpDatabaseInfo {
    pub database_path: PathBuf,
    pub identity_number: String,
    pub user_id: String,
    pub schema_version: i64,
}

impl McpServer {
    pub async fn new(settings: SettingDao) -> Result<Arc<Self>> {
        Self::ensure_access_token(&settings).await?;
        let server = Arc::new(Self {
            settings,
            state: Mutex::new(McpServerState::default()),
            settings_task: Mutex::new(None),
        });
        server.start_settings_subscription().await;
        Ok(server)
    }

    pub async fn settings(&self) -> Result<McpSettings> {
        Self::load_settings(&self.settings).await
    }

    pub async fn update_settings(&self, settings: McpSettings) -> Result<McpServerStatus> {
        Self::save_settings(&self.settings, &settings).await?;
        self.state.lock().await.apply(settings, false).await
    }

    pub async fn status(&self) -> McpServerStatus {
        self.state.lock().await.status()
    }

    pub async fn start(&self, runtime: Arc<AccountRuntime>) -> Result<McpServerStatus> {
        let settings = self.settings().await?;
        let mut state = self.state.lock().await;
        state.runtime = Some(runtime);
        state.apply(settings, true).await
    }

    /// Sets the port used the next time the server starts.
    pub async fn set_port(&self, port: u16) {
        self.state.lock().await.port = Some(port);
    }

    /// Enables the message_send scope the next time the server starts.
    pub async fn set_message_send(&self, enabled: bool) {
        self.state.lock().await.message_send = enabled;
    }

    pub async fn stop(&self) {
        let mut state = self.state.lock().await;
        state.runtime = None;
        if let Some(server) = state.running.take() {
            server.stop().await;
        }
    }

    #[cfg(test)]
    pub(super) async fn wait_for_account_stop(&self) {
        let task = self
            .state
            .lock()
            .await
            .running
            .as_mut()
            .and_then(|server| server.task.take())
            .expect("test server task");
        task.await
            .expect("account-owned server stopped without panic");
    }

    async fn apply_settings(&self, settings: McpSettings) -> Result<McpServerStatus> {
        self.state.lock().await.apply(settings, false).await
    }

    async fn start_settings_subscription(self: &Arc<Self>) {
        let subscription = Self::subscribe_settings(self.settings.clone());
        let server = Arc::downgrade(self);
        let task = crate::task::spawn(async move {
            futures::pin_mut!(subscription);
            let mut initial = true;
            while let Some(next) = subscription.next().await {
                let settings = match next {
                    Ok(_) if initial => {
                        initial = false;
                        continue;
                    }
                    Ok(settings) => settings,
                    Err(error) => {
                        initial = false;
                        log::warn!("MCP settings subscription failed: {error:?}");
                        continue;
                    }
                };
                let Some(server) = server.upgrade() else {
                    return;
                };
                if let Err(error) = server.apply_settings(settings).await {
                    log::warn!("failed to apply MCP settings: {error:?}");
                }
            }
        });
        if let Some(previous) = self.settings_task.lock().await.replace(task) {
            previous.abort();
        }
    }

    async fn ensure_access_token(settings: &SettingDao) -> Result<()> {
        let token = settings.get(MCP_TOKEN_KEY).await?;
        let token = token
            .as_deref()
            .map(serde_json::from_str::<Option<String>>)
            .transpose()?
            .flatten();
        if token.is_none_or(|token| token.is_empty()) {
            let token = serde_json::to_string(&Some(generate_access_token()))?;
            settings.set(MCP_TOKEN_KEY, Some(&token)).await?;
        }
        Ok(())
    }

    async fn load_settings(settings: &SettingDao) -> Result<McpSettings> {
        Self::ensure_access_token(settings).await?;
        Ok(McpSettings {
            enabled: Self::decode_setting(settings, MCP_ENABLED_KEY, false).await?,
            token: Self::decode_setting(settings, MCP_TOKEN_KEY, None::<String>)
                .await?
                .expect("MCP token must be initialized"),
            draft_tools_enabled: Self::decode_setting(settings, MCP_DRAFT_TOOLS_ENABLED_KEY, false)
                .await?,
            circle_management_enabled: Self::decode_setting(
                settings,
                MCP_CIRCLE_MANAGEMENT_ENABLED_KEY,
                false,
            )
            .await?,
        })
    }

    async fn save_settings(settings: &SettingDao, value: &McpSettings) -> Result<()> {
        let enabled = serde_json::to_string(&value.enabled)?;
        let token = serde_json::to_string(&Some(&value.token))?;
        let draft_tools_enabled = serde_json::to_string(&value.draft_tools_enabled)?;
        let circle_management_enabled = serde_json::to_string(&value.circle_management_enabled)?;
        settings
            .set_many(&[
                (MCP_ENABLED_KEY, Some(&enabled)),
                (MCP_TOKEN_KEY, Some(&token)),
                (MCP_DRAFT_TOOLS_ENABLED_KEY, Some(&draft_tools_enabled)),
                (
                    MCP_CIRCLE_MANAGEMENT_ENABLED_KEY,
                    Some(&circle_management_enabled),
                ),
            ])
            .await
    }

    fn subscribe_settings(
        settings: SettingDao,
    ) -> impl Stream<Item = Result<McpSettings>> + Send + 'static {
        let query_settings = settings.clone();
        settings.subscribe_query(
            MCP_SETTING_KEYS
                .into_iter()
                .map(str::to_owned)
                .collect::<HashSet<_>>(),
            move || {
                let settings = query_settings.clone();
                async move { Self::load_settings(&settings).await }
            },
        )
    }

    async fn decode_setting<T>(settings: &SettingDao, key: &str, default: T) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        match settings.get(key).await? {
            Some(value) => Ok(serde_json::from_str(&value)?),
            None => Ok(default),
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        if let Some(task) = self.settings_task.get_mut().take() {
            task.abort();
        }
    }
}

impl RunningMcpServer {
    async fn start(
        runtime: Arc<AccountRuntime>,
        settings: McpSettings,
        port: u16,
        message_send: bool,
    ) -> Result<Self> {
        if settings.token.trim().is_empty() {
            return Err(anyhow!("MCP access token is unavailable"));
        }
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        let port = listener.local_addr()?.port();
        let cancellation = CancellationToken::new();
        let mut account_shutdown = runtime.subscribe_shutdown();
        let state = ServerState {
            search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            source: McpSource::Runtime(runtime),
            settings,
            message_send,
        };
        let app = http_router(state, &cancellation);
        let stop = cancellation.clone();
        let task = crate::task::spawn(async move {
            if let Err(error) = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    while !*account_shutdown.borrow_and_update() {
                        tokio::select! {
                            biased;
                            _ = stop.cancelled() => break,
                            changed = account_shutdown.changed() => {
                                if changed.is_err() { break; }
                            }
                        }
                    }
                    stop.cancel();
                })
                .await
            {
                log::error!("local MCP server stopped unexpectedly: {error}");
            }
        });
        Ok(Self {
            cancellation,
            task: Some(task),
            endpoint: format!("http://127.0.0.1:{port}/mcp"),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn stop(mut self) {
        self.cancellation.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

pub async fn inspect_read_only_database(
    data_directory: impl AsRef<Path>,
    identity_number: impl Into<String>,
) -> Result<ReadOnlyMcpDatabaseInfo> {
    let source = ReadOnlyMcpSource::open(data_directory.as_ref(), identity_number.into()).await?;
    let info = source.info.clone();
    source.database.close().await;
    Ok(info)
}

pub async fn serve_read_only_http(
    data_directory: impl AsRef<Path>,
    identity_number: impl Into<String>,
    port: u16,
    token: String,
) -> Result<()> {
    if token.trim().is_empty() {
        return Err(anyhow!("MCP access token is unavailable"));
    }
    let source =
        Arc::new(ReadOnlyMcpSource::open(data_directory.as_ref(), identity_number.into()).await?);
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let cancellation = CancellationToken::new();
    let app = http_router(
        ServerState {
            search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            source: McpSource::ReadOnly(source),
            settings: McpSettings::disabled(token),
            message_send: false,
        },
        &cancellation,
    );
    eprintln!(
        "MCP listening on http://{}/mcp (read-only)",
        listener.local_addr()?
    );
    let stop = cancellation.clone();
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            if let Err(error) = tokio::signal::ctrl_c().await {
                log::error!("failed to listen for shutdown: {error}");
            }
            stop.cancel();
        })
        .await;
    cancellation.cancel();
    result?;
    Ok(())
}

fn http_router(state: ServerState, cancellation: &CancellationToken) -> Router {
    let config = StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_cancellation_token(cancellation.child_token());
    let service: StreamableHttpService<McpService, LocalSessionManager> =
        StreamableHttpService::new(
            {
                let state = state.clone();
                move || Ok(McpService::new(state.clone()))
            },
            Default::default(),
            config,
        );
    let events_cancellation = cancellation.child_token();
    Router::new()
        .nest_service("/mcp", service)
        .route(
            "/events",
            axum::routing::get({
                let source = state.source.clone();
                move |headers: HeaderMap, query: axum::extract::Query<EventsQuery>| {
                    events(source, events_cancellation, headers, query.0)
                }
            }),
        )
        .layer(axum::middleware::from_fn_with_state(state, authorize))
}

impl Drop for RunningMcpServer {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[derive(Clone)]
struct ServerState {
    search_slot: Arc<tokio::sync::Semaphore>,
    source: McpSource,
    settings: McpSettings,
    message_send: bool,
}

#[derive(Clone)]
enum McpSource {
    Runtime(Arc<AccountRuntime>),
    ReadOnly(Arc<ReadOnlyMcpSource>),
}

struct ReadOnlyMcpSource {
    database: Arc<MixinDatabase>,
    account_directory: PathBuf,
    info: ReadOnlyMcpDatabaseInfo,
}

impl ReadOnlyMcpSource {
    async fn open(data_directory: &Path, identity_number: String) -> Result<Self> {
        if identity_number.is_empty()
            || !identity_number
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            return Err(anyhow!("identity number must contain only digits"));
        }
        let account_directory = data_directory.join(&identity_number);
        let database_path = account_directory.join("mixin.db");
        let (database, schema_version) =
            MixinDatabase::connect_read_only_at(&database_path).await?;
        let database = Arc::new(database);
        let account = database
            .user_dao
            .find_user_by_identity_number(&identity_number)
            .await?
            .ok_or_else(|| anyhow!("account user is missing from the Mixin database"))?;

        let conversations = database
            .conversation_dao
            .list_items("chats", None, "", false, 1, 0)
            .await?;
        if let Some(conversation) = conversations.first() {
            database
                .participant_dao
                .list_items(&conversation.conversation_id)
                .await?;
            database
                .message_dao
                .list_items(&conversation.conversation_id, None, None, 1)
                .await?;
        }
        database.circle_dao.summaries().await?;

        Ok(Self {
            database,
            account_directory,
            info: ReadOnlyMcpDatabaseInfo {
                database_path,
                identity_number,
                user_id: account.user_id,
                schema_version,
            },
        })
    }

    fn message_view(
        &self,
        item: crate::db::mixin::message::MessageListItem,
    ) -> Result<model::MessageListView> {
        let mut view = item.into();
        normalize_message_media_paths(&mut view, false, &self.account_directory)?;
        Ok(view)
    }
}

impl McpSource {
    fn is_running(&self) -> bool {
        match self {
            Self::Runtime(runtime) => runtime.is_running(),
            Self::ReadOnly(_) => true,
        }
    }

    fn user_id(&self) -> &str {
        match self {
            Self::Runtime(runtime) => runtime.account_id(),
            Self::ReadOnly(source) => &source.info.user_id,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Runtime(_) => "runtime",
            Self::ReadOnly(_) => "database",
        }
    }

    fn is_read_only(&self) -> bool {
        matches!(self, Self::ReadOnly(_))
    }

    fn runtime(&self) -> Result<&Arc<AccountRuntime>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime),
            Self::ReadOnly(_) => Err(anyhow!("MCP database sidecar is read-only")),
        }
    }

    async fn conversations(
        &self,
        circle_id: Option<String>,
        query: String,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<model::ConversationListData>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .conversation_access()
                .conversations("chats".into(), circle_id, query, false, limit, offset)
                .await?),
            Self::ReadOnly(source) => Ok(source
                .database
                .conversation_dao
                .list_items("chats", circle_id.as_deref(), &query, false, limit, offset)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()),
        }
    }

    async fn conversations_by_ids(
        &self,
        conversation_ids: Vec<String>,
    ) -> Result<Vec<model::ConversationListData>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .conversation_access()
                .conversation_items_by_ids(conversation_ids)
                .await?),
            Self::ReadOnly(source) => Ok(source
                .database
                .conversation_dao
                .list_items_by_ids(&conversation_ids)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()),
        }
    }

    async fn messages(
        &self,
        conversation_id: String,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<String>,
        limit: i64,
        range_millis: (Option<i64>, Option<i64>),
    ) -> Result<Vec<model::MessageListView>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .message_access()
                .messages_in_range(
                    conversation_id,
                    before_created_at_micros,
                    before_message_id,
                    limit,
                    range_millis,
                )
                .await?),
            Self::ReadOnly(source) => {
                if before_created_at_micros.is_some() != before_message_id.is_some() {
                    return Err(anyhow!(
                        "message cursor requires both timestamp and message id"
                    ));
                }
                let before_created_at_millis =
                    before_created_at_micros.map(|value| value.div_euclid(1_000));
                source
                    .database
                    .message_dao
                    .list_items_in_range(
                        &conversation_id,
                        before_created_at_millis,
                        before_message_id.as_deref(),
                        limit,
                        range_millis,
                    )
                    .await?
                    .into_iter()
                    .map(|item| source.message_view(item))
                    .collect()
            }
        }
    }

    async fn messages_by_ids(
        &self,
        message_ids: Vec<String>,
    ) -> Result<Vec<model::MessageListView>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .message_access()
                .message_items_by_ids(message_ids)
                .await?),
            Self::ReadOnly(source) => source
                .database
                .message_dao
                .list_items_by_ids(&message_ids)
                .await?
                .into_iter()
                .map(|item| source.message_view(item))
                .collect(),
        }
    }

    async fn search_messages(
        &self,
        input: MessageFtsSearch,
        cancellation: CancellationToken,
    ) -> Result<Vec<model::MessageListView>> {
        let dao = match self {
            Self::Runtime(runtime) => {
                let access = runtime.message_access();
                access.ensure_active()?;
                access.database.message_fts_dao.clone()
            }
            Self::ReadOnly(source) => source.database.message_fts_dao.clone(),
        };
        let matches = dao
            .search_bounded(&input, cancellation, std::time::Duration::from_secs(40))
            .await?;
        let ids = matches.iter().map(|item| item.message_id.clone()).collect();
        let mut items = self
            .messages_by_ids(ids)
            .await?
            .into_iter()
            .map(|item| (item.message_id.clone(), item))
            .collect::<HashMap<_, _>>();
        Ok(matches
            .into_iter()
            .filter_map(|item| items.remove(&item.message_id))
            .collect())
    }

    async fn transcript_messages(
        &self,
        transcript_id: String,
    ) -> Result<Vec<model::MessageListView>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .message_access()
                .transcript_messages(transcript_id)
                .await?),
            Self::ReadOnly(source) => source
                .database
                .transcript_message_dao
                .list_items(&transcript_id)
                .await?
                .into_iter()
                .map(|item| {
                    let mut view = item.into();
                    normalize_message_media_paths(&mut view, true, &source.account_directory)?;
                    Ok(view)
                })
                .collect(),
        }
    }

    async fn messages_around(
        &self,
        conversation_id: String,
        message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .message_access()
                .messages_around(conversation_id, message_id, before, after)
                .await?),
            Self::ReadOnly(source) => source
                .database
                .message_dao
                .list_items_around(&conversation_id, &message_id, before, after)
                .await?
                .into_iter()
                .map(|item| source.message_view(item))
                .collect(),
        }
    }

    async fn participants(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::ConversationParticipantItem>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime
                .conversation_access()
                .conversation_participants(conversation_id)
                .await?),
            Self::ReadOnly(source) => Ok(source
                .database
                .participant_dao
                .list_items(&conversation_id)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()),
        }
    }

    /// Messages from others after `after_row_id` that an agent should see,
    /// with the rowid cursor to resume from.
    async fn agent_event_batch(&self, after_row_id: i64, limit: i64) -> Result<AgentEventBatch> {
        let (database, identity_number) = match self {
            Self::Runtime(runtime) => (&runtime.database, runtime.account().identity_number),
            Self::ReadOnly(source) => (&source.database, source.info.identity_number.clone()),
        };
        let items = database
            .message_dao
            .notification_items_after(self.user_id(), after_row_id, limit)
            .await?;
        let next_row_id = items.last().map_or(after_row_id, |item| item.row_id);
        let has_more = items.len() as i64 == limit;
        let triggered = items
            .into_iter()
            .filter_map(|item| {
                model::agent_trigger(&item, self.user_id(), &identity_number)
                    .map(|trigger| (item, trigger))
            })
            .collect::<Vec<_>>();
        let mut messages = self
            .messages_by_ids(
                triggered
                    .iter()
                    .map(|(item, _)| item.message_id.clone())
                    .collect(),
            )
            .await?
            .into_iter()
            .map(|message| (message.message_id.clone(), message))
            .collect::<HashMap<_, _>>();
        let events = triggered
            .into_iter()
            .filter_map(|(item, trigger)| {
                let message = messages.remove(&item.message_id)?;
                Some(AgentEvent {
                    cursor: item.row_id,
                    data: json!({
                        "cursor": item.row_id.to_string(),
                        "trigger": trigger,
                        "conversation": {
                            "conversation_id": item.conversation_id,
                            "category": item.conversation_category,
                            "name": item.conversation_name,
                        },
                        "message": message_json(message),
                    }),
                })
            })
            .collect();
        Ok(AgentEventBatch {
            events,
            next_row_id,
            has_more,
        })
    }

    async fn circles(&self) -> Result<Vec<model::CircleItem>> {
        match self {
            Self::Runtime(runtime) => Ok(runtime.conversation_access().circles().await?),
            Self::ReadOnly(source) => Ok(source
                .database
                .circle_dao
                .summaries()
                .await?
                .into_iter()
                .map(Into::into)
                .collect()),
        }
    }
}

async fn authorize(
    state: axum::extract::State<ServerState>,
    request: Request,
    next: Next,
) -> Response {
    if !authorized(request.headers(), &state.settings.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    if !state.source.is_running() {
        return (StatusCode::SERVICE_UNAVAILABLE, "account runtime stopped").into_response();
    }
    next.run(request).await
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Some(value) = value.strip_prefix("Bearer ") else {
        return false;
    };
    value.as_bytes().ct_eq(token.as_bytes()).into()
}

struct AgentEvent {
    cursor: i64,
    data: Value,
}

struct AgentEventBatch {
    events: Vec<AgentEvent>,
    next_row_id: i64,
    has_more: bool,
}

#[derive(serde::Deserialize)]
struct EventsQuery {
    cursor: Option<String>,
}

/// Server-sent events for messages an agent should see. Each event id is a
/// cursor; reconnecting with `?cursor=` or `Last-Event-ID` resumes after it,
/// and omitting both starts from new messages.
async fn events(
    source: McpSource,
    cancellation: CancellationToken,
    headers: HeaderMap,
    query: EventsQuery,
) -> Response {
    let McpSource::Runtime(runtime) = &source else {
        return (
            StatusCode::NOT_FOUND,
            "events require a signed-in account runtime",
        )
            .into_response();
    };
    let cursor = query.cursor.or_else(|| {
        headers
            .get("last-event-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    });
    let cursor = match cursor.map(|value| value.parse::<i64>()) {
        Some(Ok(value)) if value >= 0 => Some(value),
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                "cursor must be a non-negative integer",
            )
                .into_response()
        }
        None => None,
    };
    let mut changes = runtime.subscribe_notification_changes();
    let mut shutdown = runtime.subscribe_shutdown();
    let mut row_id = match cursor {
        Some(cursor) => cursor,
        None => match runtime.latest_notification_row_id().await {
            Ok(row_id) => row_id,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
        },
    };
    let stream = async_stream::stream! {
        yield Ok::<_, std::convert::Infallible>(
            axum::response::sse::Event::default()
                .event("ready")
                .data(json!({"cursor": row_id.to_string()}).to_string()),
        );
        loop {
            loop {
                let batch = match source.agent_event_batch(row_id, 200).await {
                    Ok(batch) => batch,
                    Err(error) => {
                        log::error!("agent event batch failed: {error}");
                        yield Ok(axum::response::sse::Event::default()
                            .event("error")
                            .data(json!({"message": error.to_string()}).to_string()));
                        return;
                    }
                };
                row_id = batch.next_row_id;
                for event in batch.events {
                    yield Ok(axum::response::sse::Event::default()
                        .event("message")
                        .id(event.cursor.to_string())
                        .data(event.data.to_string()));
                }
                if !batch.has_more {
                    break;
                }
            }
            tokio::select! {
                changed = changes.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = cancellation.cancelled() => break,
            }
        }
    };
    axum::response::sse::Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

#[derive(Clone)]
struct McpService {
    state: ServerState,
    tool_router: ToolRouter<Self>,
}

impl McpService {
    fn new(state: ServerState) -> Self {
        let mut tool_router = Self::tool_router();
        if !state.settings.draft_tools_enabled {
            for name in [
                "mixin_get_conversation_draft",
                "mixin_set_conversation_draft",
                "mixin_clear_conversation_draft",
            ] {
                tool_router.disable_route(name);
            }
        }
        if !state.settings.circle_management_enabled {
            for name in [
                "mixin_create_circle",
                "mixin_rename_circle",
                "mixin_delete_circle",
            ] {
                tool_router.disable_route(name);
            }
        }
        if !state.message_send {
            for name in SEND_TOOLS {
                tool_router.disable_route(name);
            }
        }
        Self { state, tool_router }
    }

    /// Checks the send scope and that the conversation exists locally.
    async fn send_target(&self, conversation_id: String) -> Result<(String, &Arc<AccountRuntime>)> {
        ensure_send(&self.state)?;
        let id = require_non_empty(conversation_id, "conversation_id")?;
        let runtime = self.state.source.runtime()?;
        if self
            .state
            .source
            .conversations_by_ids(vec![id.clone()])
            .await?
            .is_empty()
        {
            return Err(anyhow!("conversation not found"));
        }
        Ok((id, runtime))
    }
}

#[tool_router]
impl McpService {
    #[tool(description = "Get login state, permission scopes, and enabled MCP tools.")]
    async fn mixin_get_app_status(&self) -> Json<ToolOutput> {
        Json(object(json!({
            "logged_in": self.state.source.is_running(),
            "user_id": self.state.source.user_id(),
            "source": self.state.source.kind(),
            "read_only": self.state.source.is_read_only(),
            "permission_scopes": {
                "read": true,
                "draft_write": self.state.settings.draft_tools_enabled,
                "circle_management": self.state.settings.circle_management_enabled,
                "message_send": self.state.message_send,
                "account_write": false,
            },
            "capabilities": enabled_capabilities(&self.state),
        })))
    }

    #[tool(description = "List or search local conversations.")]
    async fn mixin_list_conversations(
        &self,
        Parameters(input): Parameters<ListConversationsInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let limit = input.limit.unwrap_or(30).clamp(1, 200);
        let offset = input.offset.unwrap_or(0).max(0);
        let result = self
            .state
            .source
            .conversations(
                input.circle_id,
                input.query.unwrap_or_default(),
                limit + 1,
                offset,
            )
            .await
            .map(|items| {
                let has_more = items.len() as i64 > limit;
                let items = items
                    .into_iter()
                    .take(limit as usize)
                    .map(conversation_json)
                    .collect::<Vec<_>>();
                json!({
                    "conversations": items,
                    "pagination": {
                        "limit": limit,
                        "offset": offset,
                        "has_more": has_more,
                        "next_offset": has_more.then_some(offset + limit),
                    }
                })
            });
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Get one conversation by conversation_id.")]
    async fn mixin_get_conversation(
        &self,
        Parameters(input): Parameters<ConversationIdInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let id = require_non_empty(input.conversation_id, "conversation_id")?;
            let conversation = self
                .state
                .source
                .conversations_by_ids(vec![id])
                .await?
                .into_iter()
                .next()
                .map(conversation_json)
                .ok_or_else(|| anyhow!("conversation not found"))?;
            Ok(json!({"conversation": conversation}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Resolve a conversation from id, Mixin URI, or fuzzy query.")]
    async fn mixin_resolve_conversation(
        &self,
        Parameters(input): Parameters<ResolveConversationInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let item = if let Some(id) = input.conversation_id {
                let id = require_non_empty(id, "conversation_id")?;
                self.state
                    .source
                    .conversations_by_ids(vec![id])
                    .await?
                    .into_iter()
                    .next()
            } else if let Some(uri) = input.uri {
                let id = uri
                    .strip_prefix("mixin://conversations/")
                    .ok_or_else(|| anyhow!("unsupported conversation URI"))?;
                let id = require_non_empty(id.to_owned(), "conversation_id")?;
                self.state
                    .source
                    .conversations_by_ids(vec![id])
                    .await?
                    .into_iter()
                    .next()
            } else {
                let query = require_non_empty(input.query.unwrap_or_default(), "query")?;
                self.state
                    .source
                    .conversations(None, query, 1, 0)
                    .await?
                    .into_iter()
                    .next()
            };
            let item = item.ok_or_else(|| anyhow!("conversation not found"))?;
            Ok(json!({"conversation": conversation_json(item)}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "List local messages newest first. Use start/end_created_at_micros for a time window. Continue with both fields from pagination.next_cursor, keeping the same time window."
    )]
    async fn mixin_list_messages(
        &self,
        Parameters(input): Parameters<ListMessagesInput>,
    ) -> Result<Json<ToolOutput>, String> {
        if input
            .start_created_at_micros
            .zip(input.end_created_at_micros)
            .is_some_and(|(start, end)| start >= end)
        {
            return Err(
                "start_created_at_micros must be earlier than end_created_at_micros".into(),
            );
        }
        let limit = input.limit.unwrap_or(100).clamp(1, 200);
        // Stored timestamps are milliseconds: ceil both bounds to preserve [start, end).
        let to_millis =
            |value: i64| value.div_euclid(1000) + i64::from(value.rem_euclid(1000) != 0);
        let result = match require_non_empty(input.conversation_id, "conversation_id") {
            Ok(conversation_id) => self
                .state
                .source
                .messages(
                    conversation_id,
                    input.before_created_at_micros,
                    input.before_message_id,
                    limit + 1,
                    (
                        input.start_created_at_micros.map(to_millis),
                        input.end_created_at_micros.map(to_millis),
                    ),
                )
                .await
                .map(|mut messages| {
                    let has_more = messages.len() > limit as usize;
                    messages.truncate(limit as usize);
                    let next_cursor = messages.last().filter(|_| has_more).map(|message| {
                        json!({
                            "before_created_at_micros": message.created_at_micros,
                            "before_message_id": message.message_id,
                        })
                    });
                    json!({"messages": messages.into_iter().map(message_json).collect::<Vec<_>>(),
                        "pagination": {"has_more": has_more, "next_cursor": next_cursor}})
                }),
            Err(error) => Err(error),
        };
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Search indexed message content across all or selected conversations, newest first. One search may run at a time, with a 40-second query deadline. On search_timeout narrow the time range; on search_busy wait for the active search. Use the returned cursor for older matches."
    )]
    async fn mixin_search_messages(
        &self,
        Parameters(input): Parameters<SearchMessagesInput>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<Json<ToolOutput>, String> {
        self.search_messages_input(input, context.ct).await
    }

    async fn search_messages_input(
        &self,
        input: SearchMessagesInput,
        cancellation: CancellationToken,
    ) -> Result<Json<ToolOutput>, String> {
        let _permit = self.state.search_slot.try_acquire().map_err(|_| {
            log::info!("MCP search outcome=busy");
            "search_busy: another search is running; retry after it completes".to_owned()
        })?;
        let result: Result<Value> = async {
            let query = require_non_empty(input.query, "query")?;
            let conversation_ids =
                validate_filter(input.conversation_ids, "conversation_ids", 100)?;
            let sender_ids = validate_filter(input.sender_ids, "sender_ids", 100)?;
            let categories = validate_filter(input.categories, "categories", 50)?;
            if input
                .start_created_at_micros
                .zip(input.end_created_at_micros)
                .is_some_and(|(start, end)| start >= end)
            {
                return Err(anyhow!(
                    "start_created_at_micros must be earlier than end_created_at_micros"
                ));
            }
            let limit = input.limit.unwrap_or(50).clamp(1, 100) as usize;
            let mut messages = self
                .state
                .source
                .search_messages(
                    MessageFtsSearch {
                        query,
                        conversation_ids,
                        sender_ids,
                        categories,
                        start_created_at_millis: input
                            .start_created_at_micros
                            .map(|value| value.div_euclid(1_000)),
                        end_created_at_millis: input
                            .end_created_at_micros
                            .map(|value| value.div_euclid(1_000)),
                        cursor_message_id: input.cursor,
                        limit: (limit + 1) as u32,
                        ..Default::default()
                    },
                    cancellation,
                )
                .await?;
            let has_more = messages.len() > limit;
            messages.truncate(limit);
            let next_cursor = has_more
                .then(|| messages.last().map(|message| message.message_id.clone()))
                .flatten();
            let conversation_ids = messages
                .iter()
                .map(|message| message.conversation_id.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let conversations = self
                .state
                .source
                .conversations_by_ids(conversation_ids)
                .await?
                .into_iter()
                .map(|conversation| {
                    (
                        conversation.conversation_id.clone(),
                        conversation_json(conversation),
                    )
                })
                .collect::<HashMap<_, _>>();
            let messages = messages
                .into_iter()
                .map(|message| {
                    let conversation = conversations.get(&message.conversation_id).cloned();
                    search_message_json(message, conversation)
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "messages": messages,
                "pagination": {
                    "limit": limit,
                    "has_more": has_more,
                    "next_cursor": next_cursor,
                }
            }))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Get one local message by message_id. A transcript (forwarded chat history) also returns its child messages in transcript_messages, with local media_url and media_status."
    )]
    async fn mixin_get_message(
        &self,
        Parameters(input): Parameters<MessageIdInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let id = require_non_empty(input.message_id, "message_id")?;
            let message = self
                .state
                .source
                .messages_by_ids(vec![id])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("message not found"))?;
            if !message.category.is_transcript() {
                return Ok(json!({"message": message_json(message)}));
            }
            let transcript_messages = self
                .state
                .source
                .transcript_messages(message.message_id.clone())
                .await?
                .into_iter()
                .map(|item| {
                    let media_status = item.media_status.clone();
                    let mut value = message_json(item);
                    value["media_status"] = json!(media_status);
                    value
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "message": message_json(message),
                "transcript_messages": transcript_messages,
            }))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Read a downloaded image attachment as image content. For an image inside a transcript, pass the transcript message_id as transcript_id."
    )]
    async fn mixin_read_attachment(
        &self,
        Parameters(input): Parameters<ReadAttachmentInput>,
    ) -> Result<CallToolResult, String> {
        let result: Result<CallToolResult> = async {
            let id = require_non_empty(input.message_id, "message_id")?;
            let message = match input.transcript_id {
                Some(transcript_id) => self
                    .state
                    .source
                    .transcript_messages(require_non_empty(transcript_id, "transcript_id")?)
                    .await?
                    .into_iter()
                    .find(|message| message.message_id == id),
                None => self
                    .state
                    .source
                    .messages_by_ids(vec![id])
                    .await?
                    .into_iter()
                    .next(),
            }
            .ok_or_else(|| anyhow!("message not found"))?;
            if !message.category.is_image() {
                return Err(anyhow!("message is not an image"));
            }
            if message.media_status != "DONE" {
                return Err(anyhow!(
                    "attachment is not downloaded: media_status={}",
                    message.media_status
                ));
            }
            let path = message
                .media_url
                .filter(|path| !path.trim().is_empty())
                .ok_or_else(|| anyhow!("attachment has no local file"))?;
            let (data, mime_type) = read_attachment_image(Path::new(&path)).await?;
            Ok(CallToolResult::success(vec![ContentBlock::image(
                data, mime_type,
            )]))
        }
        .await;
        result.map_err(|error| error.to_string())
    }

    #[tool(description = "Read messages before and after one message.")]
    async fn mixin_get_message_context(
        &self,
        Parameters(input): Parameters<MessageContextInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let id = require_non_empty(input.message_id, "message_id")?;
            let limit = input.limit.unwrap_or(10).clamp(1, 50);
            let target = self
                .state
                .source
                .messages_by_ids(vec![id.clone()])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("message not found"))?;
            let messages = self
                .state
                .source
                .messages_around(target.conversation_id.clone(), id, limit, limit)
                .await?;
            let target_index = messages
                .iter()
                .position(|message| message.message_id == target.message_id)
                .ok_or_else(|| anyhow!("message context does not contain target"))?;
            let messages = messages.into_iter().map(message_json).collect::<Vec<_>>();
            Ok(json!({
                "before": &messages[..target_index],
                "message": message_json(target),
                "after": &messages[target_index + 1..],
            }))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "List or search conversation participants.")]
    async fn mixin_list_conversation_participants(
        &self,
        Parameters(input): Parameters<ListParticipantsInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let query = input.query.unwrap_or_default().to_lowercase();
        let limit = input.limit.unwrap_or(50).clamp(1, 200) as usize;
        let result = match require_non_empty(input.conversation_id, "conversation_id") {
            Ok(conversation_id) => {
                self.state
                    .source
                    .participants(conversation_id)
                    .await
                    .map(|participants| {
                        let participants = participants
                            .into_iter()
                            .filter(|participant| {
                                query.is_empty()
                                    || participant.user_id.to_lowercase().contains(&query)
                                    || participant.identity_number.contains(&query)
                                    || participant.full_name.to_lowercase().contains(&query)
                            })
                            .take(limit)
                            .map(participant_json)
                            .collect::<Vec<_>>();
                        json!({"participants": participants})
                    })
            }
            Err(error) => Err(error),
        };
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Resolve participants by id, identity number, or name.")]
    async fn mixin_resolve_conversation_participant(
        &self,
        Parameters(input): Parameters<ResolveParticipantInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let conversation_id = require_non_empty(input.conversation_id, "conversation_id")?;
            let query = require_non_empty(input.query, "query")?.to_lowercase();
            let limit = input.limit.unwrap_or(5).clamp(1, 200) as usize;
            let participants = self
                .state
                .source
                .participants(conversation_id)
                .await?
                .into_iter()
                .filter(|participant| {
                    participant.user_id.to_lowercase().contains(&query)
                        || participant.identity_number.contains(&query)
                        || participant.full_name.to_lowercase().contains(&query)
                })
                .take(limit)
                .map(participant_json)
                .collect::<Vec<_>>();
            Ok(json!({"participants": participants}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "List local circles and their conversation counts.")]
    async fn mixin_list_circles(&self) -> Result<Json<ToolOutput>, String> {
        let result = self.state.source.circles().await.map(
            |circles| json!({"circles": circles.into_iter().map(circle_json).collect::<Vec<_>>()}),
        );
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Get the current draft text. Does not send.")]
    async fn mixin_get_conversation_draft(
        &self,
        Parameters(input): Parameters<ConversationIdInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            ensure_drafts(&self.state)?;
            let id = require_non_empty(input.conversation_id, "conversation_id")?;
            let item = self
                .state
                .source
                .conversations_by_ids(vec![id.clone()])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("conversation not found"))?;
            Ok(json!({"conversation_id": id, "draft": item.draft}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Replace a draft. Does not send.")]
    async fn mixin_set_conversation_draft(
        &self,
        Parameters(input): Parameters<SetDraftInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            ensure_drafts(&self.state)?;
            let id = require_non_empty(input.conversation_id, "conversation_id")?;
            let text = require_non_empty(input.text, "text")?;
            let runtime = self.state.source.runtime()?;
            runtime
                .database
                .conversation_dao
                .update_draft(&id, &text)
                .await?;
            runtime.notify_conversation_changed(&id);
            Ok(json!({"updated": true, "conversation_id": id}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Clear a draft. Does not send.")]
    async fn mixin_clear_conversation_draft(
        &self,
        Parameters(input): Parameters<ConversationIdInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            ensure_drafts(&self.state)?;
            let id = require_non_empty(input.conversation_id, "conversation_id")?;
            let runtime = self.state.source.runtime()?;
            runtime
                .database
                .conversation_dao
                .update_draft(&id, "")
                .await?;
            runtime.notify_conversation_changed(&id);
            Ok(json!({"updated": true, "conversation_id": id}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Send a text message as this account, optionally quoting a message. Set post to render Markdown. Requires the message_send scope."
    )]
    async fn mixin_send_text(
        &self,
        Parameters(input): Parameters<SendTextInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let (id, runtime) = self.send_target(input.conversation_id).await?;
            let text = require_non_empty(input.text, "text")?;
            let message_id = runtime
                .message_access()
                .send_text_message(
                    id.clone(),
                    text,
                    input.quote_message_id,
                    input.post.unwrap_or(false),
                    false,
                )
                .await?;
            Ok(sent_json(id, message_id))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Send a PNG, JPEG, GIF, or WebP image as this account from base64 data (a data: URL prefix is allowed), optionally with a caption and quote. Requires the message_send scope."
    )]
    async fn mixin_send_image(
        &self,
        Parameters(input): Parameters<SendImageInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let (id, runtime) = self.send_target(input.conversation_id).await?;
            let image = decode_image(&input.data_base64)?;
            let temporary = std::env::temp_dir().join(format!(
                "mixin-mcp-image-{}.{}",
                Uuid::new_v4(),
                image.extension
            ));
            crate::fs::write(&temporary, image.bytes).await?;
            // The attachment is copied into the account media directory
            // before upload, so the temporary file is only needed until then.
            let result = runtime
                .message_access()
                .send_attachment(
                    id.clone(),
                    temporary.to_string_lossy().into_owned(),
                    "IMAGE".into(),
                    image.mime_type.into(),
                    None,
                    Some(image.width),
                    Some(image.height),
                    None,
                    None,
                    input.caption,
                    input.quote_message_id,
                    false,
                )
                .await;
            let _ = crate::fs::remove_file(&temporary).await;
            Ok(sent_json(id, result?))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Send a sticker by sticker_id, as seen on received sticker messages. Requires the message_send scope."
    )]
    async fn mixin_send_sticker(
        &self,
        Parameters(input): Parameters<SendStickerInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let (id, runtime) = self.send_target(input.conversation_id).await?;
            let sticker_id = require_non_empty(input.sticker_id, "sticker_id")?;
            // Same as the UI targets: fetch stickers that are not stored locally yet.
            runtime
                .sticker_access()
                .sticker_detail(sticker_id.clone())
                .await?;
            let message_id = runtime
                .message_access()
                .send_sticker(id.clone(), sticker_id)
                .await?;
            Ok(sent_json(id, message_id))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Share a user's contact card by user_id, optionally quoting a message. Requires the message_send scope."
    )]
    async fn mixin_send_contact(
        &self,
        Parameters(input): Parameters<SendContactInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let (id, runtime) = self.send_target(input.conversation_id).await?;
            let user_id = require_non_empty(input.user_id, "user_id")?;
            let message_id = runtime
                .message_access()
                .send_contact(id.clone(), user_id, input.quote_message_id, false)
                .await?;
            Ok(sent_json(id, message_id))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(
        description = "Send an app card on behalf of a Mixin bot. card needs app_id (the bot's app ID), title, and description. Add buttons with actions: [{label, color, action: https URL}], which also requires icon_url; non-shareable button URLs drop all buttons. Requires the message_send scope."
    )]
    async fn mixin_send_app_card(
        &self,
        Parameters(input): Parameters<SendAppCardInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let (id, runtime) = self.send_target(input.conversation_id).await?;
            // Core only checks the fields it renders; the server rejects these
            // cases with an opaque invalid-field error.
            let has_text = |field: &str| {
                input
                    .card
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
            };
            if !has_text("app_id") {
                return Err(anyhow!("card.app_id is required"));
            }
            if input.card.contains_key("actions") && !has_text("icon_url") {
                return Err(anyhow!("card.icon_url is required with card.actions"));
            }
            let message_id = runtime
                .message_access()
                .send_app_card(id.clone(), serde_json::to_string(&input.card)?)
                .await?;
            Ok(sent_json(id, message_id))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Create a circle.")]
    async fn mixin_create_circle(
        &self,
        Parameters(input): Parameters<CreateCircleInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            ensure_circles(&self.state)?;
            let circle = self
                .state
                .source
                .runtime()?
                .conversation_access()
                .create_circle(input.name)
                .await?;
            Ok(json!({"created": true, "circle": circle_json(circle)}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Rename a circle.")]
    async fn mixin_rename_circle(
        &self,
        Parameters(input): Parameters<RenameCircleInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            ensure_circles(&self.state)?;
            let id = require_non_empty(input.circle_id, "circle_id")?;
            let name = require_non_empty(input.name, "name")?;
            self.state
                .source
                .runtime()?
                .conversation_access()
                .update_circle(id.clone(), name.clone())
                .await?;
            Ok(json!({"updated": true, "circle_id": id, "name": name}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Delete a circle.")]
    async fn mixin_delete_circle(
        &self,
        Parameters(input): Parameters<CircleIdInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            ensure_circles(&self.state)?;
            let id = require_non_empty(input.circle_id, "circle_id")?;
            self.state
                .source
                .runtime()?
                .conversation_access()
                .delete_circle(id.clone())
                .await?;
            Ok(json!({"deleted": true, "circle_id": id}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpService {
    fn get_info(&self) -> ServerInfo {
        let instructions = if self.state.message_send {
            "Local Mixin account access. mixin_send_text sends real messages as this account."
        } else {
            "Local Mixin desktop access. It never sends messages."
        };
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(instructions)
    }
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ConversationIdInput {
    conversation_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct CircleIdInput {
    circle_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct MessageIdInput {
    message_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ReadAttachmentInput {
    message_id: String,
    /// The transcript message_id when message_id is a message inside a transcript.
    transcript_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ListConversationsInput {
    query: Option<String>,
    circle_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ResolveConversationInput {
    conversation_id: Option<String>,
    uri: Option<String>,
    query: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ListMessagesInput {
    conversation_id: String,
    /// Inclusive lower timestamp bound, independent of the pagination cursor.
    start_created_at_micros: Option<i64>,
    /// Exclusive upper timestamp bound, independent of the pagination cursor.
    end_created_at_micros: Option<i64>,
    before_created_at_micros: Option<i64>,
    before_message_id: Option<String>,
    limit: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SearchMessagesInput {
    /// Required full-text query. Multiple terms must all match.
    query: String,
    /// Restrict results to any of these conversation IDs. Omit for all conversations.
    conversation_ids: Option<Vec<String>>,
    /// Restrict results to messages from any of these sender IDs.
    sender_ids: Option<Vec<String>>,
    /// Restrict results to exact Mixin message categories.
    categories: Option<Vec<String>>,
    /// Inclusive lower timestamp bound in microseconds since Unix epoch.
    start_created_at_micros: Option<i64>,
    /// Exclusive upper timestamp bound in microseconds since Unix epoch.
    end_created_at_micros: Option<i64>,
    /// Opaque cursor returned by the previous page.
    cursor: Option<String>,
    /// Page size from 1 through 100. Defaults to 50.
    limit: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct MessageContextInput {
    message_id: String,
    limit: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ListParticipantsInput {
    conversation_id: String,
    query: Option<String>,
    limit: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct ResolveParticipantInput {
    conversation_id: String,
    query: String,
    limit: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SetDraftInput {
    conversation_id: String,
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SendTextInput {
    conversation_id: String,
    text: String,
    /// Message to quote; it must belong to the same conversation.
    quote_message_id: Option<String>,
    /// Render text as a Markdown post. Defaults to false.
    post: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SendImageInput {
    conversation_id: String,
    /// Base64 image bytes, or a data: URL. At most 16 MiB decoded.
    data_base64: String,
    caption: Option<String>,
    /// Message to quote; it must belong to the same conversation.
    quote_message_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SendStickerInput {
    conversation_id: String,
    sticker_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SendContactInput {
    conversation_id: String,
    /// User whose contact card is shared.
    user_id: String,
    /// Message to quote; it must belong to the same conversation.
    quote_message_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct SendAppCardInput {
    conversation_id: String,
    card: Map<String, Value>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct CreateCircleInput {
    name: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct RenameCircleInput {
    circle_id: String,
    name: String,
}

fn enabled_capabilities(state: &ServerState) -> Vec<&'static str> {
    let settings = &state.settings;
    let mut capabilities = vec![
        "mixin_get_app_status",
        "mixin_list_conversations",
        "mixin_get_conversation",
        "mixin_resolve_conversation",
        "mixin_list_messages",
        "mixin_search_messages",
        "mixin_get_message",
        "mixin_get_message_context",
        "mixin_read_attachment",
        "mixin_list_conversation_participants",
        "mixin_resolve_conversation_participant",
        "mixin_list_circles",
    ];
    if settings.draft_tools_enabled {
        capabilities.extend([
            "mixin_get_conversation_draft",
            "mixin_set_conversation_draft",
            "mixin_clear_conversation_draft",
        ]);
    }
    if settings.circle_management_enabled {
        capabilities.extend([
            "mixin_create_circle",
            "mixin_rename_circle",
            "mixin_delete_circle",
        ]);
    }
    if state.message_send {
        capabilities.extend(SEND_TOOLS);
    }
    capabilities
}

const SEND_TOOLS: [&str; 5] = [
    "mixin_send_text",
    "mixin_send_image",
    "mixin_send_sticker",
    "mixin_send_contact",
    "mixin_send_app_card",
];

const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;

const MAX_READ_ATTACHMENT_BYTES: u64 = 5 * 1024 * 1024;

/// Reads an image attachment and returns its base64 data and the MIME type of its actual format.
async fn read_attachment_image(path: &Path) -> Result<(String, &'static str)> {
    use base64ct::{Base64, Encoding};
    let size = tokio::fs::metadata(path).await?.len();
    if size == 0 || size > MAX_READ_ATTACHMENT_BYTES {
        return Err(anyhow!(
            "attachment must be between 1 byte and 5 MiB, got {size} bytes"
        ));
    }
    let bytes = tokio::fs::read(path).await?;
    let mime_type = match image::guess_format(&bytes) {
        Ok(image::ImageFormat::Png) => "image/png",
        Ok(image::ImageFormat::Jpeg) => "image/jpeg",
        Ok(image::ImageFormat::Gif) => "image/gif",
        Ok(image::ImageFormat::WebP) => "image/webp",
        _ => return Err(anyhow!("unsupported image format")),
    };
    Ok((Base64::encode_string(&bytes), mime_type))
}

struct DecodedImage {
    bytes: Vec<u8>,
    mime_type: &'static str,
    extension: &'static str,
    width: i32,
    height: i32,
}

fn decode_image(data: &str) -> Result<DecodedImage> {
    use base64ct::{Base64, Encoding};
    let data = data.trim();
    let data = match data.strip_prefix("data:") {
        Some(url) => url
            .split_once(";base64,")
            .map(|(_, data)| data)
            .ok_or_else(|| anyhow!("data URL must be base64 encoded"))?,
        None => data,
    };
    if data.len() > MAX_IMAGE_BYTES / 3 * 4 + 4 {
        return Err(anyhow!("image must be at most 16 MiB"));
    }
    let bytes = Base64::decode_vec(data).map_err(|_| anyhow!("data_base64 is not valid base64"))?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(anyhow!("image must be between 1 byte and 16 MiB"));
    }
    let reader =
        image::ImageReader::new(std::io::Cursor::new(bytes.as_slice())).with_guessed_format()?;
    let (mime_type, extension) = match reader.format() {
        Some(image::ImageFormat::Png) => ("image/png", "png"),
        Some(image::ImageFormat::Jpeg) => ("image/jpeg", "jpg"),
        Some(image::ImageFormat::Gif) => ("image/gif", "gif"),
        Some(image::ImageFormat::WebP) => ("image/webp", "webp"),
        _ => {
            return Err(anyhow!(
                "unsupported image format; use PNG, JPEG, GIF, or WebP"
            ))
        }
    };
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| anyhow!("invalid image: {error}"))?;
    Ok(DecodedImage {
        bytes,
        mime_type,
        extension,
        width: i32::try_from(width)?,
        height: i32::try_from(height)?,
    })
}

fn sent_json(conversation_id: String, message_id: String) -> Value {
    json!({"sent": true, "conversation_id": conversation_id, "message_id": message_id})
}

fn ensure_send(state: &ServerState) -> Result<()> {
    if state.message_send {
        Ok(())
    } else {
        Err(anyhow!("MCP permission scope message_send is disabled"))
    }
}

fn ensure_drafts(state: &ServerState) -> Result<()> {
    if state.settings.draft_tools_enabled {
        Ok(())
    } else {
        Err(anyhow!("MCP permission scope draft_write is disabled"))
    }
}

fn ensure_circles(state: &ServerState) -> Result<()> {
    if state.settings.circle_management_enabled {
        Ok(())
    } else {
        Err(anyhow!(
            "MCP permission scope circle_management is disabled"
        ))
    }
}

fn require_non_empty(value: String, name: &str) -> Result<String> {
    if value.trim().is_empty() {
        Err(anyhow!("{name} is required"))
    } else {
        Ok(value)
    }
}

fn validate_filter(values: Option<Vec<String>>, name: &str, limit: usize) -> Result<Vec<String>> {
    let values = values.unwrap_or_default();
    if values.len() > limit {
        return Err(anyhow!("{name} supports at most {limit} values"));
    }
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(anyhow!("{name} must not contain empty values"));
    }
    Ok(values)
}

fn object(value: Value) -> ToolOutput {
    match value {
        Value::Object(value) => value,
        _ => unreachable!("MCP tool output must be an object"),
    }
}

fn conversation_json(item: super::model::ConversationListData) -> Value {
    json!({"conversation_id":item.conversation_id,"owner_id":item.owner_id,"name":item.name,"avatar_url":item.avatar_url,"category":item.category,"draft":item.draft,"updated_at_millis":item.updated_at_millis,"unseen_count":item.unseen_count,"mention_count":item.mention_count,"is_muted":item.is_muted,"is_verified":item.is_verified,"is_scam":item.is_scam,"is_bot":item.is_bot,"is_pinned":item.is_pinned,"identity_number":item.identity_number,"circle_ids":item.circle_ids,"participant_count":item.participant_count})
}
fn message_json(item: super::model::MessageListView) -> Value {
    json!({"message_id":item.message_id,"conversation_id":item.conversation_id,"sender_id":item.sender_id,"sender_name":item.sender_name,"sender_identity_number":item.sender_identity_number,"category":item.category,"content":item.content,"status":item.status,"created_at_micros":item.created_at_micros,"media_url":item.media_url,"media_mime_type":item.media_mime_type,"media_size":item.media_size,"caption":item.caption,"quote_message_id":item.quote_message_id,"quote_content":item.quote_content,"sticker_id":item.sticker_id,"shared_user_id":item.shared_user_id,"is_pinned":item.pinned})
}
fn search_message_json(item: super::model::MessageListView, conversation: Option<Value>) -> Value {
    let mut message = message_json(item);
    message["conversation"] = conversation.unwrap_or(Value::Null);
    message
}
fn participant_json(item: super::model::ConversationParticipantItem) -> Value {
    json!({"user_id":item.user_id,"role":item.role,"created_at_millis":item.created_at_millis,"identity_number":item.identity_number,"full_name":item.full_name,"avatar_url":item.avatar_url,"biography":item.biography,"is_verified":item.is_verified,"is_bot":item.is_bot,"relationship":item.relationship,"membership":item.membership})
}
fn circle_json(item: super::model::CircleItem) -> Value {
    json!({"circle_id":item.circle_id,"name":item.name,"conversation_count":item.conversation_count})
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use chrono::Utc;
    use sdk::blaze_message::MessageStatus;

    use crate::db::app::AppDatabase;
    use crate::db::mixin::message::Message;

    use super::*;

    #[tokio::test]
    async fn server_initializes_token_and_persists_complete_settings() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let server = McpServer::new(database.setting_dao).await.unwrap();

        let initial = server.settings().await.unwrap();
        assert!(!initial.enabled);
        assert!(!initial.token.is_empty());

        let updated = McpSettings {
            enabled: false,
            token: "new-token".to_owned(),
            draft_tools_enabled: true,
            circle_management_enabled: true,
        };
        let status = server.update_settings(updated.clone()).await.unwrap();

        assert!(!status.running);
        assert_eq!(server.settings().await.unwrap(), updated);
    }

    #[tokio::test]
    async fn settings_batch_update_emits_one_complete_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let settings = database.setting_dao;
        settings
            .set(MCP_TOKEN_KEY, Some("\"initial-token\""))
            .await
            .unwrap();
        let subscription = McpServer::subscribe_settings(settings.clone());
        let mut subscription = Box::pin(subscription);

        assert_eq!(
            subscription.next().await.unwrap().unwrap(),
            McpSettings::disabled("initial-token".to_owned())
        );

        let updated = McpSettings {
            enabled: false,
            token: "new-token".to_owned(),
            draft_tools_enabled: true,
            circle_management_enabled: true,
        };
        McpServer::save_settings(&settings, &updated).await.unwrap();

        assert_eq!(subscription.next().await.unwrap().unwrap(), updated);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), subscription.next())
                .await
                .is_err()
        );
    }

    #[test]
    fn tool_router_builds_object_output_schemas_for_every_tool() {
        let tools = McpService::tool_router().list_all();

        assert_eq!(tools.len(), 23);
        // mixin_read_attachment returns image content instead of structured output.
        for tool in tools
            .into_iter()
            .filter(|tool| tool.name != "mixin_read_attachment")
        {
            let schema = tool
                .output_schema
                .unwrap_or_else(|| panic!("{} has no output schema", tool.name));
            assert_eq!(
                schema.get("type"),
                Some(&Value::String("object".to_owned())),
                "{} output schema is not an object",
                tool.name,
            );
        }
    }

    #[test]
    fn decode_image_reads_format_and_size_and_rejects_non_images() {
        use base64ct::{Base64, Encoding};
        let mut png = Vec::new();
        image::RgbImage::new(3, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let encoded = Base64::encode_string(&png);

        for data in [encoded.clone(), format!("data:image/png;base64,{encoded}")] {
            let image = decode_image(&data).unwrap();
            assert_eq!(
                (image.mime_type, image.extension, image.width, image.height),
                ("image/png", "png", 3, 2)
            );
            assert_eq!(image.bytes, png);
        }
        for data in [
            "not base64!".to_owned(),
            Base64::encode_string(b"plain text, not an image"),
            String::new(),
            "data:image/png,raw".to_owned(),
        ] {
            assert!(decode_image(&data).is_err(), "{data:?} must be rejected");
        }
    }

    #[tokio::test]
    async fn agent_event_batch_returns_triggered_messages_from_others_and_resumes_after_cursor() {
        let directory = tempfile::tempdir().unwrap();
        let identity_number = "39869091";
        let path = directory.path().join(identity_number).join("mixin.db");
        let database = MixinDatabase::connect_at(&path).await.unwrap();
        for (user_id, identity) in [("account-id", identity_number), ("friend-id", "7000")] {
            crate::db::query(
                r#"INSERT INTO users (
                       user_id, identity_number, relationship, full_name, avatar_url, phone,
                       is_verified, created_at, mute_until, has_pin, biography, is_scam,
                       code_url, code_id, is_deactivated
                   ) VALUES (?, ?, NULL, 'User', '', '', FALSE, 0, 0, FALSE, '', FALSE, '', '', FALSE)"#,
            )
            .bind(user_id)
            .bind(identity)
            .execute(&database.user_dao.0)
            .await
            .unwrap();
        }
        crate::db::query(
            "INSERT INTO conversations (conversation_id, owner_id, category, name, created_at, status, mute_until) VALUES \
             ('direct', 'friend-id', 'CONTACT', '', 0, 0, 0), \
             ('group', 'friend-id', 'GROUP', 'Dev', 0, 0, ?)",
        )
        .bind(i64::MAX)
        .execute(&database.conversation_dao.0)
        .await
        .unwrap();
        for (message_id, conversation_id, user_id, category, content) in [
            ("own", "direct", "account-id", "PLAIN_TEXT", "from me"),
            ("hello", "direct", "friend-id", "PLAIN_TEXT", "hello"),
            ("muted", "group", "friend-id", "PLAIN_TEXT", "chatter"),
            ("ask", "group", "friend-id", "PLAIN_TEXT", "@39869091 help"),
            ("recall", "direct", "friend-id", "MESSAGE_RECALL", "hello"),
        ] {
            database
                .message_dao
                .insert_message(&Message {
                    message_id: message_id.into(),
                    conversation_id: conversation_id.into(),
                    user_id: user_id.into(),
                    category: category.into(),
                    content: Some(content.into()),
                    status: MessageStatus::Delivered,
                    created_at: Utc::now().naive_utc(),
                    ..Message::default()
                })
                .await
                .unwrap();
        }
        database.close().await;
        let source = McpSource::ReadOnly(Arc::new(
            ReadOnlyMcpSource::open(directory.path(), identity_number.to_owned())
                .await
                .unwrap(),
        ));

        let batch = source.agent_event_batch(0, 200).await.unwrap();
        let resumed = source
            .agent_event_batch(batch.events[0].cursor, 200)
            .await
            .unwrap();
        let caught_up = source
            .agent_event_batch(batch.next_row_id, 200)
            .await
            .unwrap();

        let summary = |batch: &AgentEventBatch| {
            batch
                .events
                .iter()
                .map(|event| {
                    (
                        event.data["message"]["message_id"]
                            .as_str()
                            .unwrap()
                            .to_owned(),
                        event.data["trigger"].as_str().unwrap().to_owned(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            summary(&batch),
            [
                ("hello".to_owned(), "direct".to_owned()),
                ("ask".to_owned(), "mention".to_owned()),
            ]
        );
        assert_eq!(batch.events[1].data["message"]["sender_id"], "friend-id");
        assert_eq!(batch.events[1].data["conversation"]["category"], "GROUP");
        assert!(!batch.has_more);
        assert_eq!(
            summary(&resumed),
            [("ask".to_owned(), "mention".to_owned())]
        );
        assert!(caught_up.events.is_empty());
        assert_eq!(caught_up.next_row_id, batch.next_row_id);
    }

    async fn read_only_test_source() -> (tempfile::TempDir, Arc<ReadOnlyMcpSource>) {
        let directory = tempfile::tempdir().unwrap();
        let identity_number = "39869091";
        let account_directory = directory.path().join(identity_number);
        let path = account_directory.join("mixin.db");
        let database = MixinDatabase::connect_at(&path).await.unwrap();
        crate::db::query(
            r#"INSERT INTO users (
                   user_id, identity_number, relationship, full_name, avatar_url, phone,
                   is_verified, created_at, mute_until, has_pin, biography, is_scam,
                   code_url, code_id, is_deactivated
               ) VALUES (?, ?, NULL, 'Mixin User', '', '', FALSE, 0, 0, FALSE, '', FALSE, '', '', FALSE)"#,
        )
        .bind("account-id")
        .bind(identity_number)
        .execute(&database.user_dao.0)
        .await
        .unwrap();
        crate::db::query(
            "INSERT INTO conversations (conversation_id, owner_id, category, name, created_at, status) \
             VALUES ('conversation', 'account-id', 'GROUP', 'Route Dev', 0, 0)",
        )
        .execute(&database.conversation_dao.0)
        .await
        .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "price-alert-message".into(),
                conversation_id: "conversation".into(),
                user_id: "account-id".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("price alert expires after 90 days".into()),
                status: MessageStatus::Sent,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        database
            .message_fts_dao
            .upsert(
                "price-alert-message",
                "conversation",
                "price alert expires after 90 days",
            )
            .await
            .unwrap();
        database.close().await;
        let source = Arc::new(
            ReadOnlyMcpSource::open(directory.path(), identity_number.to_owned())
                .await
                .unwrap(),
        );
        (directory, source)
    }

    #[tokio::test]
    async fn busy_search_rejects_immediately_while_message_reads_succeed() {
        let (_directory, source) = read_only_test_source().await;
        let service = McpService::new(ServerState {
            search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            source: McpSource::ReadOnly(source),
            settings: McpSettings::disabled("token".into()),
            message_send: false,
        });
        let permit = service.state.search_slot.acquire().await.unwrap();
        let input = || SearchMessagesInput {
            query: "price".into(),
            conversation_ids: None,
            sender_ids: None,
            categories: None,
            start_created_at_micros: None,
            end_created_at_micros: None,
            cursor: None,
            limit: Some(1),
        };
        let error = service
            .search_messages_input(input(), CancellationToken::new())
            .await
            .err()
            .expect("a second search must be rejected");
        assert!(error.starts_with("search_busy:"));
        let page = service
            .mixin_list_messages(Parameters(ListMessagesInput {
                conversation_id: "conversation".into(),
                before_created_at_micros: None,
                before_message_id: None,
                start_created_at_micros: None,
                end_created_at_micros: None,
                limit: Some(1),
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(page["messages"].as_array().unwrap().len(), 1);
        drop(permit);
        assert!(service
            .search_messages_input(input(), CancellationToken::new())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn time_range_message_pages_return_reusable_cursors_and_exact_microsecond_bounds() {
        let (_directory, source) = read_only_test_source().await;
        let database = MixinDatabase::connect_at(&source.info.database_path)
            .await
            .unwrap();
        for (id, timestamp) in [("one", 1000), ("two", 1001), ("three", 1002)] {
            database
                .message_dao
                .insert_message(&Message {
                    message_id: id.into(),
                    conversation_id: "conversation".into(),
                    user_id: "account-id".into(),
                    category: "PLAIN_TEXT".into(),
                    status: MessageStatus::Sent,
                    created_at: chrono::DateTime::from_timestamp_millis(timestamp)
                        .unwrap()
                        .naive_utc(),
                    ..Message::default()
                })
                .await
                .unwrap();
        }
        database.close().await;
        let service = McpService::new(ServerState {
            search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            source: McpSource::ReadOnly(source),
            settings: McpSettings::disabled("token".into()),
            message_send: false,
        });
        let input = |cursor: Option<&Value>| ListMessagesInput {
            conversation_id: "conversation".into(),
            start_created_at_micros: Some(1000001),
            end_created_at_micros: Some(1002001),
            limit: Some(1),
            before_created_at_micros: cursor.and_then(|c| c["before_created_at_micros"].as_i64()),
            before_message_id: cursor
                .and_then(|c| c["before_message_id"].as_str())
                .map(str::to_owned),
        };
        let first = service
            .mixin_list_messages(Parameters(input(None)))
            .await
            .unwrap()
            .0;
        assert_eq!(first["messages"][0]["message_id"], "three");
        assert_eq!(first["pagination"]["has_more"], true);
        let second = service
            .mixin_list_messages(Parameters(input(Some(&first["pagination"]["next_cursor"]))))
            .await
            .unwrap()
            .0;
        assert_eq!(second["messages"][0]["message_id"], "two");
        assert_eq!(second["pagination"]["has_more"], false);
        assert!(second["pagination"]["next_cursor"].is_null());
    }

    /// A read-only service with one transcript holding a text, a downloaded image, and a
    /// pending image child.
    async fn transcript_test_service() -> (tempfile::TempDir, McpService) {
        use crate::db::mixin::message::MediaStatus;
        let (directory, source) = read_only_test_source().await;
        let database = MixinDatabase::connect_at(&source.info.database_path)
            .await
            .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "transcript".into(),
                conversation_id: "conversation".into(),
                user_id: "account-id".into(),
                category: "SIGNAL_TRANSCRIPT".into(),
                content: Some("[]".into()),
                status: MessageStatus::Sent,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        let mut children: Vec<crate::db::mixin::transcript_message::TranscriptMessage> =
            serde_json::from_value(json!([
                {"transcript_id":"transcript","message_id":"text-child","category":"SIGNAL_TEXT",
                 "created_at":"2024-01-02T03:04:05Z","content":"signing failed"},
                {"transcript_id":"transcript","message_id":"image-child","category":"SIGNAL_IMAGE",
                 "created_at":"2024-01-02T03:04:06Z","content":"attachment-id",
                 "media_url":"image-child.png","media_mime_type":"image/png","media_size":42},
                {"transcript_id":"transcript","message_id":"pending-child","category":"SIGNAL_IMAGE",
                 "created_at":"2024-01-02T03:04:07Z","content":"attachment-id",
                 "media_mime_type":"image/png","media_size":42}
            ]))
            .unwrap();
        children[1].media_status = Some(MediaStatus::Done);
        children[2].media_status = Some(MediaStatus::Pending);
        database
            .transcript_message_dao
            .insert_all(&children)
            .await
            .unwrap();
        database.close().await;
        let service = McpService::new(ServerState {
            search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            source: McpSource::ReadOnly(source),
            settings: McpSettings::disabled("token".into()),
            message_send: false,
        });
        (directory, service)
    }

    #[tokio::test]
    async fn get_transcript_message_returns_children_with_local_media_paths() {
        let (directory, service) = transcript_test_service().await;

        let output = service
            .mixin_get_message(Parameters(MessageIdInput {
                message_id: "transcript".into(),
            }))
            .await
            .unwrap()
            .0;

        assert_eq!(output["message"]["message_id"], "transcript");
        let children = output["transcript_messages"].as_array().unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0]["content"], "signing failed");
        assert_eq!(children[1]["message_id"], "image-child");
        assert_eq!(children[1]["media_status"], "DONE");
        let expected = directory
            .path()
            .join("39869091/Media/Transcripts/image-child.png");
        assert_eq!(
            children[1]["media_url"].as_str(),
            Some(expected.to_str().unwrap())
        );
    }

    #[tokio::test]
    async fn read_attachment_returns_downloaded_transcript_image_and_rejects_pending_one() {
        use base64ct::{Base64, Encoding};
        let (directory, service) = transcript_test_service().await;
        let mut png = Vec::new();
        image::RgbaImage::new(1, 1)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let media = directory.path().join("39869091/Media/Transcripts");
        std::fs::create_dir_all(&media).unwrap();
        std::fs::write(media.join("image-child.png"), &png).unwrap();
        let input = |message_id: &str| ReadAttachmentInput {
            message_id: message_id.into(),
            transcript_id: Some("transcript".into()),
        };

        let result = service
            .mixin_read_attachment(Parameters(input("image-child")))
            .await
            .unwrap();
        let ContentBlock::Image(image) = &result.content[0] else {
            panic!("expected image content");
        };
        assert_eq!(image.mime_type, "image/png");
        assert_eq!(Base64::decode_vec(&image.data).unwrap(), png);

        let error = service
            .mixin_read_attachment(Parameters(input("pending-child")))
            .await
            .err()
            .unwrap();
        assert_eq!(error, "attachment is not downloaded: media_status=PENDING");
    }

    #[tokio::test]
    async fn database_sidecar_exposes_only_read_tools_and_reports_its_source() {
        let (_directory, source) = read_only_test_source().await;
        let cancellation = CancellationToken::new();
        let app = http_router(
            ServerState {
                search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
                source: McpSource::ReadOnly(source.clone()),
                settings: McpSettings::disabled("test-token".into()),
                message_send: false,
            },
            &cancellation,
        );
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
        let stop = cancellation.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        for token in [None, Some("wrong-token"), Some("test-token")] {
            let mut request = client
                .post(&endpoint)
                .header("Accept", "application/json, text/event-stream")
                .header("Content-Type", "application/json")
                .body(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string());
            if let Some(token) = token {
                request = request.bearer_auth(token);
            }
            let response = request.send().await.unwrap();
            if token == Some("test-token") {
                assert_eq!(response.status(), StatusCode::OK);
                let body: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
                let tools = body["result"]["tools"].as_array().unwrap();
                assert!(tools
                    .iter()
                    .any(|tool| tool["name"] == "mixin_search_messages"));
                assert!(!tools
                    .iter()
                    .any(|tool| tool["name"] == "mixin_update_draft"));
            } else {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            }
        }
        for (name, arguments) in [
            (
                "mixin_search_messages",
                json!({"query": "price alert", "limit": 1}),
            ),
            (
                "mixin_list_messages",
                json!({"conversation_id": "conversation", "start_created_at_micros": 0, "limit": 1}),
            ),
        ] {
            let response = client
                .post(&endpoint)
                .bearer_auth("test-token")
                .header("Accept", "application/json, text/event-stream")
                .header("Content-Type", "application/json")
                .body(
                    json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                    "params": {"name": name, "arguments": arguments}})
                    .to_string(),
                )
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
            assert_ne!(body["result"]["isError"], true, "{body}");
            assert_eq!(
                body["result"]["structuredContent"]["messages"][0]["message_id"],
                "price-alert-message",
                "{body}"
            );
        }
        cancellation.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        let service = McpService::new(ServerState {
            search_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            source: McpSource::ReadOnly(source),
            settings: McpSettings::disabled("token".to_owned()),
            message_send: false,
        });

        let tools = service
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<HashSet<_>>();
        let status = service.mixin_get_app_status().await.0;
        let search = service
            .search_messages_input(
                SearchMessagesInput {
                    query: "price alert".into(),
                    conversation_ids: Some(vec!["conversation".into()]),
                    sender_ids: Some(vec!["account-id".into()]),
                    categories: Some(vec!["PLAIN_TEXT".into()]),
                    start_created_at_micros: None,
                    end_created_at_micros: None,
                    cursor: None,
                    limit: Some(10),
                },
                CancellationToken::new(),
            )
            .await
            .unwrap()
            .0;

        assert_eq!(tools.len(), 12);
        assert!(tools.contains("mixin_list_conversations"));
        assert!(tools.contains("mixin_search_messages"));
        assert!(!tools.contains("mixin_set_conversation_draft"));
        assert!(!tools.contains("mixin_create_circle"));
        assert!(SEND_TOOLS.iter().all(|name| !tools.contains(*name)));
        assert_eq!(status["permission_scopes"]["message_send"], false);
        assert_eq!(status.get("source"), Some(&json!("database")));
        assert_eq!(status.get("read_only"), Some(&json!(true)));
        assert_eq!(status.get("user_id"), Some(&json!("account-id")));
        assert_eq!(search["messages"][0]["message_id"], "price-alert-message");
        assert_eq!(search["messages"][0]["conversation"]["name"], "Route Dev");
        assert_eq!(search["pagination"]["has_more"], false);
    }
}
