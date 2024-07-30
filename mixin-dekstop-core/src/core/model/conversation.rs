use std::collections::HashSet;
use std::sync::Arc;

use sdk::client::Client;

use crate::db::MixinDatabase;

pub struct ConversationService {
    db: Arc<MixinDatabase>,
    client: Arc<Client>,
}

impl ConversationService {
    pub fn new(db: Arc<MixinDatabase>, client: Arc<Client>) -> Self {
        ConversationService { db, client }
    }
}

impl ConversationService {
    pub async fn refresh_user(&self, ids: &[String], force: bool) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(());
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
            return Ok(());
        }
        self.update_users(&query_user_ids).await
    }

    pub async fn update_users(&self, ids: &[String]) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let response = self.client.user_api.get_users(ids).await?;
        self.db.user_dao.insert_sdk_users(response).await?;
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
        let service = ConversationService::new(db, client);
        service.update_users(&["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string()]).await.expect("failed to update users");
    }

    #[tokio::test]
    async fn test_refresh() {
        let client = crate::tests::new_test_client().await;
        let db = crate::tests::new_test_mixin_db().await;
        let service = ConversationService::new(db.clone(), client);
        let a = db.user_dao.find_users(&["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string(), "cfb018b0-eaf7-40ec-9e07-28a5158f1261".to_string()]).await.expect("failed to update users");
        println!("{:?}", a);
        service.refresh_user(&["cfb018b0-eaf7-40ec-9e07-28a5158f1269".to_string()], false).await.expect("failed to update users");
    }
}
