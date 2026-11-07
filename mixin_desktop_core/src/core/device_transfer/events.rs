//! Current transfer state is retained independently of the lossy event queue.
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;

use super::{DeviceTransferEvent as Event, DeviceTransferOperation, DeviceTransferSnapshot};

#[derive(Clone, Copy)]
pub(super) enum Flow {
    Backup,
    Restore,
}

impl Flow {
    fn operation(self, snapshot: &mut DeviceTransferSnapshot) -> &mut DeviceTransferOperation {
        match self {
            Self::Backup => &mut snapshot.backup,
            Self::Restore => &mut snapshot.restore,
        }
    }
    fn request(self, snapshot: &mut DeviceTransferSnapshot) -> &mut Option<u64> {
        match self {
            Self::Backup => &mut snapshot.backup_request_id,
            Self::Restore => &mut snapshot.restore_request_id,
        }
    }
    fn waiting(self) -> Event {
        match self {
            Self::Backup => Event::BackupWaiting,
            Self::Restore => Event::RestoreWaiting,
        }
    }
    fn cancelled(self) -> Event {
        match self {
            Self::Backup => Event::BackupCancelled,
            Self::Restore => Event::RestoreCancelled,
        }
    }
    fn token(self, state: &mut State) -> &mut Option<Arc<CancellationToken>> {
        match self {
            Self::Backup => &mut state.backup_token,
            Self::Restore => &mut state.restore_token,
        }
    }
}

#[derive(Default)]
struct State {
    snapshot: DeviceTransferSnapshot,
    next_id: u64,
    backup_token: Option<Arc<CancellationToken>>,
    restore_token: Option<Arc<CancellationToken>>,
}

impl State {
    fn next_id(&mut self) -> u64 {
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("transfer identity exhausted");
        self.next_id
    }
}

pub(super) struct TransferEvents {
    state: Mutex<State>,
    events: broadcast::Sender<Event>,
    snapshot: watch::Sender<DeviceTransferSnapshot>,
}

impl Default for TransferEvents {
    fn default() -> Self {
        Self {
            state: Mutex::default(),
            events: broadcast::channel(64).0,
            snapshot: watch::channel(DeviceTransferSnapshot::default()).0,
        }
    }
}

