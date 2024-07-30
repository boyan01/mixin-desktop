use std::sync::Arc;

pub use conversation::ConversationService;
use sdk::Client;

use crate::db::MixinDatabase;

mod conversation;

pub struct AppService {
    pub conversation: ConversationService,
}

impl AppService {
    pub fn new(db: Arc<MixinDatabase>, client: Arc<Client>) -> Self {
        AppService { conversation: ConversationService::new(db.clone(), client.clone()) }
    }
}
