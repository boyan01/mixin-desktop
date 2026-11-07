use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum ConversationChange {
    Conversation(String),
    /// Message rows or attachment state changed; the conversation list is unaffected.
    /// `None` means the owning conversation is unknown.
    Messages(Option<String>),
    All,
}

impl ConversationChange {
    /// Whether this change can affect messages of `conversation_id` (`None` = any).
    pub fn affects_messages(&self, conversation_id: Option<&str>) -> bool {
        match self {
            Self::Conversation(id) | Self::Messages(Some(id)) => {
                conversation_id.is_none_or(|expected| expected == id)
            }
            Self::Messages(None) | Self::All => true,
        }
    }
}

#[derive(Clone)]
pub struct ConversationChangeNotifier {
    events: broadcast::Sender<ConversationChange>,
}

impl ConversationChangeNotifier {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        Self { events }
    }

    pub fn notify(&self, conversation_id: impl Into<String>) {
        let _ = self
            .events
            .send(ConversationChange::Conversation(conversation_id.into()));
    }

    pub fn notify_all(&self) {
        let _ = self.events.send(ConversationChange::All);
    }

    pub fn notify_messages(&self, conversation_id: Option<&str>) {
        let _ = self.events.send(ConversationChange::Messages(
            conversation_id.map(str::to_owned),
        ));
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<ConversationChange> {
        self.events.subscribe()
    }
}

impl Default for ConversationChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ConversationChange::*;

    #[test]
    fn message_change_in_another_conversation_is_ignored_but_unknown_scope_wakes_everyone() {
        assert!(Messages(Some("a".into())).affects_messages(Some("a")));
        assert!(!Messages(Some("b".into())).affects_messages(Some("a")));
        assert!(!Conversation("b".into()).affects_messages(Some("a")));
        assert!(Conversation("b".into()).affects_messages(None));
        assert!(Messages(None).affects_messages(Some("a")));
        assert!(All.affects_messages(Some("a")));
    }
}
