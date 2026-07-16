use std::sync::Arc;

use anyhow::Result;
use mixin_desktop_core::core::model::auth::{AuthService, AuthorizationSession};
use mixin_desktop_core::db::app::AppDatabase;
use mixin_desktop_core::runtime::AccountRuntime;
use tokio::sync::Mutex;

use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopHandle {
    auth_service: Arc<AuthService>,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct LoginHandle {
    auth_service: Arc<AuthService>,
    session: Mutex<Option<AuthorizationSession>>,
    auth_url: String,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct AccountHandle {
    runtime: Arc<AccountRuntime>,
}

pub struct ConversationListItem {
    pub conversation_id: String,
    pub owner_id: String,
    pub name: String,
    pub avatar_url: String,
    pub category: String,
    pub draft: String,
    pub status: i32,
    pub last_message: String,
    pub last_message_category: Option<String>,
    pub last_message_status: Option<String>,
    pub last_message_sender_id: Option<String>,
    pub last_message_sender_name: Option<String>,
    pub updated_at_millis: i64,
    pub unseen_count: i64,
    pub mention_count: i64,
    pub is_muted: bool,
    pub is_verified: bool,
    pub is_bot: bool,
    pub is_pinned: bool,
    pub relationship: String,
    pub identity_number: String,
    pub circle_ids: Vec<String>,
    pub group_avatars: Vec<GroupAvatar>,
}

pub struct GroupAvatar {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
}

pub struct AccountProfile {
    pub user_id: String,
    pub full_name: String,
    pub avatar_url: String,
    pub identity_number: String,
}

pub struct CircleItem {
    pub circle_id: String,
    pub name: String,
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

pub async fn open_desktop() -> Result<DesktopHandle> {
    let database = Arc::new(AppDatabase::connect().await?);
    let auth_service = Arc::new(AuthService::new(database));
    auth_service.initialize().await?;
    Ok(DesktopHandle { auth_service })
}

impl DesktopHandle {
    pub async fn restore_account(&self) -> Result<Option<AccountHandle>> {
        let Some(auth) = self.auth_service.get_auth() else {
            return Ok(None);
        };
        let runtime = AccountRuntime::start(auth).await?;
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
        }))
    }

    pub async fn begin_login(&self) -> Result<LoginHandle> {
        let session = self
            .auth_service
            .begin_authorization(desktop_platform())
            .await?;
        let auth_url = session.auth_url().to_string();
        Ok(LoginHandle {
            auth_service: self.auth_service.clone(),
            session: Mutex::new(Some(session)),
            auth_url,
        })
    }
}

impl LoginHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn auth_url(&self) -> String {
        self.auth_url.clone()
    }

    pub async fn poll(&self) -> Result<Option<AccountHandle>> {
        let mut session = self.session.lock().await;
        let Some(active_session) = session.as_ref() else {
            return Ok(None);
        };
        let Some(result) = self.auth_service.poll_authorization(active_session).await? else {
            return Ok(None);
        };
        let auth = self.auth_service.complete_authorization(result).await?;
        let runtime = AccountRuntime::start(auth).await?;
        session.take();
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
        }))
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn account_id(&self) -> String {
        self.runtime.account_id().to_string()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn profile(&self) -> AccountProfile {
        let account = self.runtime.account();
        AccountProfile {
            user_id: account.user_id.clone(),
            full_name: account.full_name.clone().unwrap_or_default(),
            avatar_url: account.avatar_url.clone().unwrap_or_default(),
            identity_number: account.identity_number.clone(),
        }
    }

    pub async fn conversation_count(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
    ) -> Result<i64> {
        self.runtime
            .conversation_count(&category, circle_id.as_deref(), &keyword, unseen_only)
            .await
    }

    pub async fn conversations(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationListItem>> {
        Ok(self
            .runtime
            .conversations(
                &category,
                circle_id.as_deref(),
                &keyword,
                unseen_only,
                limit,
                offset,
            )
            .await?
            .into_iter()
            .map(|item| {
                let updated_at_millis = item.updated_at_millis();
                ConversationListItem {
                    conversation_id: item.conversation_id,
                    owner_id: item.owner_id,
                    name: item.name,
                    avatar_url: item.avatar_url,
                    category: item.category,
                    draft: item.draft,
                    status: item.status,
                    last_message: item.last_message,
                    last_message_category: item.last_message_category,
                    last_message_status: item.last_message_status,
                    last_message_sender_id: item.last_message_sender_id,
                    last_message_sender_name: item.last_message_sender_name,
                    updated_at_millis,
                    unseen_count: item.unseen_count,
                    mention_count: item.mention_count,
                    is_muted: item.is_muted,
                    is_verified: item.is_verified,
                    is_bot: item.is_bot,
                    is_pinned: item.is_pinned,
                    relationship: item.relationship,
                    identity_number: item.identity_number,
                    circle_ids: item
                        .circle_ids
                        .split(',')
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .collect(),
                    group_avatars: item
                        .group_avatar_data
                        .split('\u{1e}')
                        .filter_map(|value| {
                            let mut parts = value.split('\u{1f}');
                            Some(GroupAvatar {
                                user_id: parts.next()?.to_string(),
                                name: parts.next()?.to_string(),
                                avatar_url: parts.next()?.to_string(),
                            })
                        })
                        .collect(),
                }
            })
            .collect())
    }

    pub async fn conversation_changes(&self, sink: StreamSink<u64>) -> Result<()> {
        let mut changes = self.runtime.subscribe_conversation_changes();
        while changes.changed().await.is_ok() {
            if sink.add(*changes.borrow()).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn circles(&self) -> Result<Vec<CircleItem>> {
        Ok(self
            .runtime
            .circles()
            .await?
            .into_iter()
            .map(|circle| CircleItem {
                circle_id: circle.circle_id,
                name: circle.name,
            })
            .collect())
    }

    pub async fn set_conversation_pinned(
        &self,
        conversation_id: String,
        pinned: bool,
    ) -> Result<()> {
        self.runtime.set_pinned(&conversation_id, pinned).await
    }

    pub async fn set_conversation_muted(
        &self,
        conversation_id: String,
        owner_id: String,
        category: String,
        duration_seconds: i64,
    ) -> Result<()> {
        self.runtime
            .set_muted(&conversation_id, &owner_id, &category, duration_seconds)
            .await
    }

    pub async fn delete_conversation(&self, conversation_id: String) -> Result<()> {
        self.runtime.delete_conversation(&conversation_id).await
    }

    pub async fn edit_circle_conversation(
        &self,
        circle_id: String,
        conversation_id: String,
        owner_id: String,
        is_group: bool,
        add: bool,
    ) -> Result<()> {
        self.runtime
            .edit_circle_conversation(&circle_id, &conversation_id, &owner_id, is_group, add)
            .await
    }

    pub async fn shutdown(&self) {
        self.runtime.shutdown().await;
    }
}

fn desktop_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Linux"
    }
}
