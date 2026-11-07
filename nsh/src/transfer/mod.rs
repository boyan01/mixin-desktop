use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ConnectionFailedReason, DeviceTransferCommand as Command,
    DeviceTransferEvent as Event, DeviceTransferOperation, DeviceTransferSnapshot,
};
use nsh_core::prelude::*;

use crate::runtime;

mod view;
pub(crate) use view::{screen, surface};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Restore,
    Backup,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Idle,
    Waiting,
    Transferring,
    Completed,
    Failed,
    VersionMismatch,
    Cancelled,
}

impl Status {
    fn active(self) -> bool {
        matches!(self, Self::Waiting | Self::Transferring)
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Job {
    operation_id: u64,
    status: Status,
    progress: f64,
    speed: f64,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            operation_id: 0,
            status: Status::Idle,
            progress: 0.0,
            speed: 0.0,
        }
    }
}

/// Presentation state and subscriptions owned by the signed-in session.
#[derive(Clone, Copy)]
pub(crate) struct Handle {
    owner: Owner,
    account: Stored<Arc<AccountClient>>,
    restore: Signal<Job>,
    backup: Signal<Job>,
    restore_request: Signal<Option<u64>>,
    backup_request: Signal<Option<u64>>,
    restore_response_error: Signal<Option<u64>>,
    backup_response_error: Signal<Option<u64>>,
    restore_busy: Signal<bool>,
    backup_busy: Signal<bool>,
    visible: Signal<Option<Direction>>,
}

pub(crate) fn mount(account: Arc<AccountClient>) -> Handle {
    let handle = Handle {
        owner: Owner::current(),
        account: stored(account.clone()),
        restore: signal(Job::default()),
        backup: signal(Job::default()),
        restore_request: signal(None),
        backup_request: signal(None),
        restore_response_error: signal(None),
        backup_response_error: signal(None),
        restore_busy: signal(false),
        backup_busy: signal(false),
        visible: signal(None),
    };
    let mut events = Box::pin(runtime::observe(account.device_transfer_states()));
    spawn(async move {
        while let Some(event) = events.next().await {
            handle.receive(event);
        }
    });
    cleanup(move || {
        drop(runtime::commit(async move {
            account
                .device_transfer_command(Command::CancelRestore)
                .await?;
            account.device_transfer_command(Command::CancelBackup).await
        }));
    });
    handle
}

impl Handle {
    fn job(self, direction: Direction) -> Signal<Job> {
        match direction {
            Direction::Restore => self.restore,
            Direction::Backup => self.backup,
        }
    }
    fn busy(self, direction: Direction) -> Signal<bool> {
        match direction {
            Direction::Restore => self.restore_busy,
            Direction::Backup => self.backup_busy,
        }
    }
    fn request(self, direction: Direction) -> Signal<Option<u64>> {
        match direction {
            Direction::Restore => self.restore_request,
            Direction::Backup => self.backup_request,
        }
    }
    fn response_error(self, direction: Direction) -> Signal<Option<u64>> {
        match direction {
            Direction::Restore => self.restore_response_error,
            Direction::Backup => self.backup_response_error,
        }
    }
    fn receive(self, state: DeviceTransferSnapshot) {
        batch(|| {
            self.restore_request.set(state.restore_request_id);
            self.backup_request.set(state.backup_request_id);
            for (direction, operation) in [
                (Direction::Backup, state.backup),
                (Direction::Restore, state.restore),
            ] {
                let previous = self.job(direction).get_untracked();
                let job = Job::from(operation);
                self.job(direction).set(job);
                if job.status != Status::Idle
                    && (job.operation_id != previous.operation_id || job.status != previous.status)
                {
                    self.visible.set(Some(direction));
                }
            }
        });
    }
    fn send(self, direction: Direction, command: Command) {
        self.dispatch(direction, command, None);
    }
    fn send_request(self, direction: Direction, command: Command, request_id: u64) {
        if self.request(direction).get_untracked() != Some(request_id) {
            return;
        }
        self.dispatch(direction, command, Some(request_id));
    }
    fn dispatch(self, direction: Direction, command: Command, request_id: Option<u64>) {
        if self.busy(direction).get_untracked() {
            return;
        }
        self.busy(direction).set(true);
        self.response_error(direction).set(None);
        let operation_id = self.job(direction).get_untracked().operation_id;
        let account = self.account.with(|a| a.clone());
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move {
                    match request_id {
                        Some(id) => account.device_transfer_request_command(command, id).await,
                        None => account.device_transfer_command(command).await,
                    }
                })
                .await;
                // A command completion must not overwrite a replacement operation or
                // request. Business transitions arrive only through the state stream.
                if result.is_err()
                    && request_id.is_none()
                    && self.job(direction).get_untracked().operation_id == operation_id
                {
                    self.job(direction)
                        .update(|job| job.status = Status::Failed);
                    self.visible.set(Some(direction));
                }
                if result.is_err()
                    && request_id.is_some()
                    && self.request(direction).get_untracked() == request_id
                {
                    self.response_error(direction).set(request_id);
                }
                self.busy(direction).set(false);
            })
        });
    }
    fn cancel(self, direction: Direction) {
        self.send(
            direction,
            match direction {
                Direction::Restore => Command::CancelRestore,
                Direction::Backup => Command::CancelBackup,
            },
        );
    }
}

impl From<DeviceTransferOperation> for Job {
    fn from(operation: DeviceTransferOperation) -> Self {
        let status = match operation.phase {
            None => Status::Idle,
            Some(Event::BackupWaiting | Event::RestoreWaiting | Event::BackupServerCreated) => {
                Status::Waiting
            }
            Some(Event::RestoreConnected | Event::RestoreStart | Event::BackupStart) => {
                Status::Transferring
            }
            Some(Event::RestoreSucceed | Event::BackupSucceed) => Status::Completed,
            Some(Event::RestoreCancelled | Event::BackupCancelled) => Status::Cancelled,
            Some(Event::ConnectionFailed(ConnectionFailedReason::VersionNotMatched)) => {
                Status::VersionMismatch
            }
            Some(_) => Status::Failed,
        };
        Self {
            operation_id: operation.operation_id,
            status,
            progress: operation.progress,
            speed: operation.bytes_per_second,
        }
    }
}
