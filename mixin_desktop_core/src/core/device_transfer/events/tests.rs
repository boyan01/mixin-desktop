use super::*;

#[tokio::test]
async fn stalled_event_consumer_recovers_real_terminal_without_replaying_request() {
    let events = Arc::new(TransferEvents::default());
    let token = Arc::new(CancellationToken::new());
    let mut receiver = events.subscribe();
    let mut states = events.states();
    events.begin(Flow::Backup, Some(token.clone()));
    events.request(Flow::Restore);
    let before = states.borrow_and_update().clone();
    let (release, wait) = tokio::sync::oneshot::channel();
    let worker_events = events.clone();
    let worker = tokio::spawn(async move {
        wait.await.unwrap();
        worker_events.emit_for(Flow::Backup, &token, Event::BackupStart);
        for progress in 0..256 {
            worker_events.emit_for(
                Flow::Backup,
                &token,
                Event::BackupProgress(f64::from(progress % 100)),
            );
        }
        worker_events.finish(Flow::Backup, &token, Event::BackupSucceed);
    });
    release.send(()).unwrap();
    worker.await.unwrap();
    assert!(matches!(
        receiver.recv().await,
        Err(broadcast::error::RecvError::Lagged(_))
    ));
    states.changed().await.unwrap();
    let current = states.borrow_and_update().clone();
    assert_eq!(current.backup.phase, Some(Event::BackupSucceed));
    assert_eq!(current.backup.progress, 100.0);
    assert_eq!(current.restore_request_id, before.restore_request_id);
    let recovered = current.changes_since(&before);
    assert!(recovered.contains(&Event::BackupSucceed));
    assert!(!recovered.contains(&Event::BackupRequestReceived));
    assert!(!recovered.contains(&Event::RestoreRequestReceived));
    assert_eq!(*events.states().borrow(), current);
}

#[test]
fn replaced_worker_cannot_publish_start_progress_or_terminal_into_new_operation() {
    let events = TransferEvents::default();
    let old = Arc::new(CancellationToken::new());
    let old_id = events.begin(Flow::Restore, Some(old.clone()));
    let current = Arc::new(CancellationToken::new());
    let current_id = events.begin(Flow::Restore, Some(current.clone()));
    assert_ne!(old_id, current_id);
    assert!(old.is_cancelled());
    let expected = events.states().borrow().clone();
    events.emit_for(Flow::Restore, &old, Event::RestoreStart);
    events.emit_for(Flow::Restore, &old, Event::RestoreProgress(97.0));
    events.finish(Flow::Restore, &old, Event::RestoreSucceed);
    events.fail_waiting(Flow::Restore, old_id, Event::RestoreFailed);
    assert_eq!(*events.states().borrow(), expected);
    events.finish(Flow::Restore, &current, Event::RestoreSucceed);
    assert_eq!(
        events.states().borrow().restore.phase,
        Some(Event::RestoreSucceed)
    );
}

#[test]
fn replacement_request_rejects_old_confirmation_and_keeps_other_direction_active() {
    let events = TransferEvents::default();
    let token = Arc::new(CancellationToken::new());
    events.begin(Flow::Backup, Some(token.clone()));
    events.request(Flow::Restore);
    let old = events.states().borrow().restore_request_id.unwrap();
    events.request(Flow::Restore);
    let current = events.states().borrow().restore_request_id.unwrap();
    assert_ne!(old, current);
    assert!(!events.consume_request(Flow::Restore, Some(old)));
    assert_eq!(events.states().borrow().restore_request_id, Some(current));
    assert!(!token.is_cancelled());
    assert!(events.consume_request(Flow::Restore, Some(current)));
    assert!(events.states().borrow().restore_request_id.is_none());
    assert_eq!(
        events.states().borrow().backup.phase,
        Some(Event::BackupWaiting)
    );
    events.cancel(Flow::Backup);
}

#[test]
fn cancellation_rejects_late_success_and_waiting_failure() {
    let events = TransferEvents::default();
    let token = Arc::new(CancellationToken::new());
    let id = events.begin(Flow::Backup, Some(token.clone()));
    events.cancel(Flow::Backup);
    events.finish(Flow::Backup, &token, Event::BackupSucceed);
    events.fail_waiting(Flow::Backup, id, Event::BackupFailed);
    assert!(token.is_cancelled());
    assert_eq!(
        events.states().borrow().backup.phase,
        Some(Event::BackupCancelled)
    );
}

