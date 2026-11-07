//! Session-owned delivery of editing buffers to the authoritative account store.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mixin_desktop_api::AccountClient;
use nsh_core::prelude::*;
use tokio::sync::{mpsc, watch};

use crate::runtime;

type Pending = Arc<Mutex<HashMap<String, String>>>;

struct Writer {
    pending: Pending,
    wake: mpsc::Sender<()>,
}

/// A presentation command handle, not a second durable draft store.
#[derive(Clone, Copy)]
pub(crate) struct Drafts {
    writer: Stored<Writer>,
    pub failed: Signal<bool>,
}

impl Drafts {
    pub(crate) fn mount(account: Arc<AccountClient>) -> Self {
        // One wake-up is enough: pending text is coalesced by conversation. A
        // slow database must not create a queue entry for every keystroke.
        let (wake, mut receiver) = mpsc::channel(1);
        let pending: Pending = Arc::default();
        let worker_pending = pending.clone();
        let (errors, mut error_updates) = watch::channel(false);
        drop(runtime::commit(async move {
            while receiver.recv().await.is_some() {
                loop {
                    let batch = std::mem::take(&mut *worker_pending.lock().expect("draft buffers"));
                    if batch.is_empty() {
                        break;
                    }
                    let mut failed = false;
                    for (id, text) in batch {
                        if let Err(error) = account
                            .conversation()
                            .update_draft(id.clone(), text.clone())
                            .await
                        {
                            log::warn!("Saving a conversation draft failed: {error}");
                            // Preserve a failed buffer until another edit or shutdown;
                            // never overwrite a newer edit that arrived during the write.
                            worker_pending
                                .lock()
                                .expect("draft buffers")
                                .entry(id)
                                .or_insert(text);
                            failed = true;
                        }
                    }
                    errors.send_replace(failed);
                    if failed {
                        break;
                    }
                }
            }
            // Dropping the session closes the channel only after the last queued
            // wake-up, so successful writes drain before account shutdown.
            Ok(())
        }));
        let failed = signal(false);
        spawn(async move {
            while error_updates.changed().await.is_ok() {
                failed.set(*error_updates.borrow_and_update());
            }
        });
        Self {
            writer: stored(Writer { pending, wake }),
            failed,
        }
    }

    pub(crate) fn retry(self) {
        self.writer.with(|writer| {
            if let Err(mpsc::error::TrySendError::Closed(_)) = writer.wake.try_send(()) {
                self.failed.set(true);
            }
        });
    }

    pub(crate) fn save(self, id: String, text: String) {
        self.writer.with(|writer| {
            writer
                .pending
                .lock()
                .expect("draft buffers")
                .insert(id, text);
            // Full means an existing wake-up will flush the latest text.
            if let Err(mpsc::error::TrySendError::Closed(_)) = writer.wake.try_send(()) {
                self.failed.set(true);
            }
        });
    }
}
