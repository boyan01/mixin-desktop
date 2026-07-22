//! Local, authenticated MCP server for a running account.
//!
//! The server deliberately exposes no message-send or account-write operation.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::extract::Request;
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Router;
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
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::AccountRuntime;

pub const DEFAULT_PORT: u16 = 55001;
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

pub fn generate_access_token() -> String {
    Uuid::new_v4().simple().to_string()
}

pub struct McpServer {
    cancellation: CancellationToken,
    task: JoinHandle<()>,
    endpoint: String,
}

impl McpServer {
    pub async fn start(runtime: Arc<AccountRuntime>, settings: McpSettings) -> Result<Self> {
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
            task,
            endpoint: format!("http://127.0.0.1:{DEFAULT_PORT}/mcp"),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn stop(self) {
        self.cancellation.cancel();
        let _ = self.task.await;
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
            Err(error) => Err(error),
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
    use super::*;

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