impl TransferEvents {
    pub(super) fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }
    pub(super) fn states(&self) -> watch::Receiver<DeviceTransferSnapshot> {
        self.snapshot.subscribe()
    }
    fn publish(&self, state: &State, event: Option<Event>) {
        self.snapshot.send_replace(state.snapshot.clone());
        if let Some(event) = event {
            let _ = self.events.send(event);
        }
    }
    pub(super) fn begin(&self, flow: Flow, token: Option<Arc<CancellationToken>>) -> u64 {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.snapshot.closed {
            if let Some(token) = token {
                token.cancel();
            }
            return 0;
        }
        if let Some(previous) = flow.token(&mut state).take() {
            previous.cancel();
        }
        *flow.token(&mut state) = token;
        let id = state.next_id();
        let phase = flow.waiting();
        *flow.operation(&mut state.snapshot) = DeviceTransferOperation {
            operation_id: id,
            phase: Some(phase.clone()),
            ..Default::default()
        };
        self.publish(&state, Some(phase));
        id
    }
    /// Continue the same outbound pull after its peer supplies connection data.
    pub(super) fn attach(
        &self,
        flow: Flow,
        operation_id: u64,
        token: Arc<CancellationToken>,
    ) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let operation = flow.operation(&mut state.snapshot);
        if operation.operation_id != operation_id
            || operation.phase != Some(flow.waiting())
            || state.snapshot.closed
        {
            token.cancel();
            return false;
        }
        if let Some(previous) = flow.token(&mut state).replace(token) {
            previous.cancel();
        }
        true
    }
    pub(super) fn request(&self, flow: Flow) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.snapshot.closed {
            return;
        }
        let id = state.next_id();
        *flow.request(&mut state.snapshot) = Some(id);
        let event = match flow {
            Flow::Backup => Event::RestoreRequestReceived,
            Flow::Restore => Event::BackupRequestReceived,
        };
        self.publish(&state, Some(event));
    }
    /// Caller holds the service state lock while capturing/removing request payload.
    pub(super) fn consume_request(&self, flow: Flow, expected: Option<u64>) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let current = *flow.request(&mut state.snapshot);
        if state.snapshot.closed || expected.is_some_and(|expected| current != Some(expected)) {
            return false;
        }
        *flow.request(&mut state.snapshot) = None;
        self.publish(&state, None);
        true
    }
    pub(super) fn emit_for(&self, flow: Flow, token: &Arc<CancellationToken>, event: Event) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.snapshot.closed
            || token.is_cancelled()
            || !flow
                .token(&mut state)
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            return;
        }
        Self::apply(flow.operation(&mut state.snapshot), &event);
        self.publish(&state, Some(event));
    }
    pub(super) fn finish(&self, flow: Flow, token: &Arc<CancellationToken>, event: Event) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.snapshot.closed
            || token.is_cancelled()
            || !flow
                .token(&mut state)
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            return;
        }
        *flow.token(&mut state) = None;
        Self::apply(flow.operation(&mut state.snapshot), &event);
        self.publish(&state, Some(event));
    }
    pub(super) fn fail_waiting(&self, flow: Flow, operation_id: u64, event: Event) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.snapshot.closed
            || flow.operation(&mut state.snapshot).operation_id != operation_id
            || !matches!(
                flow.operation(&mut state.snapshot).phase,
                Some(Event::BackupWaiting | Event::BackupServerCreated | Event::RestoreWaiting)
            )
        {
            return;
        }
        if let Some(token) = flow.token(&mut state).take() {
            token.cancel();
        }
        Self::apply(flow.operation(&mut state.snapshot), &event);
        self.publish(&state, Some(event));
    }
    pub(super) fn cancel(&self, flow: Flow) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(token) = flow.token(&mut state).take() {
            token.cancel();
        }
        let operation = flow.operation(&mut state.snapshot);
        if matches!(
            operation.phase,
            Some(
                Event::RestoreWaiting
                    | Event::RestoreConnected
                    | Event::RestoreStart
                    | Event::BackupWaiting
                    | Event::BackupServerCreated
                    | Event::BackupStart
            )
        ) {
            let event = flow.cancelled();
            Self::apply(operation, &event);
            self.publish(&state, Some(event));
        }
    }
    pub(super) fn failure(&self, reason: super::ConnectionFailedReason) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.snapshot.closed {
            return;
        }
        let event = Event::ConnectionFailed(reason);
        let mut affected = false;
        for flow in [Flow::Backup, Flow::Restore] {
            let operation = flow.operation(&mut state.snapshot);
            if matches!(
                operation.phase,
                Some(
                    Event::RestoreWaiting
                        | Event::RestoreConnected
                        | Event::RestoreStart
                        | Event::BackupWaiting
                        | Event::BackupServerCreated
                        | Event::BackupStart
                )
            ) {
                Self::apply(operation, &event);
                affected = true;
                if let Some(token) = flow.token(&mut state).take() {
                    token.cancel();
                }
            }
        }
        if !affected {
            let id = state.next_id();
            state.snapshot.backup = DeviceTransferOperation {
                operation_id: id,
                ..Default::default()
            };
            Self::apply(&mut state.snapshot.backup, &event);
        }
        state.snapshot.backup_request_id = None;
        state.snapshot.restore_request_id = None;
        self.publish(&state, Some(event));
    }
    pub(super) fn clear_requests(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.snapshot.backup_request_id = None;
        state.snapshot.restore_request_id = None;
        self.publish(&state, None);
    }
    pub(super) fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut cancelled = [None, None];
        for (flow, event) in [Flow::Backup, Flow::Restore]
            .into_iter()
            .zip(&mut cancelled)
        {
            if let Some(token) = flow.token(&mut state).take() {
                token.cancel();
            }
            let operation = flow.operation(&mut state.snapshot);
            if matches!(
                operation.phase,
                Some(
                    Event::RestoreWaiting
                        | Event::RestoreConnected
                        | Event::RestoreStart
                        | Event::BackupWaiting
                        | Event::BackupServerCreated
                        | Event::BackupStart
                )
            ) {
                *event = Some(flow.cancelled());
                Self::apply(operation, event.as_ref().expect("cancel event assigned"));
            }
        }
        state.snapshot.backup_request_id = None;
        state.snapshot.restore_request_id = None;
        state.snapshot.closed = true;
        self.publish(&state, None);
        for event in cancelled.into_iter().flatten() {
            let _ = self.events.send(event);
        }
    }
    fn apply(operation: &mut DeviceTransferOperation, event: &Event) {
        match event {
            Event::RestoreProgress(value) | Event::BackupProgress(value) if value.is_finite() => {
                operation.progress = value.clamp(0.0, 100.0)
            }
            Event::RestoreNetworkSpeed(value) | Event::BackupNetworkSpeed(value)
                if value.is_finite() =>
            {
                operation.bytes_per_second = value.max(0.0)
            }
            Event::RestoreProgress(_)
            | Event::BackupProgress(_)
            | Event::RestoreNetworkSpeed(_)
            | Event::BackupNetworkSpeed(_) => {}
            _ => {
                operation.phase = Some(event.clone());
                if matches!(event, Event::RestoreSucceed | Event::BackupSucceed) {
                    operation.progress = 100.0;
                }
            }
        }
    }
}

impl DeviceTransferSnapshot {
    /// Coalesced legacy events derived from authoritative state. An unchanged
    /// pending request is never replayed when intermediate progress was skipped.
    pub fn changes_since(&self, previous: &Self) -> Vec<Event> {
        let mut events = Vec::new();
        for (flow, operation, old) in [
            (Flow::Backup, &self.backup, &previous.backup),
            (Flow::Restore, &self.restore, &previous.restore),
        ] {
            let replaced = operation.operation_id != old.operation_id;
            if replaced || operation.phase != old.phase {
                if let Some(phase) = &operation.phase {
                    events.push(phase.clone());
                }
            }
            if replaced || operation.progress != old.progress {
                events.push(match flow {
                    Flow::Backup => Event::BackupProgress(operation.progress),
                    Flow::Restore => Event::RestoreProgress(operation.progress),
                });
            }
            if replaced || operation.bytes_per_second != old.bytes_per_second {
                events.push(match flow {
                    Flow::Backup => Event::BackupNetworkSpeed(operation.bytes_per_second),
                    Flow::Restore => Event::RestoreNetworkSpeed(operation.bytes_per_second),
                });
            }
        }
        if self.backup_request_id.is_some() && self.backup_request_id != previous.backup_request_id
        {
            events.push(Event::RestoreRequestReceived);
        }
        if self.restore_request_id.is_some()
            && self.restore_request_id != previous.restore_request_id
        {
            events.push(Event::BackupRequestReceived);
        }
        events
    }
}

#[cfg(test)]
mod tests;
