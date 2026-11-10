use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Error;

use sdk::Client;

use crate::core::conversation_change::ConversationChangeNotifier;
use crate::core::model::ConversationService;
use crate::db::MixinDatabase;

pub struct CircleService {
    pub(crate) db: Arc<MixinDatabase>,
    pub(crate) client: Arc<Client>,
    pub(crate) conversation: ConversationService,
    pub(crate) changes: Option<ConversationChangeNotifier>,
}

impl CircleService {
    pub async fn refresh_circles(&self) -> Result<(), Error> {
        let circles = self.client.circle_api.get_circles().await?;
        self.db.circle_dao.insert_circles(&circles).await?;
        let mut user_ids = HashSet::new();
        for circle in circles {
            let ids = self.update_circle_conversations(&circle.circle_id).await?;
            user_ids.extend(ids);
        }
        let user_ids = user_ids.into_iter().collect::<Vec<_>>();
        self.conversation.refresh_user(&user_ids, false).await?;
        self.notify_changes();
        Ok(())
    }

    /// Circles otherwise arrive only through SYSTEM_CIRCLE, so a new sign-in
    /// must fetch the existing ones once.
    pub async fn refresh_circles_if_empty(&self) -> Result<(), Error> {
        if self.db.circle_dao.summaries().await?.is_empty() {
            self.refresh_circles().await?;
        }
        Ok(())
    }

    pub async fn refresh_circle(&self, cid: &str) -> Result<(), Error> {
        let circle = self.client.circle_api.get_circle(cid).await?;
        self.db.circle_dao.insert_circles(&[circle]).await?;
        let user_ids = self.update_circle_conversations(cid).await?;
        let user_ids = user_ids.into_iter().collect::<Vec<_>>();
        self.conversation.refresh_user(&user_ids, false).await?;
        self.notify_changes();
        Ok(())
    }

    pub async fn sync_circle(&self, cid: &str) -> Result<(), Error> {
        if self.db.circle_dao.exists(cid).await? {
            return Ok(());
        }
        self.refresh_circle(cid).await?;
        Ok(())
    }

    async fn update_circle_conversations(&self, cid: &str) -> Result<HashSet<String>, Error> {
        let conversations = self
            .client
            .circle_api
            .get_circle_conversations(cid, None, None)
            .await?;
        self.db
            .circle_conversation_dao
            .insert(&conversations)
            .await?;
        Ok(conversations
            .into_iter()
            .filter_map(|c| c.user_id)
            .collect())
    }

    fn notify_changes(&self) {
        if let Some(changes) = &self.changes {
            changes.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn startup_circle_fetch_runs_only_while_no_circle_is_stored() {
        let directory = tempfile::tempdir().unwrap();
        let db = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        // Invalid signing material fails any API call before network I/O, so
        // an error here means a fetch was attempted.
        let client = Arc::new(Client::new(sdk::Credential::KeyStore(sdk::KeyStore {
            app_id: "test".into(),
            session_id: "test".into(),
            server_public_key: String::new(),
            session_private_key: "invalid".into(),
            scp: String::new(),
        })));
        let service = CircleService {
            db: db.clone(),
            client: client.clone(),
            conversation: ConversationService::new(db.clone(), client, "me".into()),
            changes: None,
        };

        assert!(service.refresh_circles_if_empty().await.is_err());

        db.circle_dao
            .insert_circles(&[sdk::Circle {
                circle_id: "circle".into(),
                name: "Work".into(),
                created_at: chrono::Utc::now(),
            }])
            .await
            .unwrap();
        service.refresh_circles_if_empty().await.unwrap();
        db.close().await;
    }
}
