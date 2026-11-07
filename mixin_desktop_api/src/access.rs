//! Account-scoped public handles, organized by their owning business feature.

mod attachment;
mod conversation;
mod message;
mod sticker;
mod user;

pub use crate::dto::*;
pub use attachment::AttachmentAccess;
pub use conversation::ConversationAccess;
pub use message::MessageAccess;
pub use sticker::StickerAccess;
pub use user::UserAccess;
