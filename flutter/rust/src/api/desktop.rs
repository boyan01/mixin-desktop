use std::sync::Arc;

use anyhow::Result;
use log::warn;
use mixin_desktop_core::core::model::auth::{AuthService, AuthorizationSession};
use mixin_desktop_core::db::app::AppDatabase;
use mixin_desktop_core::db::SignalDatabase;
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
    auth_service: Arc<AuthService>,
}

pub struct ConversationListItem {
    pub conversation_id: String,
    pub owner_id: String,
    pub name: String,
    pub avatar_url: String,
    pub category: String,
    pub draft: String,
    pub status: i32,
    pub last_read_message_id: Option<String>,
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
    pub participant_count: i64,
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
    pub biography: String,
    pub phone: String,
    pub created_at: String,
}

pub struct UserProfileItem {
    pub user_id: String,
    pub identity_number: String,
    pub full_name: String,
    pub avatar_url: String,
    pub biography: String,
    pub is_verified: bool,
    pub is_bot: bool,
    pub relationship: String,
}

pub struct CircleItem {
    pub circle_id: String,
    pub name: String,
}

pub struct MessageListItem {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_identity_number: Option<String>,
    pub sender_avatar_url: String,
    pub sender_is_verified: bool,
    pub sender_relationship: String,
    pub sender_app_id: Option<String>,
    pub sender_is_scam: bool,
    pub sender_is_bot: bool,
    pub category: String,
    pub content: String,
    pub status: String,
    pub created_at_micros: i64,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_duration: String,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub thumb_image: Option<String>,
    pub media_status: String,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub caption: Option<String>,
    pub action: Option<String>,
    pub participant_id: Option<String>,
    pub participant_full_name: Option<String>,
    pub snapshot_id: Option<String>,
    pub snapshot_type: Option<String>,
    pub snapshot_amount: Option<String>,
    pub snapshot_memo: Option<String>,
    pub snapshot_asset_id: Option<String>,
    pub snapshot_asset_symbol: Option<String>,
    pub snapshot_asset_icon_url: Option<String>,
    pub snapshot_chain_icon_url: Option<String>,
    pub snapshot_opponent_id: Option<String>,
    pub snapshot_transaction_hash: Option<String>,
    pub snapshot_created_at: Option<String>,
    pub inscription_hash: Option<String>,
    pub inscription_collection_hash: Option<String>,
    pub inscription_sequence: Option<i64>,
    pub inscription_content_type: Option<String>,
    pub inscription_content_url: Option<String>,
    pub inscription_name: Option<String>,
    pub inscription_icon_url: Option<String>,
    pub hyperlink: Option<String>,
    pub media_name: Option<String>,
    pub album_id: Option<String>,
    pub sticker_id: Option<String>,
    pub shared_user_id: Option<String>,
    pub media_waveform: Option<String>,
    pub thumb_url: Option<String>,
    pub conversation_owner_id: Option<String>,
    pub conversation_category: Option<String>,
    pub shared_user_full_name: Option<String>,
    pub shared_user_identity_number: Option<String>,
    pub shared_user_avatar_url: Option<String>,
    pub shared_user_is_verified: bool,
    pub shared_user_app_id: Option<String>,
    pub sticker_asset_url: Option<String>,
    pub sticker_asset_width: Option<i32>,
    pub sticker_asset_height: Option<i32>,
    pub sticker_asset_name: Option<String>,
    pub sticker_asset_type: Option<String>,
    pub mention_read: Option<bool>,
    pub pinned: bool,
    pub expire_in: Option<i64>,
}

pub struct ImageMessageItem {
    pub message_id: String,
    pub created_at_micros: i64,
    pub media_url: String,
    pub media_name: Option<String>,
    pub can_forward: bool,
}

