//! Document authority is native-only and survives navigation and process restart.
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

#[derive(Clone, PartialEq, Eq)]
struct Document {
    epoch: String,
    token: u64,
}

#[derive(Default)]
pub(super) struct Gate {
    pub generation: Cell<u64>,
    pub committed: Cell<bool>,
    pub pending: RefCell<HashSet<String>>,
    announced: RefCell<Option<Document>>,
    prior: RefCell<Option<Document>>,
    last_ready: RefCell<Option<Document>>,
    active: RefCell<Option<Document>>,
    retired: RefCell<HashSet<String>>,
}

impl Gate {
    pub fn begin_navigation(&self, generation: u64) {
        *self.prior.borrow_mut() = self.last_ready.borrow().clone();
        self.invalidate(generation);
    }

    pub fn invalidate(&self, generation: u64) {
        self.generation.set(generation);
        self.committed.set(false);
        self.active.borrow_mut().take();
        self.pending.borrow_mut().clear();
    }

    pub fn terminate(&self, generation: u64) -> bool {
        if let Some(document) = self.announced.borrow_mut().take() {
            self.retired.borrow_mut().insert(document.epoch);
        }
        self.prior.borrow_mut().take();
        self.last_ready.borrow_mut().take();
        self.invalidate(generation);
        // A repeatedly crashing page must be reopened after this bounded history.
        self.retired.borrow().len() <= 64
    }

    pub fn announce(&self, epoch: &str, token: u64) -> bool {
        if epoch.len() != 36 || token == 0 || self.retired.borrow().contains(epoch) {
            return false;
        }
        let announced = self.announced.borrow();
        if let Some(old) = announced.as_ref() {
            if old.epoch == epoch && token <= old.token {
                return false;
            }
            if old.epoch != epoch && self.active.borrow().is_some() {
                return false;
            }
        }
        if self
            .prior
            .borrow()
            .as_ref()
            .is_some_and(|old| old.epoch == epoch && token <= old.token)
        {
            return false;
        }
        drop(announced);
        *self.announced.borrow_mut() = Some(Document {
            epoch: epoch.into(),
            token,
        });
        self.active.borrow_mut().take();
        self.pending.borrow_mut().clear();
        true
    }

    pub fn ready(
        &self,
        epoch: &str,
        token: u64,
        uri: &str,
        generation: u64,
        current_uri: Option<&str>,
    ) -> bool {
        let document = Document {
            epoch: epoch.into(),
            token,
        };
        if !self.committed.get()
            || generation != self.generation.get()
            || current_uri != Some(uri)
            || self.announced.borrow().as_ref() != Some(&document)
            || self.retired.borrow().contains(epoch)
            || self
                .prior
                .borrow()
                .as_ref()
                .is_some_and(|old| old.epoch == epoch && token <= old.token)
        {
            return false;
        }
        *self.last_ready.borrow_mut() = Some(document.clone());
        if self.active.borrow().as_ref() != Some(&document) {
            self.pending.borrow_mut().clear();
            *self.active.borrow_mut() = Some(document);
        }
        true
    }

    pub fn accepts(
        &self,
        epoch: &str,
        token: u64,
        uri: &str,
        generation: u64,
        current_uri: Option<&str>,
    ) -> bool {
        self.committed.get()
            && generation == self.generation.get()
            && current_uri == Some(uri)
            && self
                .active
                .borrow()
                .as_ref()
                .is_some_and(|document| document.epoch == epoch && document.token == token)
    }

    pub fn reply(&self, id: &str, uri: &str, generation: u64, current_uri: Option<&str>) -> bool {
        if !self.committed.get()
            || generation != self.generation.get()
            || self.active.borrow().is_none()
        {
            return false;
        }
        // Completing a request at an old SPA URL must also release its slot.
        let pending = self.pending.borrow_mut().remove(id);
        pending && current_uri == Some(uri)
    }

    pub fn is_ready(&self) -> bool {
        self.active.borrow().is_some()
    }
}
