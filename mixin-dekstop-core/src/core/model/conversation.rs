use std::collections::HashSet;
use std::sync::Arc;

use anyhow::bail;

use sdk::client::Client;
use sdk::{ConversationCategory, UserSession};

use crate::db::mixin::conversation::{Conversation, ConversationStatus};
use crate::db::mixin::participant::Participant;
use crate::db::mixin::participant_session::ParticipantSession;
use crate::db::mixin::user::User;
use crate::db::MixinDatabase;

#[derive(Clone)]
pub struct ConversationService {
    db: Arc<MixinDatabase>,
    client: Arc<Client>,
    account_id: String,
}

impl ConversationService {
    pub fn new(db: Arc<MixinDatabase>, client: Arc<Client>, account_id: String) -> Self {
        ConversationService {
            db,
            client,
            account_id,
        }
    }
}

impl ConversationService {
    pub async fn refresh_user(&self, ids: &[String], force: bool) -> anyhow::Result<Vec<User>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        if force {
            return self.update_users(ids).await;
        }

        let users = self.db.user_dao.find_users(ids).await?;
        let exists_user_ids = users.iter().map(|e| &e.user_id).collect::<HashSet<_>>();
        let query_user_ids = ids
            .iter()
            .filter(|id| !exists_user_ids.contains(id))
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        if query_user_ids.is_empty() {
            return Ok(users);
        }
        let updated_users = self.update_users(&query_user_ids).await?;
        Ok(users.into_iter().chain(updated_users.into_iter()).collect())
    }

    async fn update_users(&self, ids: &[String]) -> anyhow::Result<Vec<User>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let response = self.client.user_api.get_users(ids).await?;
        let users = self.db.user_dao.insert_sdk_users(response).await?;
        Ok(users)
    }

    pub async fn sync_conversation(&self, conversation_id: &str) -> anyhow::Result<()> {
        if conversation_id.is_empty()
            || conversation_id == sdk::SYSTEM_USER
            || conversation_id == self.account_id
        {
            return Ok(());
        }
        let conversation = self
            .db
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?;
        if conversation.is_none() {
            self.refresh_conversation(conversation_id).await?
        }
        Ok(())
    }

    pub async fn refresh_conversation(&self, conversation_id: &str) -> anyhow::Result<()> {
        let c = self
            .client
            .conversation_api
            .get_conversation(conversation_id)
            .await?;
        let mut owner_id = &c.creator_id;

        let status = if c.participants.iter().any(|p| p.user_id == self.account_id) {
            ConversationStatus::SUCCESS
        } else {
            ConversationStatus::FAILURE
        };

        if c.category == Some(ConversationCategory::Contact) {
            if status == ConversationStatus::FAILURE {
                bail!("conversation is not legal");
            }
            for p in c.participants.iter() {
                if p.user_id != self.account_id {
                    owner_id = &p.user_id
                }
            }
        } else if c.category == Some(ConversationCategory::Group) {
            self.refresh_user(&[owner_id.to_string()], false).await?;
        }

        self.db
            .conversation_dao
            .insert(&Conversation {
                conversation_id: c.conversation_id.clone(),
                owner_id: Some(owner_id.to_string()),
                category: c.category,
                name: c.name,
                icon_url: c.icon_url,
                announcement: c.announcement,
                code_url: c.code_url,
                created_at: c.created_at,
                status,
                mute_until: c.mute_until,
                expire_in: c.expire_in,
            })
            .await?;

        let mut uids = vec![];
        let mut participants = vec![];
        for p in c.participants {
            uids.push(p.user_id.clone());
            participants.push(Participant {
                conversation_id: c.conversation_id.clone(),
                user_id: p.user_id,
                role: p.role,
                created_at: p.created_at,
            });
        }
        self.refresh_user(&uids, false).await?;
        self.refresh_participants(conversation_id, &participants, c.participant_sessions)
            .await?;
        Ok(())
    }

    async fn refresh_participants(
        &self,
        conversation_id: &str,
        participants: &[Participant],
        sessions: Option<Vec<UserSession>>,
    ) -> anyhow::Result<()> {
        self.db
            .participant_dao
            .replace_all(conversation_id, participants)
            .await?;
        if let Some(sessions) = sessions {
            let sessions = sessions
                .into_iter()
                .map(|s| ParticipantSession {
                    conversation_id: conversation_id.to_string(),
                    user_id: s.user_id,
                    session_id: s.session_id,
                    sent_to_server: None,
                    created_at: None,
                    public_key: s.public_key,
                })
                .collect::<Vec<_>>();
            self.db
                .participant_session_dao
                .replace_all(conversation_id, &sessions)
                .await?;
        }
        Ok(())
    }

    pub async fn refresh_session(
        &self,
        conversation_id: &str,
        user_ids: &[String],
    ) -> anyhow::Result<()> {
        let sessions = self.client.user_api.get_sessions(user_ids).await?;
        if !sessions.is_empty() {
            self.db
                .participant_session_dao
                .insert(conversation_id, &sessions)
                .await?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test() {
        let client = crate::tests::new_test_client().await;
        let db = crate::tests::new_test_mixin_db().await;
        let service = ConversationService::new(
            db,
            client,
            "cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string(),
        );
        service
            .update_users(&["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string()])
            .await
            .expect("failed to update users");
    }

    #[tokio::test]
    async fn test_refresh() {
        let client = crate::tests::new_test_client().await;
        let db = crate::tests::new_test_mixin_db().await;
        let service = ConversationService::new(
            db.clone(),
            client,
            "cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string(),
        );
        let a = db
            .user_dao
            .find_users(&[
                "cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string(),
                "cfb018b0-eaf7-40ec-9e07-28a5158f1261".to_string(),
            ])
            .await
            .expect("failed to update users");
        println!("{:?}", a);
        service
            .refresh_user(&["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string()], false)
            .await
            .expect("failed to update users");
    }
}
