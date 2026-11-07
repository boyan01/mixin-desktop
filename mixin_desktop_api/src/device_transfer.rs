//! Snapshot observation and the compatibility event projection share one source.
use async_stream::stream;
use futures::Stream;
use mixin_desktop_core::core::device_transfer::DeviceTransferSnapshot as CoreSnapshot;
use tokio::sync::watch;

use crate::{DeviceTransferEvent, DeviceTransferSnapshot};

pub(crate) fn states(
    mut states: watch::Receiver<CoreSnapshot>,
    mut shutdown: watch::Receiver<bool>,
) -> impl Stream<Item = DeviceTransferSnapshot> + Send + 'static {
    stream! {
        loop {
            if *shutdown.borrow() { break; }
            let current = states.borrow_and_update().clone();
            let closed = current.closed;
            yield current.into();
            if closed { break; }
            tokio::select! {
                biased;
                changed = shutdown.changed() => if changed.is_err() || *shutdown.borrow() { break; },
                changed = states.changed() => if changed.is_err() { break; },
            }
        }
    }
}

pub(crate) fn events(
    mut states: watch::Receiver<CoreSnapshot>,
    mut shutdown: watch::Receiver<bool>,
) -> impl Stream<Item = DeviceTransferEvent> + Send + 'static {
    // Capture at subscription time, so updates before the first poll survive.
    let mut previous = states.borrow_and_update().clone();
    stream! {
        if *shutdown.borrow() || previous.closed { return; }
        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => if changed.is_err() || *shutdown.borrow() { break; },
                changed = states.changed() => if changed.is_err() { break; },
            }
            let current = states.borrow_and_update().clone();
            let events = current.changes_since(&previous);
            previous = current;
            for event in events {
                if *shutdown.borrow() { return; }
                yield event.into();
            }
            if previous.closed { break; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use mixin_desktop_core::core::device_transfer::{
        DeviceTransferEvent as CoreEvent, DeviceTransferOperation,
    };

    async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
        tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .expect("transfer observation did not resolve")
    }

    #[tokio::test]
    async fn state_observation_yields_current_then_terminal_and_ends_on_closed() {
        let (sender, receiver) = watch::channel(CoreSnapshot::default());
        let (_shutdown, shutdown) = watch::channel(false);
        let mut stream = Box::pin(states(receiver, shutdown));
        assert_eq!(
            next(&mut stream).await.unwrap(),
            DeviceTransferSnapshot::default()
        );
        sender.send_replace(CoreSnapshot {
            restore: DeviceTransferOperation {
                operation_id: 7,
                phase: Some(CoreEvent::RestoreSucceed),
                progress: 100.0,
                bytes_per_second: 0.0,
            },
            closed: true,
            ..Default::default()
        });
        let terminal = next(&mut stream).await.unwrap();
        assert_eq!(
            terminal.restore.phase,
            Some(DeviceTransferEvent::RestoreSucceed)
        );
        assert!(terminal.closed);
        assert!(next(&mut stream).await.is_none());
    }

    #[tokio::test]
    async fn account_shutdown_ends_observation_even_with_state_update_ready() {
        let (sender, receiver) = watch::channel(CoreSnapshot::default());
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let mut stream = Box::pin(states(receiver, shutdown_receiver));
        assert!(next(&mut stream).await.is_some());
        sender.send_modify(|state| state.backup_request_id = Some(9));
        shutdown.send_replace(true);
        assert!(next(&mut stream).await.is_none());
    }

    #[test]
    fn dropping_unpolled_observation_releases_both_subscriptions() {
        let (sender, receiver) = watch::channel(CoreSnapshot::default());
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let stream = states(receiver, shutdown_receiver);
        assert_eq!(sender.receiver_count(), 1);
        assert_eq!(shutdown.receiver_count(), 1);
        drop(stream);
        assert_eq!(sender.receiver_count(), 0);
        assert_eq!(shutdown.receiver_count(), 0);
    }

    #[tokio::test]
    async fn compatibility_observation_keeps_updates_before_first_poll_and_skips_old_request() {
        let initial = CoreSnapshot {
            restore_request_id: Some(3),
            ..Default::default()
        };
        let (sender, receiver) = watch::channel(initial);
        let (_shutdown, shutdown) = watch::channel(false);
        let mut stream = Box::pin(events(receiver, shutdown));
        for id in 1..128 {
            sender.send_modify(|state| state.backup.progress = f64::from(id));
        }
        sender.send_modify(|state| {
            state.backup.phase = Some(CoreEvent::BackupSucceed);
            state.backup.progress = 100.0;
            state.closed = true;
        });
        let mut observed = Vec::new();
        while let Some(event) = next(&mut stream).await {
            observed.push(event);
        }
        assert!(observed.contains(&DeviceTransferEvent::BackupSucceed));
        assert!(!observed.contains(&DeviceTransferEvent::BackupRequestReceived));
    }
}
