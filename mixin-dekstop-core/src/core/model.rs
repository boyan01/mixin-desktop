use std::sync::Arc;

pub use circle::CircleService;
pub use conversation::ConversationService;
pub use message::*;
use sdk::Client;

use crate::db::MixinDatabase;

pub mod circle;
pub mod conversation;
pub mod message;

pub struct AppService {
    pub conversation: ConversationService,
    pub circle: CircleService,
    pub message: MessageService,
}

impl AppService {
    pub fn new(db: Arc<MixinDatabase>, client: Arc<Client>, account_id: String) -> Self {
        let conversation = ConversationService::new(db.clone(), client.clone(), account_id.clone());
        AppService {
            circle: CircleService {
                db: db.clone(),
                client,
                conversation: conversation.clone(),
            },
            conversation,
            message: MessageService::new(db.clone()),
        }
    }
}
