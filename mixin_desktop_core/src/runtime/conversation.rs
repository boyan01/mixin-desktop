use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use sdk::{
    CircleConversationRequest, ConversationCategory, ConversationRequest, ParticipantRequest,
};

use super::{model, AccountState};

pub struct ConversationAccess {
    state: Arc<AccountState>,
}

impl ConversationAccess {
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn create_circle(&self, name: String) -> Result<model::CircleItem> {
        let name = name.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let circle = self.client.circle_api.create_circle(name).await?;
        self.database
            .circle_dao
            .insert_circles(std::slice::from_ref(&circle))
            .await?;
        self.notify_conversation_changed();
        Ok(circle.into())
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
        Ok(())
    }
}
