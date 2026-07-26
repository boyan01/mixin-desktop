use mixin_desktop_api::ConversationListItem;

#[derive(uniffi::Record)]
pub struct SwiftConversationListItem {
    pub conversation_id: String,
    pub name: String,
    pub icon_url: String,
    pub last_message: String,
    pub unseen_count: i64,
    pub mention_count: i64,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub updated_at_millis: i64,
}

impl From<ConversationListItem> for SwiftConversationListItem {
    fn from(value: ConversationListItem) -> Self {
        Self {
            conversation_id: value.conversation_id,
            name: value.name,
            icon_url: value.icon_url,
            last_message: value.last_message,
            unseen_count: value.unseen_count,
            mention_count: value.mention_count,
            is_pinned: value.is_pinned,
            is_muted: value.is_muted,
            updated_at_millis: value.updated_at_millis,
        }
    }
}
