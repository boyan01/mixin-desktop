use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::core::conversation_change::ConversationChangeNotifier;
use tokio::sync::{Mutex, Notify};

use anyhow::bail;

use sdk::client::Client;
use sdk::{ConversationCategory, ConversationRequest, ParticipantRequest, UserSession};

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
    pending_users: Arc<Mutex<HashSet<String>>>,
    user_refresh: Arc<Notify>,
}

impl ConversationService {
    pub fn new(db: Arc<MixinDatabase>, client: Arc<Client>, account_id: String) -> Self {
        ConversationService {
            db,
            client,
            account_id,
            pending_users: Arc::new(Mutex::new(HashSet::new())),
            user_refresh: Arc::new(Notify::new()),
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
        Ok(users.into_iter().chain(updated_users).collect())
    }

    /// Profile metadata is optional for receiving messages. Never wait for HTTP here.
    pub async fn cached_users_or_enqueue(
        &self,
        ids: &[String],
        force: bool,
    ) -> anyhow::Result<Vec<User>> {
        let users = self.db.user_dao.find_users(ids).await?;
        let existing = users
            .iter()
            .map(|user| &user.user_id)
            .collect::<HashSet<_>>();
        let mut pending = self.pending_users.lock().await;
        pending.extend(
            ids.iter()
                .filter(|id| !id.is_empty() && (force || !existing.contains(id)))
                .cloned(),
        );
        if !pending.is_empty() {
            self.user_refresh.notify_one();
        }
        Ok(users)
    }

    #[cfg(test)]
    pub(crate) async fn pending_user_count(&self) -> usize {
        self.pending_users.lock().await.len()
    }

    /// Polled alongside the other account services; dropping the account cancels HTTP and retries.
    pub(crate) async fn run_user_refresh(&self, changes: ConversationChangeNotifier) {
        // Reconstruct missing cache entries after restart, outside the startup readiness path.
        match self.db.user_dao.missing_referenced_user_ids().await {
            Ok(ids) => self.pending_users.lock().await.extend(ids),
            Err(error) => log::warn!("failed to recover missing user profiles: {error}"),
        }
        self.refresh_pending_users(changes, |ids| async move {
            self.client
                .user_api
                .get_users(&ids)
                .await
                .map_err(Into::into)
        })
        .await;
    }

    async fn refresh_pending_users<F, Fut>(&self, changes: ConversationChangeNotifier, mut fetch: F)
    where
        F: FnMut(Vec<String>) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<Vec<sdk::User>>>,
    {
        let mut retry_seconds = 1;
        loop {
            let ids = self.pending_users.lock().await.drain().collect::<Vec<_>>();
            if ids.is_empty() {
                self.user_refresh.notified().await;
                continue;
            }
            let mut retry = false;
            for (index, batch) in ids.chunks(100).enumerate() {
                let result = match fetch(batch.to_vec()).await {
                    Ok(users) => self
                        .db
                        .user_dao
                        .insert_sdk_users(users)
                        .await
                        .map_err(Into::into),
                    Err(error) => Err(error),
                };
                match result {
                    Ok(users) => {
                        let fetched = users
                            .iter()
                            .map(|user| &user.user_id)
                            .collect::<HashSet<_>>();
                        let missing = batch
                            .iter()
                            .filter(|id| !fetched.contains(id))
                            .cloned()
                            .collect::<Vec<_>>();
                        retry |= !missing.is_empty();
                        self.pending_users.lock().await.extend(missing);
                        if !users.is_empty() {
                            changes.notify_all();
                        }
                    }
                    Err(error) => {
                        // Retain both this batch and the unattempted part of the wave.
                        self.pending_users
                            .lock()
                            .await
                            .extend(ids[index * 100..].iter().cloned());
                        log::warn!("failed to refresh user profiles: {error}");
                        retry = true;
                        break;
                    }
                }
            }
            if retry {
                // Incoming messages cannot bypass backoff; omitted users cannot starve later batches.
                tokio::time::sleep(Duration::from_secs(retry_seconds)).await;
                retry_seconds = (retry_seconds * 2).min(60);
            } else {
                retry_seconds = 1;
            }
        }
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
            self.cached_users_or_enqueue(&[owner_id.to_string()], false)
                .await?;
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
        self.cached_users_or_enqueue(&uids, false).await?;
        self.refresh_participants(conversation_id, &participants, c.participant_sessions)
            .await?;
        Ok(())
    }

    pub async fn ensure_conversation_exists(
        &self,
        conversation: &Conversation,
    ) -> anyhow::Result<()> {
        if conversation.status == ConversationStatus::SUCCESS {
            return Ok(());
        }
        let owner_id = conversation
            .owner_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("conversation has no owner"))?;
        let response = self
            .client
            .conversation_api
            .create_conversation(&ConversationRequest {
                conversation_id: conversation.conversation_id.clone(),
                random_id: None,
                category: conversation.category.clone(),
                name: None,
                icon_base64: None,
                announcement: None,
                participants: Some(vec![ParticipantRequest {
                    user_id: owner_id.clone(),
                }]),
                duration: None,
            })
            .await?;

        self.db
            .conversation_dao
            .update_status(&conversation.conversation_id, ConversationStatus::SUCCESS)
            .await?;
        if let Some(sessions) = response.participant_sessions {
            if !sessions.is_empty() {
                let sessions = sessions
                    .into_iter()
                    .map(|session| ParticipantSession {
                        conversation_id: conversation.conversation_id.clone(),
                        user_id: session.user_id,
                        session_id: session.session_id,
                        sent_to_server: None,
                        created_at: None,
                        public_key: session.public_key,
                    })
                    .collect::<Vec<_>>();
                self.db
                    .participant_session_dao
                    .replace_all(&conversation.conversation_id, &sessions)
                    .await?;
            }
        }
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
mod profile_refresh_tests {
    use super::*;
    use tokio::sync::{mpsc, oneshot};

    #[tokio::test]
    async fn failed_profile_batch_retries_without_blocking_enqueue_and_publishes_persisted_users() {
        let directory = tempfile::tempdir().unwrap();
        let database = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        let service = ConversationService::new(
            database.clone(),
            Arc::new(Client::new(sdk::Credential::None)),
            "me".into(),
        );
        let ids = (0..205)
            .map(|index| format!("user-{index}"))
            .collect::<Vec<_>>();
        assert!(service
            .cached_users_or_enqueue(&ids, false)
            .await
            .unwrap()
            .is_empty());
        service.cached_users_or_enqueue(&ids, false).await.unwrap();
        assert_eq!(service.pending_user_count().await, 205);
        let changes = ConversationChangeNotifier::new();
        let mut events = changes.subscribe_events();
        let (requests, mut received) = mpsc::unbounded_channel::<(
            Vec<String>,
            oneshot::Sender<anyhow::Result<Vec<sdk::User>>>,
        )>();
        let worker_service = service.clone();
        let worker = tokio::spawn(async move {
            worker_service
                .refresh_pending_users(changes, |ids| {
                    let (reply, response) = oneshot::channel();
                    requests.send((ids, reply)).unwrap();
                    async move { response.await.unwrap() }
                })
                .await;
        });
        let (first, reply) = received.recv().await.unwrap();
        assert_eq!(first.len(), 100);
        // This includes IDs already in flight, exercising deduplication during failure.
        service
            .cached_users_or_enqueue(&first, false)
            .await
            .unwrap();
        tokio::time::pause();
        let failed_at = tokio::time::Instant::now();
        reply
            .send(Err(anyhow::anyhow!("request timed out")))
            .unwrap();
        let mut fetched = HashSet::new();
        for batch_index in 0..3 {
            let (batch, reply) = received.recv().await.unwrap();
            if batch_index == 0 {
                assert!(failed_at.elapsed() >= Duration::from_secs(1));
                tokio::time::resume();
            }
            assert!(batch.len() <= 100);
            for id in &batch {
                assert!(
                    fetched.insert(id.clone()),
                    "duplicate profile fetch after retry"
                );
            }
            let users = batch
                .iter()
                .map(|id| {
                    serde_json::from_value(serde_json::json!({
                "user_id": id, "identity_number":"1", "biography":"", "full_name":"Fetched profile",
                "is_verified":false, "is_scam":false
            })).unwrap()
                })
                .collect();
            reply.send(Ok(users)).unwrap();
            assert!(matches!(
                events.recv().await.unwrap(),
                crate::core::conversation_change::ConversationChange::All
            ));
            assert_eq!(
                database.user_dao.find_users(&batch).await.unwrap().len(),
                batch.len()
            );
        }
        assert_eq!(fetched, ids.into_iter().collect());
        assert_eq!(service.pending_user_count().await, 0);
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        database.close().await;
    }
}
