use std::sync::Arc;

pub use circle::CircleService;
pub use conversation::ConversationService;
pub use message::*;
use sdk::Client;
use tokio::sync::watch;

use crate::core::attachment::AttachmentService;
use crate::core::message::sender::MessageSender;
use crate::core::model::job::JobService;
use crate::db::app::Auth;
use crate::db::MixinDatabase;

pub mod auth;
pub mod circle;
pub mod conversation;
pub mod expired_message;
pub mod job;
pub mod message;
pub mod signal;

pub struct AppService {
    pub conversation: ConversationService,
    pub circle: CircleService,
    pub message: MessageService,
    pub job: JobService,
    pub attachment: Arc<AttachmentService>,
    pub(crate) expired_message: expired_message::ExpiredMessageService,
}

impl AppService {
    pub fn new(
        db: Arc<MixinDatabase>,
        client: Arc<Client>,
        auth: &Auth,
        message_sender: Arc<MessageSender>,
        attachment: Arc<AttachmentService>,
        changes: Option<watch::Sender<u64>>,
    ) -> Self {
        let account_id = auth.account.user_id.clone();
        let conversation = ConversationService::new(db.clone(), client.clone(), account_id.clone());
        let expired_message =
            expired_message::ExpiredMessageService::new(db.clone(), changes.clone());
        let expired_message_notify = expired_message.notifier();
        AppService {
            circle: CircleService {
                db: db.clone(),
                client: client.clone(),
                conversation: conversation.clone(),
            },
            conversation,
            message: MessageService::new(db.clone(), account_id.clone()),
            expired_message,
            attachment,
            job: JobService::new(
                db,
                message_sender,
                client.clone(),
                auth,
                changes,
                expired_message_notify,
            ),
        }
    }
}