impl From<mixin_desktop_core::db::mixin::message::MessageListItem> for MessageListItem {
    fn from(item: mixin_desktop_core::db::mixin::message::MessageListItem) -> Self {
        let created_at_micros = item.created_at_micros();
        Self {
            message_id: item.message_id,
            conversation_id: item.conversation_id,
            sender_id: item.user_id,
            sender_name: item.sender_name,
            sender_identity_number: Some(item.sender_identity_number),
            sender_avatar_url: item.sender_avatar_url,
            sender_is_verified: item.sender_is_verified,
            sender_relationship: item.sender_relationship,
            sender_app_id: item.sender_app_id,
            sender_is_scam: item.sender_is_scam,
            sender_is_bot: item.sender_is_bot,
            category: item.category,
            content: item.content.unwrap_or_default(),
            status: item.status.into(),
            created_at_micros,
            media_url: item.media_url,
            media_mime_type: item.media_mime_type,
            media_size: item.media_size,
            media_duration: item.media_duration,
            media_width: item.media_width,
            media_height: item.media_height,
            thumb_image: item.thumb_image,
            media_status: format!("{:?}", item.media_status).to_uppercase(),
            quote_message_id: item.quote_message_id,
            quote_content: item.quote_content,
            caption: item.caption,
            action: item.action,
            participant_id: item.participant_id,
            participant_full_name: item.participant_full_name,
            snapshot_id: item.snapshot_id,
            snapshot_type: item.snapshot_type,
            snapshot_amount: item.snapshot_amount,
            snapshot_memo: item.snapshot_memo,
            snapshot_asset_id: item.snapshot_asset_id,
            snapshot_asset_symbol: item.snapshot_asset_symbol,
            snapshot_asset_icon_url: item.snapshot_asset_icon_url,
            snapshot_chain_icon_url: item.snapshot_chain_icon_url,
            snapshot_opponent_id: item.snapshot_opponent_id,
            snapshot_transaction_hash: item.snapshot_transaction_hash,
            snapshot_created_at: item.snapshot_created_at,
            inscription_hash: item.inscription_hash,
            inscription_collection_hash: item.inscription_collection_hash,
            inscription_sequence: item.inscription_sequence,
            inscription_content_type: item.inscription_content_type,
            inscription_content_url: item.inscription_content_url,
            inscription_name: item.inscription_name,
            inscription_icon_url: item.inscription_icon_url,
            hyperlink: item.hyperlink,
            media_name: item.media_name,
            album_id: item.album_id,
            sticker_id: item.sticker_id,
            shared_user_id: item.shared_user_id,
            media_waveform: item.media_waveform,
            thumb_url: item.thumb_url,
            conversation_owner_id: item.conversation_owner_id,
            conversation_category: item.conversation_category,
            shared_user_full_name: item.shared_user_full_name,
            shared_user_identity_number: item.shared_user_identity_number,
            shared_user_avatar_url: item.shared_user_avatar_url,
            shared_user_is_verified: item.shared_user_is_verified,
            shared_user_app_id: item.shared_user_app_id,
            sticker_asset_url: item.sticker_asset_url,
            sticker_asset_width: item.sticker_asset_width,
            sticker_asset_height: item.sticker_asset_height,
            sticker_asset_name: item.sticker_asset_name,
            sticker_asset_type: item.sticker_asset_type,
            mention_read: item.mention_read,
            pinned: item.pinned,
            expire_in: item.expire_in,
        }
    }
}

