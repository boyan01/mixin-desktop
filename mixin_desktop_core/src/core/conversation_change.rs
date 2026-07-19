use tokio::sync::{broadcast, watch};

#[derive(Clone, Debug)]
pub enum ConversationChange {
    Conversation(String),
    All,
}

#[derive(Clone)]
pub struct ConversationChangeNotifier {
    revision: watch::Sender<u64>,
    events: broadcast::Sender<ConversationChange>,
}

impl ConversationChangeNotifier {
    pub fn new() -> Self {
        let (revision, _) = watch::channel(0);
        let (events, _) = broadcast::channel(256);
        Self { revision, events }
    }

    pub fn notify(&self, conversation_id: impl Into<String>) {
        self.bump_revision();
        let _ = self
            .events
            .send(ConversationChange::Conversation(conversation_id.into()));
    }

    pub fn notify_all(&self) {
        self.bump_revision();
        let _ = self.events.send(ConversationChange::All);
    }

    pub fn notify_messages(&self) {
        self.bump_revision();
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<ConversationChange> {
        self.events.subscribe()
    }

    pub fn subscribe_revision(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }

    fn bump_revision(&self) {
        self.revision
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }
}

impl Default for ConversationChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}