#[tokio::test]
async fn shutdown_publishes_closed_cancels_workers_and_disallows_restart() {
    let events = TransferEvents::default();
    let token = Arc::new(CancellationToken::new());
    events.begin(Flow::Restore, Some(token.clone()));
    events.request(Flow::Backup);
    let mut states = events.states();
    events.close();
    states.changed().await.unwrap();
    let closed = states.borrow_and_update().clone();
    assert!(closed.closed);
    assert_eq!(closed.restore.phase, Some(Event::RestoreCancelled));
    assert!(closed.backup_request_id.is_none());
    assert!(closed.restore_request_id.is_none());
    assert!(token.is_cancelled());
    let replacement = Arc::new(CancellationToken::new());
    assert_eq!(events.begin(Flow::Restore, Some(replacement.clone())), 0);
    events.request(Flow::Backup);
    events.finish(Flow::Restore, &token, Event::RestoreSucceed);
    assert!(!events.consume_request(Flow::Backup, None));
    assert!(replacement.is_cancelled());
    assert_eq!(*states.borrow(), closed);
}

#[test]
fn replaced_restore_approval_captures_only_matching_payload_and_registers_before_cancel() {
    let events = TransferEvents::default();
    let mut state = super::super::TransferState::default();
    let remote = |code| super::super::RemotePushData {
        ip: "127.0.0.1".into(),
        port: 1234,
        code,
        key: [3; super::super::KEY_SIZE],
    };
    state.remote_push = Some(remote(1111));
    events.request(Flow::Restore);
    let old = events.states().borrow().restore_request_id.unwrap();
    state.remote_push = Some(remote(2222));
    events.request(Flow::Restore);
    let current = events.states().borrow().restore_request_id.unwrap();
    assert!(state.accept_restore(&events, Some(old)).is_err());
    assert_eq!(state.remote_push.as_ref().unwrap().code, 2222);
    assert_eq!(events.states().borrow().restore_request_id, Some(current));
    let (captured, token) = state.accept_restore(&events, Some(current)).unwrap();
    assert_eq!(captured.code, 2222);
    assert!(state.remote_push.is_none());
    assert!(events.states().borrow().restore_request_id.is_none());
    assert!(Arc::ptr_eq(state.receiver_cancel.as_ref().unwrap(), &token));
    events.cancel(Flow::Restore);
    assert!(token.is_cancelled());
    events.finish(Flow::Restore, &token, Event::RestoreSucceed);
    assert_eq!(
        events.states().borrow().restore.phase,
        Some(Event::RestoreCancelled)
    );
}

#[test]
fn protocol_failure_ends_active_operation_and_invalidates_pending_approval() {
    let events = TransferEvents::default();
    let token = Arc::new(CancellationToken::new());
    events.begin(Flow::Restore, Some(token.clone()));
    events.request(Flow::Backup);
    let request = events.states().borrow().backup_request_id.unwrap();
    events.failure(super::super::ConnectionFailedReason::VersionNotMatched);
    assert!(token.is_cancelled());
    assert_eq!(
        events.states().borrow().restore.phase,
        Some(Event::ConnectionFailed(
            super::super::ConnectionFailedReason::VersionNotMatched
        ))
    );
    assert!(!events.consume_request(Flow::Backup, Some(request)));
    events.finish(Flow::Restore, &token, Event::RestoreSucceed);
    assert_eq!(
        events.states().borrow().restore.phase,
        Some(Event::ConnectionFailed(
            super::super::ConnectionFailedReason::VersionNotMatched
        ))
    );
}

#[test]
fn peer_connection_continues_waiting_identity_and_rejects_cancelled_or_replaced_pull() {
    let events = TransferEvents::default();
    let waiting = events.begin(Flow::Restore, None);
    let worker = Arc::new(CancellationToken::new());
    assert!(events.attach(Flow::Restore, waiting, worker.clone()));
    events.emit_for(Flow::Restore, &worker, Event::RestoreConnected);
    assert_eq!(events.states().borrow().restore.operation_id, waiting);
    events.cancel(Flow::Restore);
    let late = Arc::new(CancellationToken::new());
    assert!(!events.attach(Flow::Restore, waiting, late.clone()));
    assert!(late.is_cancelled());
    let replacement = events.begin(Flow::Restore, None);
    let old = Arc::new(CancellationToken::new());
    assert!(!events.attach(Flow::Restore, waiting, old.clone()));
    assert!(old.is_cancelled());
    assert_eq!(events.states().borrow().restore.operation_id, replacement);
    events.cancel(Flow::Restore);
}
