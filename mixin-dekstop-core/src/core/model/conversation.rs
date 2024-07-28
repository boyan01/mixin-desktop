use std::sync::Arc;

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
    pub async fn refresh_user(&self) -> anyhow::Result<()> {
        todo!()
    }
}
