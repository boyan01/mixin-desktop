use std::sync::Arc;

pub use circle::CircleService;
pub use conversation::ConversationService;
pub use message::*;
use sdk::Client;

use crate::core::message::sender::MessageSender;
use crate::core::model::job::JobService;
use crate::db::MixinDatabase;

pub mod auth;
pub mod circle;
pub mod conversation;
pub mod job;
pub mod message;

pub struct AppService {
    pub conversation: ConversationService,
    pub circle: CircleService,
    pub message: MessageService,
    pub job: JobService,
}

impl AppService {
    pub fn new(
        db: Arc<MixinDatabase>,
        client: Arc<Client>,
        account_id: String,
        primary_session_id: Option<String>,
        message_sender: Arc<MessageSender>,
    ) -> Self {
        let conversation = ConversationService::new(db.clone(), client.clone(), account_id.clone());
        AppService {
            circle: CircleService {
                db: db.clone(),
                client: client.clone(),
                conversation: conversation.clone(),
            },
            conversation,
            message: MessageService::new(db.clone()),
            job: JobService::new(
                db,
                message_sender,
                client.clone(),
                account_id,
                primary_session_id,
            ),
        }
    }
}
