use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, AttachmentTransferProgress};
use nsh_core::prelude::*;

use crate::runtime;

/// Transient presentation of the account's authoritative transfer snapshots.
#[derive(Clone, Copy)]
pub(crate) struct AttachmentProgress(Signal<Vec<AttachmentTransferProgress>>);

impl AttachmentProgress {
    /// Follows the account's snapshots for as long as the current owner lives.
    pub(crate) fn mount(account: &AccountClient) -> Self {
        let progress = signal(Vec::new());
        let changes = runtime::observe(account.attachment_progress_changes());
        spawn(async move {
            futures::pin_mut!(changes);
            while let Some(snapshot) = changes.next().await {
                progress.set(snapshot);
            }
        });
        Self(progress)
    }

    /// No transfers, for the debug preview.
    #[cfg(debug_assertions)]
    pub(crate) fn detached() -> Self {
        Self(signal(Vec::new()))
    }

    pub(crate) fn bytes(self, transcript_id: Option<&str>, message_id: &str) -> Option<(u64, u64)> {
        self.0.with(|values| {
            values
                .iter()
                .find(|value| {
                    value.message_id == message_id
                        && value.transcript_id.as_deref() == transcript_id
                })
                .map(|value| (value.completed, value.total))
        })
    }
}
