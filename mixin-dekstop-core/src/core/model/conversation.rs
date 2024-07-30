use std::sync::Arc;
use anyhow::anyhow;
use crate::db::MixinDatabase;
use crate::sdk::client::Client;

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
    pub async fn refresh_user(&self, ids: Vec<String>, force: bool) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(()); 
        }
        
        Ok(())
    }
    
    pub async fn update_users(&self, ids: &Vec<String>) -> anyhow::Result<()> {
        if ids.is_empty() {
            return Ok(()); 
        }
        let response = self.client.user_api.get_users(ids).await?;
        self.db.user_dao.find_user();
        Ok(())
    }
}
