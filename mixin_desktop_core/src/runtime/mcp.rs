//! Local, authenticated MCP server for a running account.
//!
//! The server deliberately exposes no message-send or account-write operation.

use std::collections::HashSet;
use std::sync::Arc;

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
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use serde_json::{json, Map, Value};
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::app::SettingDao;

use super::AccountRuntime;

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
}

impl McpServerState {
    fn status(&self) -> McpServerStatus {
        McpServerStatus {
            running: self.running.is_some(),
            endpoint: self
                .running
                .as_ref()
                .map(|server| server.endpoint().to_owned()),
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
            match RunningMcpServer::start(runtime, settings).await {
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

    pub async fn stop(&self) {
        let mut state = self.state.lock().await;
        state.runtime = None;
        if let Some(server) = state.running.take() {
            server.stop().await;
        }
    }

    async fn apply_settings(&self, settings: McpSettings) -> Result<McpServerStatus> {
        self.state.lock().await.apply(settings, false).await
    }

    async fn start_settings_subscription(self: &Arc<Self>) {
        let subscription = Self::subscribe_settings(self.settings.clone());
        let server = Arc::downgrade(self);
        let task = tokio::spawn(async move {
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
    async fn start(runtime: Arc<AccountRuntime>, settings: McpSettings) -> Result<Self> {
        if settings.token.trim().is_empty() {
            return Err(anyhow!("MCP access token is unavailable"));
        }
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, DEFAULT_PORT)).await?;
        let cancellation = CancellationToken::new();
        let state = ServerState { runtime, settings };
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
        let app = Router::new().nest_service("/mcp", service).layer(
            axum::middleware::from_fn_with_state(state.settings.token.clone(), authorize),
        );
        let stop = cancellation.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
            {
                log::error!("local MCP server stopped unexpectedly: {error}");
            }
        });
        Ok(Self {
            cancellation,
            task: Some(task),
            endpoint: format!("http://127.0.0.1:{DEFAULT_PORT}/mcp"),
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
    runtime: Arc<AccountRuntime>,
    settings: McpSettings,
}

async fn authorize(token: axum::extract::State<String>, request: Request, next: Next) -> Response {
    if authorized(request.headers(), &token) {
        next.run(request).await
    } else {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
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
        Self { state, tool_router }
    }
}

#[tool_router]
impl McpService {
    #[tool(description = "Get login state, permission scopes, and enabled MCP tools.")]
    async fn mixin_get_app_status(&self) -> Json<ToolOutput> {
        Json(object(json!({
            "logged_in": self.state.runtime.is_running(),
            "user_id": self.state.runtime.account_id(),
            "permission_scopes": {
                "read": true,
                "draft_write": self.state.settings.draft_tools_enabled,
                "circle_management": self.state.settings.circle_management_enabled,
                "message_send": false,
                "account_write": false,
            },
            "capabilities": enabled_capabilities(&self.state.settings),
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
            .runtime
            .conversation_access()
            .conversations(
                "chats".into(),
                input.circle_id,
                input.query.unwrap_or_default(),
                false,
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
                .runtime
                .conversation_access()
                .conversation_items_by_ids(vec![id])
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
                    .runtime
                    .conversation_access()
                    .conversation_items_by_ids(vec![id])
                    .await?
                    .into_iter()
                    .next()
            } else if let Some(uri) = input.uri {
                let id = uri
                    .strip_prefix("mixin://conversations/")
                    .ok_or_else(|| anyhow!("unsupported conversation URI"))?;
                let id = require_non_empty(id.to_owned(), "conversation_id")?;
                self.state
                    .runtime
                    .conversation_access()
                    .conversation_items_by_ids(vec![id])
                    .await?
                    .into_iter()
                    .next()
            } else {
                let query = require_non_empty(input.query.unwrap_or_default(), "query")?;
                self.state
                    .runtime
                    .conversation_access()
                    .conversations("chats".into(), None, query, false, 1, 0)
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

    #[tool(description = "List local messages in newest-to-oldest order.")]
    async fn mixin_list_messages(
        &self,
        Parameters(input): Parameters<ListMessagesInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result = match require_non_empty(input.conversation_id, "conversation_id") {
            Ok(conversation_id) => self
                .state
                .runtime
                .message_access()
                .messages(
                    conversation_id,
                    input.before_created_at_micros,
                    input.before_message_id,
                    input.limit.unwrap_or(100).clamp(1, 200),
                )
                .await
                .map(|messages| {
                    json!({"messages": messages.into_iter().map(message_json).collect::<Vec<_>>()})
                }),
            Err(error) => Err(error.into()),
        };
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
    }

    #[tool(description = "Get one local message by message_id.")]
    async fn mixin_get_message(
        &self,
        Parameters(input): Parameters<MessageIdInput>,
    ) -> Result<Json<ToolOutput>, String> {
        let result: Result<Value> = async {
            let id = require_non_empty(input.message_id, "message_id")?;
            let message = self
                .state
                .runtime
                .message_access()
                .message_items_by_ids(vec![id])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("message not found"))?;
            Ok(json!({"message": message_json(message)}))
        }
        .await;
        result
            .map(object)
            .map(Json)
            .map_err(|error| error.to_string())
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
                .runtime
                .message_access()
                .message_items_by_ids(vec![id.clone()])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("message not found"))?;
            let messages = self
                .state
                .runtime
                .message_access()
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
            Ok(conversation_id) => self
                .state
                .runtime
                .conversation_access()
                .conversation_participants(conversation_id)
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
                }),
            Err(error) => Err(error.into()),
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
                .runtime
                .conversation_access()
                .conversation_participants(conversation_id)
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
        let result = self
            .state
            .runtime
            .conversation_access()
            .circles()
            .await
            .map(|circles| {
                json!({"circles": circles.into_iter().map(circle_json).collect::<Vec<_>>()})
            });
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
                .runtime
                .conversation_access()
                .conversation_items_by_ids(vec![id.clone()])
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
            self.state
                .runtime
                .database
                .conversation_dao
                .update_draft(&id, &text)
                .await?;
            self.state.runtime.notify_conversation_changed(&id);
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
            self.state
                .runtime
                .database
                .conversation_dao
                .update_draft(&id, "")
                .await?;
            self.state.runtime.notify_conversation_changed(&id);
            Ok(json!({"updated": true, "conversation_id": id}))
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
                .runtime
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
                .runtime
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
                .runtime
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
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Local Mixin desktop access. It never sends messages.")
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
    before_created_at_micros: Option<i64>,
    before_message_id: Option<String>,
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
struct CreateCircleInput {
    name: String,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct RenameCircleInput {
    circle_id: String,
    name: String,
}

fn enabled_capabilities(settings: &McpSettings) -> Vec<&'static str> {
    let mut capabilities = vec![
        "mixin_get_app_status",
        "mixin_list_conversations",
        "mixin_get_conversation",
        "mixin_resolve_conversation",
        "mixin_list_messages",
        "mixin_get_message",
        "mixin_get_message_context",
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
    capabilities
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
    json!({"message_id":item.message_id,"conversation_id":item.conversation_id,"sender_id":item.sender_id,"sender_name":item.sender_name,"sender_identity_number":item.sender_identity_number,"category":item.category,"content":item.content,"status":item.status,"created_at_micros":item.created_at_micros,"media_url":item.media_url,"media_mime_type":item.media_mime_type,"media_size":item.media_size,"caption":item.caption,"quote_message_id":item.quote_message_id,"quote_content":item.quote_content,"is_pinned":item.pinned})
}
fn participant_json(item: super::model::ConversationParticipantItem) -> Value {
    json!({"user_id":item.user_id,"role":item.role,"created_at_millis":item.created_at_millis,"identity_number":item.identity_number,"full_name":item.full_name,"avatar_url":item.avatar_url,"biography":item.biography,"is_verified":item.is_verified,"is_bot":item.is_bot,"relationship":item.relationship,"membership":item.membership})
}
fn circle_json(item: super::model::CircleItem) -> Value {
    json!({"circle_id":item.circle_id,"name":item.name,"conversation_count":item.conversation_count})
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::db::app::AppDatabase;

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

        assert_eq!(tools.len(), 16);
        for tool in tools {
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
}