impl From<mixin_desktop_core::db::mixin::transcript_message::TranscriptMessageListItem>
    for MessageListItem
{
    fn from(
        item: mixin_desktop_core::db::mixin::transcript_message::TranscriptMessageListItem,
    ) -> Self {
        Self {
            message_id: item.message_id,
            conversation_id: item.conversation_id,
            sender_id: item.user_id,
            sender_name: item.sender_name,
            sender_identity_number: Some(item.sender_identity_number),
            sender_avatar_url: item.sender_avatar_url,
            sender_is_verified: item.sender_is_verified,
            sender_relationship: item.sender_relationship,
            sender_app_id: item.sender_app_id,
            sender_is_scam: item.sender_is_scam,
            sender_is_bot: item.sender_is_bot,
            category: item.category,
            content: item.content.unwrap_or_default(),
            status: item.status.into(),
            created_at_micros: item.created_at.timestamp_micros(),
            media_url: item.media_url,
            media_mime_type: item.media_mime_type,
            media_size: item.media_size,
            media_duration: item.media_duration.unwrap_or_default(),
            media_width: item.media_width,
            media_height: item.media_height,
            thumb_image: item.thumb_image,
            media_status: format!("{:?}", item.media_status.unwrap_or_default()).to_uppercase(),
            quote_message_id: item.quote_message_id,
            quote_content: item.quote_content,
            caption: item.caption,
            action: None,
            participant_id: None,
            participant_full_name: None,
            snapshot_id: None,
            snapshot_type: None,
            snapshot_amount: None,
            snapshot_memo: None,
            snapshot_asset_id: None,
            snapshot_asset_symbol: None,
            snapshot_asset_icon_url: None,
            snapshot_chain_icon_url: None,
            snapshot_opponent_id: None,
            snapshot_transaction_hash: None,
            snapshot_created_at: None,
            inscription_hash: None,
            inscription_collection_hash: None,
            inscription_sequence: None,
            inscription_content_type: None,
            inscription_content_url: None,
            inscription_name: None,
            inscription_icon_url: None,
            hyperlink: None,
            media_name: item.media_name,
            album_id: None,
            sticker_id: item.sticker_id,
            shared_user_id: item.shared_user_id,
            media_waveform: item.media_waveform,
            thumb_url: item.thumb_url,
            conversation_owner_id: None,
            conversation_category: None,
            shared_user_full_name: item.shared_user_full_name,
            shared_user_identity_number: item.shared_user_identity_number,
            shared_user_avatar_url: item.shared_user_avatar_url,
            shared_user_is_verified: item.shared_user_is_verified,
            shared_user_app_id: item.shared_user_app_id,
            sticker_asset_url: item.sticker_asset_url,
            sticker_asset_width: item.sticker_asset_width,
            sticker_asset_height: item.sticker_asset_height,
            sticker_asset_name: item.sticker_asset_name,
            sticker_asset_type: item.sticker_asset_type,
            mention_read: None,
            pinned: false,
            expire_in: None,
        }
    }
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
        let signal_database = SignalDatabase::connect(auth.account.identity_number.clone())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        if signal_database
            .identity_dao
            .get_local_identity()
            .await?
            .is_none()
        {
            self.auth_service.clear_auth(&auth.account.user_id).await?;
            return Ok(None);
        }
        let runtime = AccountRuntime::start(auth).await?;
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
            auth_service: self.auth_service.clone(),
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
            auth_service: self.auth_service.clone(),
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
            biography: account.biography.clone(),
            phone: account.phone.clone(),
            created_at: account.created_at.clone(),
        }
    }

    pub async fn user_profile(
        &self,
        user_id: Option<String>,
        identity_number: Option<String>,
    ) -> Result<Option<UserProfileItem>> {
        Ok(self
            .runtime
            .user_profile(user_id.as_deref(), identity_number.as_deref())
            .await?
            .map(|user| UserProfileItem {
                user_id: user.user_id,
                identity_number: user.identity_number,
                full_name: user.full_name,
                avatar_url: user.avatar_url,
                biography: user.biography,
                is_verified: user.is_verified,
                is_bot: user.app_id.is_some_and(|app_id| !app_id.is_empty()),
                relationship: format!("{:?}", user.relationship).to_uppercase(),
            }))
    }

    pub async fn users_by_identity_numbers(
        &self,
        identity_numbers: Vec<String>,
    ) -> Result<Vec<UserProfileItem>> {
        Ok(self
            .runtime
            .users_by_identity_numbers(&identity_numbers)
            .await?
            .into_iter()
            .map(|user| UserProfileItem {
                user_id: user.user_id,
                identity_number: user.identity_number,
                full_name: user.full_name,
                avatar_url: user.avatar_url,
                biography: user.biography,
                is_verified: user.is_verified,
                is_bot: user.app_id.is_some_and(|app_id| !app_id.is_empty()),
                relationship: format!("{:?}", user.relationship).to_uppercase(),
            })
            .collect())
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
                    last_read_message_id: item.last_read_message_id,
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
                    participant_count: item.participant_count,
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
        forward_changes(&self.runtime, sink).await
    }

    pub async fn messages(
        &self,
        conversation_id: String,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<String>,
        limit: i64,
    ) -> Result<Vec<MessageListItem>> {
        Ok(self
            .runtime
            .messages(
                &conversation_id,
                before_created_at_micros,
                before_message_id.as_deref(),
                limit,
            )
            .await?
            .into_iter()
            .map(MessageListItem::from)
            .collect())
    }

    pub async fn messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageListItem>> {
        Ok(self
            .runtime
            .messages_around(&conversation_id, &target_message_id, before, after)
            .await?
            .into_iter()
            .map(MessageListItem::from)
            .collect())
    }

    pub async fn image_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<ImageMessageItem>> {
        Ok(self
            .runtime
            .image_messages_around(
                &conversation_id,
                &target_message_id,
                before,
                after,
            )
            .await?
            .into_iter()
            .map(|item| ImageMessageItem {
                message_id: item.message_id,
                created_at_micros: item.created_at.and_utc().timestamp_micros(),
                media_url: item.media_url,
                media_name: item.media_name,
                can_forward: item.can_forward,
            })
            .collect())
    }

    pub async fn pinned_messages(
        &self,
        conversation_id: String,
    ) -> Result<Vec<MessageListItem>> {
        Ok(self
            .runtime
            .pinned_messages(&conversation_id)
            .await?
            .into_iter()
            .map(MessageListItem::from)
            .collect())
    }

    pub async fn transcript_messages(&self, transcript_id: String) -> Result<Vec<MessageListItem>> {
        Ok(self
            .runtime
            .transcript_messages(&transcript_id)
            .await?
            .into_iter()
            .map(MessageListItem::from)
            .collect())
    }

    pub async fn message_changes(&self, sink: StreamSink<u64>) -> Result<()> {
        forward_changes(&self.runtime, sink).await
    }

    pub async fn current_user_role(&self, conversation_id: String) -> Result<Option<String>> {
        self.runtime.current_user_role(&conversation_id).await
    }

    pub async fn send_text(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
    ) -> Result<String> {
        self.runtime
            .send_text(&conversation_id, &content, quote_message_id.as_deref())
            .await
    }

    pub async fn forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        self.runtime
            .forward_messages(&target_conversation_id, &source_message_ids)
            .await
    }

    pub async fn combine_forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<String> {
        self.runtime
            .combine_forward_messages(&target_conversation_id, &source_message_ids)
            .await
    }

    pub async fn set_message_pinned(
        &self,
        conversation_id: String,
        message_id: String,
        pinned: bool,
    ) -> Result<()> {
        self.runtime
            .set_message_pinned(&conversation_id, &message_id, pinned)
            .await
    }

    pub async fn recall_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<()> {
        self.runtime
            .recall_messages(&conversation_id, &message_ids)
            .await
    }

    pub async fn delete_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<()> {
        self.runtime
            .delete_messages(&conversation_id, &message_ids)
            .await
    }

    pub async fn mark_mention_read(
        &self,
        conversation_id: String,
        message_id: String,
    ) -> Result<()> {
        self.runtime
            .mark_mention_read(&conversation_id, &message_id)
            .await
    }

    pub async fn download_attachment(&self, message_id: String) -> Result<()> {
        self.runtime.download_attachment(&message_id).await
    }

    pub async fn cancel_attachment(&self, message_id: String) -> Result<()> {
        self.runtime.cancel_attachment(&message_id).await
    }

    pub async fn mark_audio_read(&self, message_id: String) -> Result<()> {
        self.runtime.mark_audio_read(&message_id).await
    }

    pub async fn download_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<()> {
        self.runtime
            .download_transcript_attachment(&transcript_id, &message_id)
            .await
    }

    pub async fn cancel_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<()> {
        self.runtime
            .cancel_transcript_attachment(&transcript_id, &message_id)
            .await
    }

    pub async fn mark_transcript_audio_read(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<()> {
        self.runtime
            .mark_transcript_audio_read(&transcript_id, &message_id)
            .await
    }

    pub async fn add_sticker(&self, sticker_id: String) -> Result<()> {
        self.runtime.add_sticker(&sticker_id).await
    }

    pub async fn add_sticker_from_file(&self, message_id: String) -> Result<()> {
        self.runtime.add_sticker_from_file(&message_id).await
    }

    pub async fn add_contact(&self, user_id: String, full_name: String) -> Result<()> {
        self.runtime.add_contact(&user_id, &full_name).await
    }

    pub async fn block_user(&self, user_id: String) -> Result<()> {
        self.runtime.block_user(&user_id).await
    }

    pub async fn bot_home_uri(&self, app_id: String) -> Result<Option<String>> {
        self.runtime.bot_home_uri(&app_id).await
    }

    pub async fn mark_conversation_read(&self, conversation_id: String) -> Result<()> {
        self.runtime.mark_conversation_read(&conversation_id).await
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

    pub async fn sign_out(&self) -> Result<()> {
        self.runtime.begin_sign_out();
        let clear_auth_error = self
            .auth_service
            .clear_auth(self.runtime.account_id())
            .await
            .err();
        self.runtime.sign_out().await;
        if let Some(error) = clear_auth_error {
            warn!("failed to clear local auth after sign out: {error}");
        }
        Ok(())
    }
}

async fn forward_changes(runtime: &AccountRuntime, sink: StreamSink<u64>) -> Result<()> {
    let mut changes = runtime.subscribe_conversation_changes();
    let mut shutdown = runtime.subscribe_shutdown();
    loop {
        if *shutdown.borrow() {
            break;
        }
        tokio::select! {
            result = changes.changed() => {
                if result.is_err() {
                    break;
                }
                let revision = *changes.borrow();
                if sink.add(revision).is_err() {
                    break;
                }
            }
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
    Ok(())
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
