use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::watch;

use super::model::AttachmentTransferProgress;

type ProgressMap = HashMap<(Option<String>, String), AttachmentTransferProgress>;

pub(super) struct AttachmentProgresses {
    values: Arc<Mutex<ProgressMap>>,
    revision: watch::Sender<u64>,
}

impl Default for AttachmentProgresses {
    fn default() -> Self {
        Self {
            values: Arc::default(),
            revision: watch::channel(0).0,
        }
    }
}

impl AttachmentProgresses {
    pub(super) fn set(
        &self,
        transcript_id: Option<&str>,
        message_id: &str,
        completed: u64,
        total: u64,
    ) {
        let mut values = self.values.lock().unwrap_or_else(|p| p.into_inner());
        let key = (transcript_id.map(str::to_owned), message_id.to_owned());
        let value = AttachmentTransferProgress {
            message_id: key.1.clone(),
            transcript_id: key.0.clone(),
            completed,
            total,
        };
        if values.get(&key) != Some(&value) {
            values.insert(key, value);
            self.revision
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }

    pub(super) fn remove(&self, transcript_id: Option<&str>, message_id: &str) {
        let mut values = self.values.lock().unwrap_or_else(|p| p.into_inner());
        if values
            .remove(&(transcript_id.map(str::to_owned), message_id.to_owned()))
            .is_some()
        {
            self.revision
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }

    pub(super) fn fraction(&self, message_id: &str) -> f64 {
        self.values
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&(None, message_id.to_owned()))
            .filter(|value| value.total > 0)
            .map(|value| (value.completed as f64 / value.total as f64).clamp(0.0, 1.0))
            .unwrap_or_default()
    }

    pub(super) fn changes(
        &self,
        mut shutdown: watch::Receiver<bool>,
    ) -> impl futures::Stream<Item = Vec<AttachmentTransferProgress>> + Send + 'static {
        let values = self.values.clone();
        let mut revision = self.revision.subscribe();
        async_stream::stream! {
            loop {
                if *shutdown.borrow() { break; }
                revision.borrow_and_update();
                let snapshot = values.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
                yield snapshot;
                tokio::select! {
                    changed = revision.changed() => if changed.is_err() { break; },
                    changed = shutdown.changed() => if changed.is_err() || *shutdown.borrow() { break; },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn same_child_in_different_roots_keeps_progress_and_completion_isolated() {
        let progress = AttachmentProgresses::default();
        let (_stop, shutdown) = watch::channel(false);
        let changes = progress.changes(shutdown);
        futures::pin_mut!(changes);
        assert!(changes.next().await.unwrap().is_empty());
        progress.set(None, "file", 20, 100);
        progress.set(Some("root-a"), "file", 30, 100);
        progress.set(Some("root-b"), "file", 40, 100);
        let snapshot = changes.next().await.unwrap();
        assert_eq!(snapshot.len(), 3);
        assert_eq!(progress.fraction("file"), 0.2);
        progress.remove(Some("root-a"), "file");
        let snapshot = changes.next().await.unwrap();
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot
            .iter()
            .any(|p| p.transcript_id.as_deref() == Some("root-b") && p.completed == 40));
        progress.remove(None, "file");
        assert_eq!(progress.fraction("file"), 0.0);
    }

    #[tokio::test]
    async fn late_subscriber_gets_latest_bytes_and_coalesces_updates_without_losing_completion() {
        let progress = AttachmentProgresses::default();
        progress.set(None, "file", 2, 10);
        let (_stop, shutdown) = watch::channel(false);
        let changes = progress.changes(shutdown);
        futures::pin_mut!(changes);
        assert_eq!(changes.next().await.unwrap()[0].completed, 2);
        for completed in 3..=10 {
            progress.set(None, "file", completed, 10);
        }
        assert_eq!(changes.next().await.unwrap()[0].completed, 10);
        progress.remove(None, "file");
        assert!(changes.next().await.unwrap().is_empty());
        drop(progress);
        assert!(changes.next().await.is_none());
    }

    #[tokio::test]
    async fn account_shutdown_ends_live_and_unstarted_subscriptions() {
        let progress = AttachmentProgresses::default();
        let (stop, shutdown) = watch::channel(false);
        let changes = progress.changes(shutdown.clone());
        futures::pin_mut!(changes);
        assert!(changes.next().await.unwrap().is_empty());
        stop.send_replace(true);
        assert!(changes.next().await.is_none());
        let changes = progress.changes(shutdown);
        futures::pin_mut!(changes);
        assert!(changes.next().await.is_none());
    }
}
