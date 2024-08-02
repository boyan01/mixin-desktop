use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Error;

use sdk::Client;

use crate::core::model::ConversationService;
use crate::db::MixinDatabase;

pub struct CircleService {
    pub(crate) db: Arc<MixinDatabase>,
    pub(crate) client: Arc<Client>,
    pub(crate) conversation: ConversationService,
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
        Ok(())
    }

    pub async fn refresh_circle(&self, cid: &str) -> Result<(), Error> {
        let circle = self.client.circle_api.get_circle(cid).await?;
        self.db.circle_dao.insert_circles(&[circle]).await?;
        let user_ids = self.update_circle_conversations(cid).await?;
        let user_ids = user_ids.into_iter().collect::<Vec<_>>();
        self.conversation.refresh_user(&user_ids, false).await?;
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
}
