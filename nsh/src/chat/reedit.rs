//! Session-only recovery of text after a local, single-message recall.

use std::{collections::HashMap, time::Duration};

use nsh_core::prelude::*;

/// An editing intent retained briefly, never an alternate message store.
#[derive(Clone, Copy)]
pub(crate) struct Reedit {
    owner: Owner,
    recalled: Signal<HashMap<String, (String, String)>>,
    request: Signal<Option<String>>,
}

impl Reedit {
    pub(crate) fn mount() -> Self {
        Self {
            owner: Owner::current(),
            recalled: signal(HashMap::new()),
            request: signal(None),
        }
    }

    pub(crate) fn clear(self) {
        batch(|| {
            self.recalled.set(HashMap::new());
            self.request.set(None);
        });
    }

    pub(super) fn remember(self, id: String, conversation_id: String, text: String) {
        if !self.owner.is_live() {
            return;
        }
        self.recalled.update(|rows| {
            rows.insert(id.clone(), (conversation_id, text));
        });
        // The session owns expiry, so changing chats cannot retain text forever
        // or remove the re-edit action before the six-minute window ends.
        self.owner.run(|| {
            set_timeout(Duration::from_secs(6 * 60), move || {
                self.recalled.update(|rows| {
                    rows.remove(&id);
                });
                if self
                    .request
                    .with_untracked(|request| request.as_deref() == Some(&id))
                {
                    self.request.set(None);
                }
            });
        });
    }

    pub(super) fn available(self, id: &str) -> bool {
        self.recalled.with(|rows| rows.contains_key(id))
    }

    pub(super) fn request(self, id: &str) {
        if self
            .recalled
            .with_untracked(|rows| rows.get(id).is_some_and(|(_, text)| !text.is_empty()))
        {
            self.request.set(Some(id.to_owned()));
        }
    }

    pub(crate) fn take_request(self, conversation_id: &str) -> Option<String> {
        let id = self.request.get()?;
        let recalled = self.recalled.with_untracked(|rows| rows.get(&id).cloned());
        match recalled {
            Some((conversation, text)) if conversation == conversation_id => {
                self.request.set(None);
                Some(text)
            }
            Some(_) => None,
            None => {
                self.request.set(None);
                None
            }
        }
    }
}
