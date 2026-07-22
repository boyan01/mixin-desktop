use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use sdk::{
    CircleConversationRequest, ConversationCategory, ConversationRequest, ParticipantRequest,
};
use uuid::Uuid;

use super::{model, AccountState};
use crate::core::util::generate_conversation_id;

pub struct ConversationAccess {
    state: Arc<AccountState>,
}

impl ConversationAccess {
    pub async fn resolve_code(&self, code: String) -> Result<model::CodeResult> {
        let code = code.trim();
        if code.is_empty() {
            return Err(anyhow!("code is empty"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let value = self.client.account_api.code(code).await?;
        match value.get("type").and_then(serde_json::Value::as_str) {
            Some("user") => {
                let user: sdk::User = serde_json::from_value(value)?;
                let user_id = user.user_id.clone();
                self.database.user_dao.insert_sdk_users(vec![user]).await?;
                self.notify_all_conversations_changed();
                Ok(model::CodeResult {
                    kind: "user".to_string(),
                    user_id: Some(user_id),
                    conversation_id: None,
                    conversation_name: None,
                    participant_count: 0,
                    participant_avatars: Vec::new(),
                    already_member: false,
                    asset_id: None,
                    asset_symbol: None,
                    asset_icon_url: None,
                    chain_icon_url: None,
                    amount: None,
                    senders: Vec::new(),
                    receivers: Vec::new(),
                    threshold: 0,
                    state: None,
                    action: None,
                })
            }
            Some("conversation") => {
                let conversation: sdk::Conversation = serde_json::from_value(value)?;
                let already_member = conversation
                    .participants
                    .iter()
                    .any(|participant| participant.user_id == self.account_id);
                let participant_ids = conversation
                    .participants
                    .iter()
                    .take(4)
                    .map(|participant| participant.user_id.clone())
                    .collect::<Vec<_>>();
                let users = self.client.user_api.get_users(&participant_ids).await?;
                let participant_avatars = users
                    .iter()
                    .map(|user| model::GroupAvatar {
                        user_id: user.user_id.clone(),
                        name: user.full_name.clone(),
                        avatar_url: user.avatar_url.clone(),
                    })
                    .collect();
                self.database.user_dao.insert_sdk_users(users).await?;
                Ok(model::CodeResult {
                    kind: "conversation".to_string(),
                    user_id: None,
                    conversation_id: Some(conversation.conversation_id),
                    conversation_name: Some(conversation.name),
                    participant_count: conversation.participants.len() as i64,
                    participant_avatars,
                    already_member,
                    asset_id: None,
                    asset_symbol: None,
                    asset_icon_url: None,
                    chain_icon_url: None,
                    amount: None,
                    senders: Vec::new(),
                    receivers: Vec::new(),
                    threshold: 0,
                    state: None,
                    action: None,
                })
            }
            Some(kind @ ("payment" | "multisig_request")) => {
                let asset_id = value
                    .get("asset_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| anyhow!("code asset id is missing"))?
                    .to_string();
                let amount = value
                    .get("amount")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| anyhow!("code amount is missing"))?
                    .to_string();
                let string_list = |name: &str| {
                    value
                        .get(name)
                        .and_then(serde_json::Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                };
                let senders = if kind == "payment" {
                    vec![self.account_id.clone()]
                } else {
                    string_list("senders")
                };
                let receivers = string_list("receivers");
                let threshold = value
                    .get("threshold")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or_default();
                let state = value
                    .get(if kind == "payment" { "status" } else { "state" })
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let action = value
                    .get("action")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let asset = self.client.asset_api.get_asset_by_id(&asset_id).await?;
                let chain = self.client.asset_api.get_chain(&asset.chain_id).await?;
                self.database.asset_dao.insert_asset(&asset).await?;
                self.database.asset_dao.insert_chain(&chain).await?;
                let user_ids = senders
                    .iter()
                    .chain(&receivers)
                    .cloned()
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let users = self.client.user_api.get_users(&user_ids).await?;
                let participant_avatars = users
                    .iter()
                    .map(|user| model::GroupAvatar {
                        user_id: user.user_id.clone(),
                        name: user.full_name.clone(),
                        avatar_url: user.avatar_url.clone(),
                    })
                    .collect();
                self.database.user_dao.insert_sdk_users(users).await?;
                Ok(model::CodeResult {
                    kind: kind.to_string(),
                    user_id: None,
                    conversation_id: None,
                    conversation_name: None,
                    participant_count: 0,
                    participant_avatars,
                    already_member: false,
                    asset_id: Some(asset_id),
                    asset_symbol: Some(asset.symbol),
                    asset_icon_url: Some(asset.icon_url),
                    chain_icon_url: Some(chain.icon_url),
                    amount: Some(amount),
                    senders,
                    receivers,
                    threshold,
                    state,
                    action,
                })
            }
            _ => Ok(model::CodeResult {
                kind: "unknown".to_string(),
                user_id: None,
                conversation_id: None,
                conversation_name: None,
                participant_count: 0,
                participant_avatars: Vec::new(),
                already_member: false,
                asset_id: None,
                asset_symbol: None,
                asset_icon_url: None,
                chain_icon_url: None,
                amount: None,
                senders: Vec::new(),
                receivers: Vec::new(),
                threshold: 0,
                state: None,
                action: None,
            }),
        }
    }

    pub async fn join_group(&self, code: String) -> Result<String> {
        let code = code.trim();
        if code.is_empty() {
            return Err(anyhow!("code is empty"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let conversation = self.client.conversation_api.join(code).await?;
        let conversation_id = conversation.conversation_id;
        self.app_service
            .conversation
            .refresh_conversation(&conversation_id)
            .await?;
        self.notify_conversation_changed(&conversation_id);
        Ok(conversation_id)
    }

    pub async fn open_user_conversation(&self, user_id: String) -> Result<String> {
        let user_id = user_id.trim();
        if user_id.is_empty() || user_id == self.account_id {
            return Err(anyhow!("invalid conversation user"));
        }
        let conversation_id = generate_conversation_id(&self.account_id, user_id).to_string();
        if self
            .database
            .conversation_dao
            .find_conversation_by_id(&conversation_id)
            .await?
            .is_some()
        {
            return Ok(conversation_id);
        }

        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user_ids = [user_id.to_string()];
        let users = self.client.user_api.get_users(&user_ids).await?;
        if users.is_empty() {
            return Err(anyhow!("conversation user not found"));
        }
        self.database.user_dao.insert_sdk_users(users).await?;
        self.client
            .conversation_api
            .create_conversation(&ConversationRequest {
                conversation_id: conversation_id.clone(),
                random_id: None,
                category: Some(ConversationCategory::Contact),
                name: None,
                icon_base64: None,
                announcement: None,
                participants: Some(vec![ParticipantRequest {
                    user_id: user_id.to_string(),
                }]),
                duration: None,
            })
            .await?;
        self.app_service
            .conversation
            .refresh_conversation(&conversation_id)
            .await?;
        self.notify_conversation_changed(&conversation_id);
        Ok(conversation_id)
    }

    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }
}

impl Deref for ConversationAccess {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl ConversationAccess {
    pub async fn conversation_items(&self) -> Result<Vec<model::ConversationListData>> {
        Ok(self
            .database
            .conversation_dao
            .all_items()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn conversation_items_by_ids(
        &self,
        conversation_ids: Vec<String>,
    ) -> Result<Vec<model::ConversationListData>> {
        Ok(self
            .database
            .conversation_dao
            .list_items_by_ids(&conversation_ids)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn unseen_conversation_counts(&self) -> Result<Vec<model::ConversationUnseenCount>> {
        Ok(self
            .database
            .conversation_dao
            .unseen_counts()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn is_bot_group(&self, conversation_id: String) -> Result<bool> {
        Ok(self
            .database
            .conversation_dao
            .is_bot_group(&conversation_id)
            .await?)
    }

    pub async fn conversation_count(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
    ) -> Result<i64> {
        let circle_id = circle_id.as_deref();
        let category = category.as_str();
        let keyword = keyword.as_str();
        Ok(self
            .database
            .conversation_dao
            .count_items(category, circle_id, keyword, unseen_only)
            .await?)
    }

    pub async fn conversations(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<model::ConversationListData>> {
        let circle_id = circle_id.as_deref();
        let category = category.as_str();
        let keyword = keyword.as_str();
        Ok(self
            .database
            .conversation_dao
            .list_items(category, circle_id, keyword, unseen_only, limit, offset)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn current_user_role(&self, conversation_id: String) -> Result<Option<String>> {
        let conversation_id = conversation_id.as_str();
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        if conversation.category != Some(ConversationCategory::Group) {
            return Ok(Some("OWNER".to_string()));
        }
        Ok(self
            .database
            .participant_dao
            .find_participant_by_id(conversation_id, &self.account_id)
            .await?
            .and_then(|participant| participant.role))
    }

    pub async fn conversation_participants(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::ConversationParticipantItem>> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .participant_dao
            .list_items(conversation_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_bot_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<model::ConversationParticipantItem>> {
        let conversation_id = conversation_id.as_str();
        let keyword = keyword.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .user_dao
            .search_bot_group_users(&self.account_id, conversation_id, keyword)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<model::ConversationParticipantItem>> {
        let conversation_id = conversation_id.as_str();
        let keyword = keyword.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .user_dao
            .search_group_users(&self.account_id, conversation_id, keyword)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<model::ConversationDetailItem> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        self.app_service
            .conversation
            .refresh_conversation(conversation_id)
            .await?;
        Ok(self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?
            .into())
    }

    pub async fn local_conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<model::ConversationDetailItem> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?
            .into())
    }

    pub async fn groups_in_common(
        &self,
        user_id: String,
    ) -> Result<Vec<model::GroupConversationItem>> {
        let user_id = user_id.as_str();
        self.ensure_active()?;
        let mut result = Vec::new();
        let mut offset = 0;
        loop {
            let page = self
                .conversations(
                    "groups".to_string(),
                    None,
                    String::new(),
                    false,
                    200,
                    offset,
                )
                .await?;
            if page.is_empty() {
                break;
            }
            let page_len = page.len();
            for conversation in page {
                if self
                    .database
                    .participant_dao
                    .find_participant_by_id(&conversation.conversation_id, user_id)
                    .await?
                    .is_some()
                {
                    result.push(model::GroupConversationItem {
                        conversation_id: conversation.conversation_id,
                        name: conversation.name,
                        avatar_url: conversation.avatar_url,
                        participant_count: conversation.participant_count,
                    });
                }
            }
            if page_len < 200 {
                break;
            }
            offset += page_len as i64;
        }
        Ok(result)
    }

    pub async fn update_participants(
        &self,
        conversation_id: String,
        action: String,
        user_ids: Vec<String>,
        role: Option<String>,
    ) -> Result<()> {
        let role = role.as_deref();
        let conversation_id = conversation_id.as_str();
        let action = action.as_str();
        let user_ids = user_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let participants = user_ids
            .iter()
            .map(|user_id| sdk::Participant {
                user_id: user_id.clone(),
                role: role.map(str::to_string),
                created_at: Utc::now(),
            })
            .collect::<Vec<_>>();
        self.client
            .conversation_api
            .update_participants(conversation_id, action, &participants)
            .await?;
        self.app_service
            .conversation
            .refresh_conversation(conversation_id)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn set_disappearing_messages(
        &self,
        conversation_id: String,
        duration: i64,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.client
            .conversation_api
            .disappear(conversation_id, duration)
            .await?;
        self.database
            .conversation_dao
            .update_expire_in(conversation_id, duration)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn create_circle(&self, name: String) -> Result<model::CircleItem> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err(anyhow!("circle name must contain 1 to 64 characters"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let circle = self.client.circle_api.create_circle(name).await?;
        self.database
            .circle_dao
            .insert_circles(std::slice::from_ref(&circle))
            .await?;
        self.notify_all_conversations_changed();
        Ok(circle.into())
    }

    pub async fn update_circle(&self, circle_id: String, name: String) -> Result<()> {
        let circle_id = circle_id.as_str();
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err(anyhow!("circle name must contain 1 to 64 characters"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let circle = self
            .client
            .circle_api
            .update_circle(circle_id, name)
            .await?;
        self.database
            .circle_dao
            .update_name(circle_id, &circle.name)
            .await?;
        self.notify_all_conversations_changed();
        Ok(())
    }

    pub async fn delete_circle(&self, circle_id: String) -> Result<()> {
        let circle_id = circle_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.client.circle_api.delete_circle(circle_id).await?;
        self.database
            .circle_conversation_dao
            .delete_by_circle(circle_id)
            .await?;
        self.database.circle_dao.delete(circle_id).await?;
        self.notify_all_conversations_changed();
        Ok(())
    }

    pub async fn reorder_circles(&self, circle_ids: Vec<String>) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let existing = self.database.circle_dao.list().await?;
        let expected = existing
            .iter()
            .map(|circle| circle.circle_id.as_str())
            .collect::<HashSet<_>>();
        let requested = circle_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if requested.len() != circle_ids.len() || requested != expected {
            return Err(anyhow!(
                "circle order must contain every circle exactly once"
            ));
        }
        self.database.circle_dao.update_orders(&circle_ids).await?;
        self.notify_all_conversations_changed();
        Ok(())
    }

    pub async fn create_group(&self, name: String, user_ids: Vec<String>) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err(anyhow!("group name must contain 1 to 40 characters"));
        }
        let mut user_ids = user_ids
            .into_iter()
            .filter(|user_id| user_id != &self.account_id)
            .collect::<Vec<_>>();
        user_ids.sort();
        user_ids.dedup();
        if user_ids.is_empty() {
            return Err(anyhow!("group requires at least one participant"));
        }
        let random_id = Uuid::new_v4().to_string();
        let conversation_id = group_conversation_id(&self.account_id, name, &user_ids, &random_id);

        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.client
            .conversation_api
            .create_conversation(&ConversationRequest {
                conversation_id: conversation_id.clone(),
                random_id: Some(random_id),
                category: Some(ConversationCategory::Group),
                name: Some(name.to_string()),
                icon_base64: None,
                announcement: None,
                participants: Some(
                    user_ids
                        .into_iter()
                        .map(|user_id| ParticipantRequest { user_id })
                        .collect(),
                ),
                duration: None,
            })
            .await?;
        self.app_service
            .conversation
            .refresh_conversation(&conversation_id)
            .await?;
        self.notify_conversation_changed(&conversation_id);
        Ok(conversation_id)
    }

    pub async fn edit_conversation(
        &self,
        conversation_id: String,
        name: Option<String>,
        announcement: Option<String>,
    ) -> Result<()> {
        let name = name.as_deref();
        let announcement = announcement.as_deref();
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.client
            .conversation_api
            .update(&ConversationRequest {
                conversation_id: conversation_id.to_string(),
                random_id: None,
                category: Some(ConversationCategory::Group),
                name: name.map(str::to_string),
                icon_base64: None,
                announcement: announcement.map(str::to_string),
                participants: None,
                duration: None,
            })
            .await?;
        self.app_service
            .conversation
            .refresh_conversation(conversation_id)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn exit_group(&self, conversation_id: String) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.client.conversation_api.exit(conversation_id).await?;
        self.database
            .conversation_dao
            .delete_local(conversation_id)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn rotate_group_invite(&self, conversation_id: String) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.client.conversation_api.rotate(conversation_id).await?;
        self.app_service
            .conversation
            .refresh_conversation(conversation_id)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn clear_conversation(&self, conversation_id: String) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .message_dao
            .clear_conversation(conversation_id)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn circles(&self) -> Result<Vec<model::CircleItem>> {
        Ok(self
            .database
            .circle_dao
            .summaries()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn set_pinned(&self, conversation_id: String, pinned: bool) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .conversation_dao
            .set_pinned(conversation_id, pinned)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn set_muted(
        &self,
        conversation_id: String,
        owner_id: String,
        category: String,
        duration_seconds: i64,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let owner_id = owner_id.as_str();
        let category = category.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let category = if category == "GROUP" {
            ConversationCategory::Group
        } else {
            ConversationCategory::Contact
        };
        let response = self
            .client
            .conversation_api
            .mute(&ConversationRequest {
                conversation_id: conversation_id.to_string(),
                random_id: None,
                category: Some(category.clone()),
                name: None,
                icon_base64: None,
                announcement: None,
                participants: (category == ConversationCategory::Contact).then(|| {
                    vec![ParticipantRequest {
                        user_id: self.account_id.clone(),
                    }]
                }),
                duration: Some(duration_seconds),
            })
            .await?;
        self.database
            .conversation_dao
            .set_mute_until(
                conversation_id,
                owner_id,
                if category == ConversationCategory::Group {
                    "GROUP"
                } else {
                    "CONTACT"
                },
                response.mute_until,
            )
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn delete_conversation(&self, conversation_id: String) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .conversation_dao
            .delete_local(conversation_id)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn edit_circle_conversation(
        &self,
        circle_id: String,
        conversation_id: String,
        owner_id: String,
        is_group: bool,
        add: bool,
    ) -> Result<()> {
        let circle_id = circle_id.as_str();
        let conversation_id = conversation_id.as_str();
        let owner_id = owner_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user_id = (!is_group).then(|| owner_id.to_string());
        let request = if add {
            CircleConversationRequest::Add {
                conversation_id: conversation_id.to_string(),
                user_id,
            }
        } else {
            CircleConversationRequest::Remove {
                conversation_id: conversation_id.to_string(),
                user_id,
            }
        };
        let result = self
            .client
            .circle_api
            .update_circle_conversation(circle_id, &request)
            .await?;
        if add {
            self.database
                .circle_conversation_dao
                .insert(&[result])
                .await?;
        } else {
            self.database
                .circle_conversation_dao
                .delete(circle_id, conversation_id)
                .await?;
        }
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }
}

fn group_conversation_id(
    owner_id: &str,
    name: &str,
    user_ids: &[String],
    random_id: &str,
) -> String {
    let mut conversation_id = generate_conversation_id(owner_id, name).to_string();
    conversation_id = generate_conversation_id(&conversation_id, random_id).to_string();
    let mut sorted_user_ids = user_ids.to_vec();
    sorted_user_ids.sort();
    for user_id in sorted_user_ids {
        conversation_id = generate_conversation_id(&conversation_id, &user_id).to_string();
    }
    conversation_id
}

#[cfg(test)]
mod tests {
    use super::group_conversation_id;

    #[test]
    fn group_id_matches_flutter() {
        let user_ids = [
            "f937ca18-d1ff-46f5-99e8-e23fbd6fd5f2",
            "0e0a20c8-31b8-4093-81b8-9cebd9bc8afc",
            "8391e472-cdbe-4704-be1f-7d184635b885",
            "831fdb67-13ed-4dc5-ac64-dda89aeda2bb",
            "f7ff9dde-18c2-4375-8097-b364068b120e",
            "088c1e3e-1f07-4065-85b5-6b49b4370d32",
        ]
        .map(str::to_string);
        assert_eq!(
            group_conversation_id(
                "c8cb0ac7-d456-4341-be66-0b143aa09922",
                "Mixin Rocks",
                &user_ids,
                "01d21e2c-76f5-4940-8ea0-9b7f21728674",
            ),
            "5dac944e-2037-31b4-bbd9-e5fd3ffe571e"
        );
    }
}
