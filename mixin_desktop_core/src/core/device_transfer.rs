#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use crate::db::QueryBuilder;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{anyhow, bail, Context, Result};
#[cfg(not(target_arch = "wasm32"))]
use base64ct::{Base64, Encoding};
#[cfg(not(target_arch = "wasm32"))]
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
#[cfg(not(target_arch = "wasm32"))]
use ring::rand::{SecureRandom, SystemRandom};
#[cfg(not(target_arch = "wasm32"))]
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Map, Value};
#[cfg(not(target_arch = "wasm32"))]
use sha2::Sha256;
#[cfg(not(target_arch = "wasm32"))]
use sqlx::AssertSqlSafe;
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::{TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::{broadcast, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use tokio_util::sync::CancellationToken;
#[cfg(not(target_arch = "wasm32"))]
use uuid::Uuid;

#[cfg(all(test, not(target_arch = "wasm32")))]
use crate::core::attachment::attachment_path;
#[cfg(not(target_arch = "wasm32"))]
use crate::core::conversation_change::ConversationChangeNotifier;
#[cfg(not(target_arch = "wasm32"))]
use crate::core::message::sender::MessageSender;
#[cfg(not(target_arch = "wasm32"))]
use crate::db::mixin::message::{MediaStatus, Message};
#[cfg(not(target_arch = "wasm32"))]
use crate::db::mixin::message_fts::message_fts_content;
#[cfg(not(target_arch = "wasm32"))]
use crate::db::MixinDatabase;

pub const DEVICE_TRANSFER_ACTION: &str = "DEVICE_TRANSFER";
#[cfg(not(target_arch = "wasm32"))]
const PROTOCOL_VERSION: i32 = 3;
#[cfg(not(target_arch = "wasm32"))]
const PACKET_COMMAND: u8 = 1;
#[cfg(not(target_arch = "wasm32"))]
const PACKET_JSON: u8 = 2;
#[cfg(not(target_arch = "wasm32"))]
const PACKET_FILE: u8 = 3;
#[cfg(not(target_arch = "wasm32"))]
const HMAC_SIZE: usize = 32;
#[cfg(not(target_arch = "wasm32"))]
const IV_SIZE: usize = 16;
#[cfg(not(target_arch = "wasm32"))]
const KEY_SIZE: usize = 64;
#[cfg(not(target_arch = "wasm32"))]
const JSON_PACKET_LIMIT: usize = 500 * 1024;

#[derive(Clone, Debug)]
pub struct DeviceTransferControlEvent {
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionFailedReason {
    VersionNotMatched,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceTransferEvent {
    RestoreWaiting,
    RestoreCancelled,
    BackupWaiting,
    BackupCancelled,
    RestoreConnected,
    RestoreStart,
    RestoreSucceed,
    RestoreFailed,
    BackupServerCreated,
    BackupStart,
    BackupSucceed,
    BackupFailed,
    RestoreProgress(f64),
    BackupProgress(f64),
    RestoreNetworkSpeed(f64),
    BackupNetworkSpeed(f64),
    BackupRequestReceived,
    RestoreRequestReceived,
    ConnectionFailed(ConnectionFailedReason),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceTransferOperation {
    pub operation_id: u64,
    pub phase: Option<DeviceTransferEvent>,
    pub progress: f64,
    pub bytes_per_second: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceTransferSnapshot {
    pub backup: DeviceTransferOperation,
    pub restore: DeviceTransferOperation,
    pub backup_request_id: Option<u64>,
    pub restore_request_id: Option<u64>,
    pub closed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
mod attachments;
#[cfg(not(target_arch = "wasm32"))]
mod protocol;
#[cfg(all(test, not(target_arch = "wasm32")))]
use protocol::{decrypt_cbc, encrypt_cbc};
#[cfg(not(target_arch = "wasm32"))]
use protocol::{read_packet, write_command, write_file_packet, write_json_record, Packet};
#[cfg(not(target_arch = "wasm32"))]
mod events;
#[cfg(not(target_arch = "wasm32"))]
use events::{Flow, TransferEvents};

#[derive(Clone, Debug)]
pub enum DeviceTransferCommand {
    PullToRemote,
    PushToRemote,
    CancelRestore,
    CancelBackup,
    CancelBackupRequest,
    CancelRestoreRequest,
    ConfirmRestore,
    ConfirmBackup,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct TransferCommand {
    #[serde(rename = "device_id")]
    device_id: String,
    action: String,
    version: i32,
    ip: Option<String>,
    port: Option<u16>,
    #[serde(rename = "secret_key")]
    secret_key: Option<String>,
    #[serde(default = "desktop_platform")]
    platform: String,
    code: Option<u16>,
    total: Option<u64>,
    #[serde(rename = "user_id")]
    user_id: Option<String>,
    progress: Option<f64>,
}

#[cfg(not(target_arch = "wasm32"))]
fn desktop_platform() -> String {
    "desktop".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
impl TransferCommand {
    fn simple(device_id: &str, action: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            action: action.to_string(),
            version: PROTOCOL_VERSION,
            ip: None,
            port: None,
            secret_key: None,
            platform: desktop_platform(),
            code: None,
            total: None,
            user_id: None,
            progress: None,
        }
    }

    fn pull(device_id: &str) -> Self {
        Self::simple(device_id, "pull")
    }

    fn push(device_id: &str, ip: String, port: u16, code: u16, key: &[u8]) -> Self {
        Self {
            ip: Some(ip),
            port: Some(port),
            code: Some(code),
            secret_key: Some(Base64::encode_string(key)),
            ..Self::simple(device_id, "push")
        }
    }

    fn connect(device_id: &str, code: u16, user_id: &str) -> Self {
        Self {
            code: Some(code),
            user_id: Some(user_id.to_string()),
            ..Self::simple(device_id, "connect")
        }
    }

    fn start(device_id: &str, total: u64) -> Self {
        Self {
            total: Some(total),
            ..Self::simple(device_id, "start")
        }
    }

    fn progress(device_id: &str, progress: f64) -> Self {
        Self {
            progress: Some(progress),
            ..Self::simple(device_id, "progress")
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct RemotePushData {
    ip: String,
    port: u16,
    code: u16,
    key: [u8; KEY_SIZE],
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct TransferState {
    waiting_for_remote_push: Option<u64>,
    remote_push: Option<RemotePushData>,
    sender_cancel: Option<Arc<CancellationToken>>,
    receiver_cancel: Option<Arc<CancellationToken>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl TransferState {
    fn cancel_resources(&mut self) {
        for token in [self.sender_cancel.take(), self.receiver_cancel.take()]
            .into_iter()
            .flatten()
        {
            token.cancel();
        }
        self.remote_push = None;
        self.waiting_for_remote_push = None;
    }

    fn accept_restore(
        &mut self,
        events: &TransferEvents,
        expected: Option<u64>,
    ) -> Result<(RemotePushData, Arc<CancellationToken>)> {
        if self.remote_push.is_none() {
            bail!("no pending remote transfer");
        }
        if !events.consume_request(Flow::Restore, expected) {
            bail!("device transfer request was replaced");
        }
        let remote = self
            .remote_push
            .take()
            .expect("pending transfer checked under lock");
        let cancel = replace_transfer(&mut self.receiver_cancel);
        events.begin(Flow::Restore, Some(cancel.clone()));
        Ok((remote, cancel))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn replace_transfer(slot: &mut Option<Arc<CancellationToken>>) -> Arc<CancellationToken> {
    if let Some(previous) = slot.take() {
        previous.cancel();
    }
    let cancel = Arc::new(CancellationToken::new());
    *slot = Some(cancel.clone());
    cancel
}

#[cfg(not(target_arch = "wasm32"))]
fn finish_transfer(
    slot: &mut Option<Arc<CancellationToken>>,
    cancel: &Arc<CancellationToken>,
) -> bool {
    if slot
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, cancel))
    {
        *slot = None;
        true
    } else {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn emit_running(
    events: &TransferEvents,
    cancel: &Arc<CancellationToken>,
    event: DeviceTransferEvent,
) {
    let flow = match &event {
        DeviceTransferEvent::BackupStart
        | DeviceTransferEvent::BackupProgress(_)
        | DeviceTransferEvent::BackupNetworkSpeed(_) => Flow::Backup,
        DeviceTransferEvent::RestoreConnected
        | DeviceTransferEvent::RestoreStart
        | DeviceTransferEvent::RestoreProgress(_)
        | DeviceTransferEvent::RestoreNetworkSpeed(_) => Flow::Restore,
        _ => return,
    };
    events.emit_for(flow, cancel, event);
}

#[cfg(not(target_arch = "wasm32"))]
pub struct DeviceTransferService {
    database: Arc<MixinDatabase>,
    message_sender: Arc<MessageSender>,
    user_id: String,
    primary_session_id: Option<String>,
    device_id: String,
    account_data_dir: PathBuf,
    state: Mutex<TransferState>,
    events: TransferEvents,
    conversation_changes: ConversationChangeNotifier,
}

#[cfg(not(target_arch = "wasm32"))]
impl DeviceTransferService {
    pub fn new(
        database: Arc<MixinDatabase>,
        message_sender: Arc<MessageSender>,
        user_id: String,
        session_id: String,
        primary_session_id: Option<String>,
        account_data_dir: PathBuf,
        conversation_changes: ConversationChangeNotifier,
    ) -> Arc<Self> {
        Arc::new(Self {
            database,
            message_sender,
            user_id,
            primary_session_id,
            device_id: session_id,
            account_data_dir,
            state: Mutex::new(TransferState::default()),
            events: TransferEvents::default(),
            conversation_changes,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DeviceTransferEvent> {
        self.events.subscribe()
    }

    pub fn states(&self) -> tokio::sync::watch::Receiver<DeviceTransferSnapshot> {
        self.events.states()
    }

    pub async fn shutdown(&self) {
        let mut state = self.state.lock().await;
        state.cancel_resources();
        self.events.close();
    }

    pub async fn cancel_all(&self) {
        let mut state = self.state.lock().await;
        state.cancel_resources();
        self.events.cancel(Flow::Backup);
        self.events.cancel(Flow::Restore);
        self.events.clear_requests();
    }

    pub async fn run(
        self: Arc<Self>,
        mut controls: broadcast::Receiver<DeviceTransferControlEvent>,
    ) {
        loop {
            match controls.recv().await {
                Ok(event) => {
                    if let Err(error) = self.handle_remote_content(&event.content).await {
                        log::error!("handle device transfer command: {error:?}");
                        self.fail_connection(ConnectionFailedReason::Unknown).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("device transfer command stream lagged by {skipped}");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    self.shutdown().await;
                    return;
                }
            }
        }
    }

    pub async fn command(self: &Arc<Self>, command: DeviceTransferCommand) -> Result<()> {
        if self.primary_session_id.is_none() {
            bail!("device transfer is unavailable without a primary session");
        }
        match command {
            DeviceTransferCommand::PullToRemote => {
                let operation = {
                    let mut state = self.state.lock().await;
                    let operation = self.events.begin(Flow::Restore, None);
                    state.waiting_for_remote_push = (operation != 0).then_some(operation);
                    operation
                };
                if operation == 0 {
                    bail!("device transfer service is closed");
                }
                let result = self
                    .send_remote(TransferCommand::pull(&self.device_id))
                    .await;
                if result.is_err() {
                    self.events.fail_waiting(
                        Flow::Restore,
                        operation,
                        DeviceTransferEvent::RestoreFailed,
                    );
                }
                result
            }
            DeviceTransferCommand::PushToRemote => self.start_sender().await,
            DeviceTransferCommand::ConfirmRestore
            | DeviceTransferCommand::ConfirmBackup
            | DeviceTransferCommand::CancelBackupRequest
            | DeviceTransferCommand::CancelRestoreRequest => {
                self.request_command_inner(command, None).await
            }
            DeviceTransferCommand::CancelRestore => {
                let mut state = self.state.lock().await;
                state.waiting_for_remote_push = None;
                if let Some(cancel) = state.receiver_cancel.take() {
                    cancel.cancel();
                }
                self.events.cancel(Flow::Restore);
                Ok(())
            }
            DeviceTransferCommand::CancelBackup => {
                let mut state = self.state.lock().await;
                if let Some(cancel) = state.sender_cancel.take() {
                    cancel.cancel();
                }
                self.events.cancel(Flow::Backup);
                Ok(())
            }
        }
    }

    pub async fn request_command(
        self: &Arc<Self>,
        command: DeviceTransferCommand,
        request_id: u64,
    ) -> Result<()> {
        if self.primary_session_id.is_none() {
            bail!("device transfer is unavailable without a primary session");
        }
        self.request_command_inner(command, Some(request_id)).await
    }

    async fn request_command_inner(
        self: &Arc<Self>,
        command: DeviceTransferCommand,
        expected: Option<u64>,
    ) -> Result<()> {
        match command {
            DeviceTransferCommand::ConfirmRestore => {
                let (remote, cancel) = {
                    let mut state = self.state.lock().await;
                    state.accept_restore(&self.events, expected)?
                };
                self.spawn_receiver(remote, cancel);
                Ok(())
            }
            DeviceTransferCommand::ConfirmBackup => {
                let (cancel, operation) = {
                    let mut state = self.state.lock().await;
                    if !self.events.consume_request(Flow::Backup, expected) {
                        bail!("device transfer request was replaced");
                    }
                    let cancel = replace_transfer(&mut state.sender_cancel);
                    let operation = self.events.begin(Flow::Backup, Some(cancel.clone()));
                    (cancel, operation)
                };
                self.start_registered_sender(cancel, operation).await
            }
            DeviceTransferCommand::CancelBackupRequest
            | DeviceTransferCommand::CancelRestoreRequest => {
                {
                    let mut state = self.state.lock().await;
                    let flow = if matches!(command, DeviceTransferCommand::CancelBackupRequest) {
                        Flow::Backup
                    } else {
                        Flow::Restore
                    };
                    if !self.events.consume_request(flow, expected) {
                        bail!("device transfer request was replaced");
                    }
                    if matches!(flow, Flow::Restore) {
                        state.remote_push = None;
                    }
                }
                self.send_remote(TransferCommand::simple(&self.device_id, "cancel"))
                    .await
            }
            _ => bail!("command does not address a device transfer request"),
        }
    }

    fn emit(&self, event: DeviceTransferEvent) {
        match event {
            DeviceTransferEvent::BackupRequestReceived => self.events.request(Flow::Restore),
            DeviceTransferEvent::RestoreRequestReceived => self.events.request(Flow::Backup),
            _ => unreachable!("worker events require operation ownership"),
        }
    }

    async fn fail_connection(&self, reason: ConnectionFailedReason) {
        let mut state = self.state.lock().await;
        state.cancel_resources();
        self.events.failure(reason);
    }

    async fn send_remote(&self, command: TransferCommand) -> Result<()> {
        self.message_sender
            .send_plain_json_to_session(
                DEVICE_TRANSFER_ACTION,
                serde_json::to_string(&command)?,
                self.primary_session_id.clone(),
            )
            .await
    }

    async fn handle_remote_content(self: &Arc<Self>, content: &str) -> Result<()> {
        if self.primary_session_id.is_none() || self.events.states().borrow().closed {
            return Ok(());
        }
        let command: TransferCommand = serde_json::from_str(content)?;
        if command.device_id == self.device_id {
            return Ok(());
        }
        if command.version != PROTOCOL_VERSION {
            self.fail_connection(ConnectionFailedReason::VersionNotMatched)
                .await;
            self.send_remote(TransferCommand::simple(&self.device_id, "cancel"))
                .await?;
            return Ok(());
        }
        match command.action.as_str() {
            "pull" => self.emit(DeviceTransferEvent::RestoreRequestReceived),
            "push" => {
                let key = command
                    .secret_key
                    .as_deref()
                    .ok_or_else(|| anyhow!("push command has no secret key"))?;
                let key = Base64::decode_vec(key)?;
                let key: [u8; KEY_SIZE] = key
                    .try_into()
                    .map_err(|_| anyhow!("invalid transfer key length"))?;
                let remote = RemotePushData {
                    ip: command
                        .ip
                        .ok_or_else(|| anyhow!("push command has no ip"))?,
                    port: command
                        .port
                        .ok_or_else(|| anyhow!("push command has no port"))?,
                    code: command
                        .code
                        .ok_or_else(|| anyhow!("push command has no code"))?,
                    key,
                };
                let mut state = self.state.lock().await;
                if self.events.states().borrow().closed {
                    return Ok(());
                }
                if let Some(operation) = state.waiting_for_remote_push.take() {
                    let cancel = Arc::new(CancellationToken::new());
                    if self.events.attach(Flow::Restore, operation, cancel.clone()) {
                        if let Some(previous) = state.receiver_cancel.replace(cancel.clone()) {
                            previous.cancel();
                        }
                        drop(state);
                        self.spawn_receiver(remote, cancel);
                        return Ok(());
                    }
                }
                state.remote_push = Some(remote);
                self.emit(DeviceTransferEvent::BackupRequestReceived);
            }
            "cancel" => {
                self.cancel_all().await;
            }
            other => log::warn!("unknown device transfer action: {other}"),
        }
        Ok(())
    }

    async fn start_sender(self: &Arc<Self>) -> Result<()> {
        // Register before the first fallible setup await, so a concurrent cancel
        // also owns an operation whose listener is still being created.
        let (cancel, operation) = {
            let mut state = self.state.lock().await;
            let cancel = replace_transfer(&mut state.sender_cancel);
            let operation = self.events.begin(Flow::Backup, Some(cancel.clone()));
            (cancel, operation)
        };
        self.start_registered_sender(cancel, operation).await
    }

    async fn start_registered_sender(
        self: &Arc<Self>,
        cancel: Arc<CancellationToken>,
        operation: u64,
    ) -> Result<()> {
        if operation == 0 {
            bail!("device transfer service is closed");
        }
        let setup = tokio::select! {
            biased;
            _ = cancel.cancelled() => None,
            setup = async {
                let listener = TcpListener::bind(("0.0.0.0", 0)).await?;
                let port = listener.local_addr()?.port();
                Ok::<_, anyhow::Error>((listener, port, first_ipv4_address()?, random_u16()? % 10_000, generate_transfer_key()?))
            } => Some(setup),
        };
        let (listener, port, ip, code, key) = match setup {
            Some(Ok(setup)) => setup,
            Some(Err(error)) => {
                finish_transfer(&mut self.state.lock().await.sender_cancel, &cancel);
                self.events.fail_waiting(
                    Flow::Backup,
                    operation,
                    DeviceTransferEvent::BackupFailed,
                );
                return Err(error);
            }
            None => {
                finish_transfer(&mut self.state.lock().await.sender_cancel, &cancel);
                return Ok(());
            }
        };
        {
            let state = self.state.lock().await;
            if cancel.is_cancelled()
                || !state
                    .sender_cancel
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &cancel))
            {
                return Ok(());
            }
            self.events.emit_for(
                Flow::Backup,
                &cancel,
                DeviceTransferEvent::BackupServerCreated,
            );
        }
        let service = self.clone();
        let worker_cancel = cancel.clone();
        crate::task::spawn(async move {
            let result = run_cancellable(
                &worker_cancel,
                service.run_sender(listener, code, key, worker_cancel.clone()),
            )
            .await;
            let mut state = service.state.lock().await;
            if finish_transfer(&mut state.sender_cancel, &worker_cancel)
                && !worker_cancel.is_cancelled()
            {
                match result {
                    Ok(()) => service.events.finish(
                        Flow::Backup,
                        &worker_cancel,
                        DeviceTransferEvent::BackupSucceed,
                    ),
                    Err(error) => {
                        log::error!("device transfer sender failed: {error:?}");
                        service.events.finish(
                            Flow::Backup,
                            &worker_cancel,
                            DeviceTransferEvent::BackupFailed,
                        );
                    }
                }
            }
        });
        let result = self
            .send_remote(TransferCommand::push(&self.device_id, ip, port, code, &key))
            .await;
        if result.is_err() {
            let current = finish_transfer(&mut self.state.lock().await.sender_cancel, &cancel);
            if current {
                self.events
                    .finish(Flow::Backup, &cancel, DeviceTransferEvent::BackupFailed);
            }
            cancel.cancel();
        }
        result
    }

    fn spawn_receiver(self: &Arc<Self>, remote: RemotePushData, cancel: Arc<CancellationToken>) {
        // Both approval paths register the token under the same lock that takes
        // the pending request. No worker can install a token after cancellation.
        let service = self.clone();
        crate::task::spawn(async move {
            let result =
                run_cancellable(&cancel, service.run_receiver(remote, cancel.clone())).await;
            let mut state = service.state.lock().await;
            if finish_transfer(&mut state.receiver_cancel, &cancel) && !cancel.is_cancelled() {
                match result {
                    Ok(()) => service.events.finish(
                        Flow::Restore,
                        &cancel,
                        DeviceTransferEvent::RestoreSucceed,
                    ),
                    Err(error) => {
                        log::error!("device transfer receiver failed: {error:?}");
                        service.events.finish(
                            Flow::Restore,
                            &cancel,
                            DeviceTransferEvent::RestoreFailed,
                        );
                    }
                }
            }
        });
    }

    async fn run_sender(
        &self,
        listener: TcpListener,
        verification_code: u16,
        key: [u8; KEY_SIZE],
        cancel: Arc<CancellationToken>,
    ) -> Result<()> {
        let (stream, _) = tokio::select! {
            accepted = listener.accept() => accepted?,
            _ = cancel.cancelled() => return Ok(()),
        };
        let (mut reader, mut writer) = stream.into_split();
        let packet = tokio::select! {
            packet = read_packet(&mut reader, &key, &self.account_data_dir) => packet?,
            _ = cancel.cancelled() => return Ok(()),
        };
        let Packet::Command(connect) = packet else {
            bail!("first transfer packet is not a command");
        };
        if connect.action != "connect" || connect.code != Some(verification_code) {
            bail!("device transfer verification failed");
        }

        emit_running(&self.events, &cancel, DeviceTransferEvent::BackupStart);
        let specs = record_specs();
        let mut total = 0u64;
        for spec in &specs {
            if !matches!(spec.kind, "inscription_item" | "inscription_collection") {
                total += self.count_records(spec).await?;
            }
        }
        total += crate::db::query_scalar::< i64>(
            "SELECT COUNT(*) FROM messages WHERE category IN ('SIGNAL_IMAGE', 'SIGNAL_VIDEO', 'SIGNAL_DATA', 'SIGNAL_AUDIO', 'PLAIN_IMAGE', 'PLAIN_VIDEO', 'PLAIN_DATA', 'PLAIN_AUDIO', 'ENCRYPTED_IMAGE', 'ENCRYPTED_VIDEO', 'ENCRYPTED_DATA', 'ENCRYPTED_AUDIO')",
        )
        .fetch_one(self.pool())
        .await?
        .max(0) as u64;
        write_command(
            &mut writer,
            &key,
            &TransferCommand::start(&self.device_id, total),
        )
        .await?;

        let started = Instant::now();
        let mut bytes_sent = 0u64;
        let mut records_sent = 0u64;
        let mut attachment_paths = std::collections::HashMap::<Uuid, PathBuf>::new();
        for spec in &specs {
            let mut offset = 0u64;
            loop {
                let records = self.select_records(spec, offset, 100).await?;
                if records.is_empty() {
                    break;
                }
                offset += records.len() as u64;
                for record in records {
                    tokio::select! {
                        result = write_json_record(&mut writer, &key, spec.kind, &record) => {
                            bytes_sent += result? as u64;
                        }
                        _ = cancel.cancelled() => return Ok(()),
                    }
                    emit_running(
                        &self.events,
                        &cancel,
                        DeviceTransferEvent::BackupNetworkSpeed(transfer_speed(
                            bytes_sent, started,
                        )),
                    );
                    records_sent += 1;
                    let progress = if total == 0 {
                        0.0
                    } else {
                        (records_sent as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                    };
                    emit_running(
                        &self.events,
                        &cancel,
                        DeviceTransferEvent::BackupProgress(progress),
                    );
                    if matches!(spec.kind, "message" | "transcript_message") {
                        if let Some(path) = self.attachment_for_record(spec.kind, &record).await? {
                            let message_id = record
                                .get("message_id")
                                .and_then(Value::as_str)
                                .ok_or_else(|| anyhow!("attachment record has no message id"))?;
                            let file_id = Uuid::parse_str(message_id)?;
                            if let Some(previous) = attachment_paths.get(&file_id) {
                                let equal = tokio::select! {
                                    equal = attachments::same_bytes(previous, &path) => equal?,
                                    _ = cancel.cancelled() => return Ok(()),
                                };
                                if !equal {
                                    bail!("same message UUID has conflicting attachment bytes");
                                }
                            } else {
                                attachment_paths.insert(file_id, path.clone());
                            }
                            bytes_sent += tokio::select! {
                                result = write_file_packet(&mut writer, &key, message_id, &path) => result? as u64,
                                _ = cancel.cancelled() => return Ok(()),
                            };
                        }
                    }
                }
                if !offset.is_multiple_of(100) {
                    break;
                }
            }
        }
        write_command(
            &mut writer,
            &key,
            &TransferCommand::simple(&self.device_id, "finish"),
        )
        .await?;

        loop {
            let packet = tokio::select! {
                packet = read_packet(&mut reader, &key, &self.account_data_dir) => packet?,
                _ = cancel.cancelled() => return Ok(()),
            };
            if let Packet::Command(command) = packet {
                if command.action == "progress" {
                    if let Some(progress) = command.progress {
                        emit_running(
                            &self.events,
                            &cancel,
                            DeviceTransferEvent::BackupProgress(progress),
                        );
                    }
                } else if command.action == "finish" {
                    return Ok(());
                } else if command.action == "close" || command.action == "cancel" {
                    bail!("receiver closed transfer");
                }
            }
        }
    }

    async fn run_receiver(
        &self,
        remote: RemotePushData,
        cancel: Arc<CancellationToken>,
    ) -> Result<()> {
        let stream = tokio::select! {
            stream = crate::time::timeout(Duration::from_secs(60), TcpStream::connect((&*remote.ip, remote.port))) => stream??,
            _ = cancel.cancelled() => return Ok(()),
        };
        let (mut reader, mut writer) = stream.into_split();
        write_command(
            &mut writer,
            &remote.key,
            &TransferCommand::connect(&self.device_id, remote.code, &self.user_id),
        )
        .await?;
        emit_running(&self.events, &cancel, DeviceTransferEvent::RestoreConnected);

        let mut total = 0u64;
        let mut progress_count = 0u64;
        let mut last_progress_sent = Instant::now() - Duration::from_secs(1);
        let started = Instant::now();
        let mut bytes_received = 0u64;
        loop {
            let packet = tokio::select! {
                packet = read_packet(&mut reader, &remote.key, &self.account_data_dir) => packet?,
                _ = cancel.cancelled() => return Ok(()),
            };
            bytes_received += packet.wire_size() as u64;
            emit_running(
                &self.events,
                &cancel,
                DeviceTransferEvent::RestoreNetworkSpeed(transfer_speed(bytes_received, started)),
            );
            match packet {
                Packet::Command(command) if command.action == "start" => {
                    total = command.total.unwrap_or_default();
                    progress_count = 0;
                    emit_running(&self.events, &cancel, DeviceTransferEvent::RestoreStart);
                    emit_running(
                        &self.events,
                        &cancel,
                        DeviceTransferEvent::RestoreProgress(0.0),
                    );
                }
                Packet::Command(command) if command.action == "finish" => {
                    emit_running(
                        &self.events,
                        &cancel,
                        DeviceTransferEvent::RestoreProgress(100.0),
                    );
                    write_command(
                        &mut writer,
                        &remote.key,
                        &TransferCommand::progress(&self.device_id, 100.0),
                    )
                    .await?;
                    write_command(
                        &mut writer,
                        &remote.key,
                        &TransferCommand::simple(&self.device_id, "finish"),
                    )
                    .await?;
                    self.conversation_changes.notify_all();
                    return Ok(());
                }
                Packet::Command(command)
                    if command.action == "close" || command.action == "cancel" =>
                {
                    bail!("sender closed transfer")
                }
                Packet::Command(_) => {}
                Packet::Json { kind, data, .. } => {
                    self.insert_record(&kind, &data)
                        .await
                        .with_context(|| format!("insert transferred {kind} record"))?;
                    progress_count += 1;
                    let progress = if total == 0 {
                        0.0
                    } else {
                        (progress_count as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                    };
                    emit_running(
                        &self.events,
                        &cancel,
                        DeviceTransferEvent::RestoreProgress(progress),
                    );
                    if last_progress_sent.elapsed() >= Duration::from_millis(200) {
                        write_command(
                            &mut writer,
                            &remote.key,
                            &TransferCommand::progress(&self.device_id, progress),
                        )
                        .await?;
                        last_progress_sent = Instant::now();
                    }
                }
                Packet::File {
                    message_id, file, ..
                } => {
                    tokio::select! {
                        result = self.insert_attachment(&message_id, file.path()) => result?,
                        _ = cancel.cancelled() => return Ok(()),
                    }
                }
            }
        }
    }
}

/// Cancelling the owning operation drops its socket and staging files, including
/// when a peer stops reading a control packet rather than an attachment chunk.
#[cfg(not(target_arch = "wasm32"))]
async fn run_cancellable(
    cancel: &CancellationToken,
    operation: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Ok(()),
        result = operation => result,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn transfer_speed(bytes: u64, started: Instant) -> f64 {
    bytes as f64 / started.elapsed().as_secs_f64().max(0.001)
}

#[cfg(not(target_arch = "wasm32"))]
fn first_ipv4_address() -> Result<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_loopback() => Ok(ip.to_string()),
        _ => bail!("no local IPv4 address"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn random_u16() -> Result<u16> {
    let mut bytes = [0u8; 2];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| anyhow!("secure random failed"))?;
    Ok(u16::from_be_bytes(bytes))
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_transfer_key() -> Result<[u8; KEY_SIZE]> {
    let mut input = [0u8; 32];
    SystemRandom::new()
        .fill(&mut input)
        .map_err(|_| anyhow!("secure random failed"))?;
    hkdf_sha256(&input, b"Mixin Device Transfer")
}

#[cfg(not(target_arch = "wasm32"))]
fn hkdf_sha256(input: &[u8], info: &[u8]) -> Result<[u8; KEY_SIZE]> {
    let salt = [0u8; 32];
    let mut extract = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&salt)
        .map_err(|_| anyhow!("invalid HKDF salt"))?;
    extract.update(input);
    let prk = extract.finalize().into_bytes();
    let mut output = [0u8; KEY_SIZE];
    let mut previous = Vec::new();
    for index in 1..=2u8 {
        let mut expand = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&prk)
            .map_err(|_| anyhow!("invalid HKDF key"))?;
        expand.update(&previous);
        expand.update(info);
        expand.update(&[index]);
        previous = expand.finalize().into_bytes().to_vec();
        let offset = (index as usize - 1) * 32;
        output[offset..offset + 32].copy_from_slice(&previous);
    }
    Ok(output)
}

#[cfg(not(target_arch = "wasm32"))]
struct RecordSpec {
    kind: &'static str,
    table: &'static str,
    columns: &'static [&'static str],
    boolean_columns: &'static [&'static str],
}

#[cfg(not(target_arch = "wasm32"))]
fn record_specs() -> Vec<RecordSpec> {
    vec![
        spec(
            "conversation",
            "conversations",
            &[
                "conversation_id",
                "owner_id",
                "category",
                "name",
                "announcement",
                "code_url",
                "pay_type",
                "created_at",
                "pin_time",
                "last_message_id",
                "last_message_created_at",
                "last_read_message_id",
                "unseen_message_count",
                "status",
                "mute_until",
                "expire_in",
            ],
        ),
        spec(
            "participant",
            "participants",
            &["conversation_id", "user_id", "role", "created_at"],
        ),
        spec_bool(
            "user",
            "users",
            &[
                "user_id",
                "identity_number",
                "relationship",
                "full_name",
                "avatar_url",
                "phone",
                "is_verified",
                "created_at",
                "mute_until",
                "has_pin",
                "app_id",
                "biography",
                "is_scam",
                "code_url",
                "code_id",
                "is_deactivated",
                "membership",
            ],
            &["is_verified", "has_pin", "is_scam", "is_deactivated"],
        ),
        spec(
            "app",
            "apps",
            &[
                "app_id",
                "app_number",
                "home_uri",
                "redirect_uri",
                "name",
                "icon_url",
                "category",
                "description",
                "app_secret",
                "capabilities",
                "creator_id",
                "resource_patterns",
                "updated_at",
            ],
        ),
        spec(
            "sticker",
            "stickers",
            &[
                "sticker_id",
                "album_id",
                "name",
                "asset_url",
                "asset_type",
                "asset_width",
                "asset_height",
                "created_at",
                "last_use_at",
            ],
        ),
        spec(
            "asset",
            "assets",
            &[
                "asset_id",
                "symbol",
                "name",
                "icon_url",
                "balance",
                "destination",
                "tag",
                "price_btc",
                "price_usd",
                "chain_id",
                "change_usd",
                "change_btc",
                "confirmations",
                "asset_key",
                "reserve",
            ],
        ),
        spec(
            "token",
            "tokens",
            &[
                "asset_id",
                "kernel_asset_id",
                "symbol",
                "name",
                "icon_url",
                "price_btc",
                "price_usd",
                "chain_id",
                "change_usd",
                "change_btc",
                "confirmations",
                "asset_key",
                "dust",
                "collection_hash",
            ],
        ),
        spec(
            "snapshot",
            "snapshots",
            &[
                "snapshot_id",
                "trace_id",
                "type",
                "asset_id",
                "amount",
                "created_at",
                "opponent_id",
                "transaction_hash",
                "sender",
                "receiver",
                "memo",
                "confirmations",
            ],
        ),
        spec(
            "safe_snapshot",
            "safe_snapshots",
            &[
                "snapshot_id",
                "type",
                "asset_id",
                "amount",
                "user_id",
                "opponent_id",
                "memo",
                "transaction_hash",
                "created_at",
                "trace_id",
                "confirmations",
                "opening_balance",
                "closing_balance",
                "withdrawal",
                "deposit",
                "inscription_hash",
            ],
        ),
        spec(
            "inscription_collection",
            "inscription_collections",
            &[
                "collection_hash",
                "supply",
                "unit",
                "symbol",
                "name",
                "icon_url",
                "created_at",
                "updated_at",
            ],
        ),
        spec(
            "inscription_item",
            "inscription_items",
            &[
                "inscription_hash",
                "collection_hash",
                "sequence",
                "content_type",
                "content_url",
                "occupied_by",
                "occupied_at",
                "created_at",
                "updated_at",
            ],
        ),
        spec(
            "transcript_message",
            "transcript_messages",
            &[
                "transcript_id",
                "message_id",
                "user_id",
                "user_full_name",
                "category",
                "created_at",
                "content",
                "media_url",
                "media_name",
                "media_size",
                "media_width",
                "media_height",
                "media_mime_type",
                "media_duration",
                "media_status",
                "media_waveform",
                "thumb_image",
                "thumb_url",
                "media_key",
                "media_digest",
                "media_created_at",
                "sticker_id",
                "shared_user_id",
                "mentions",
                "quote_id",
                "quote_content",
                "caption",
            ],
        ),
        spec(
            "pin_message",
            "pin_messages",
            &["message_id", "conversation_id", "created_at"],
        ),
        spec(
            "message",
            "messages",
            &[
                "message_id",
                "conversation_id",
                "user_id",
                "category",
                "content",
                "media_url",
                "media_mime_type",
                "media_size",
                "media_duration",
                "media_width",
                "media_height",
                "media_hash",
                "thumb_image",
                "media_key",
                "media_digest",
                "media_status",
                "status",
                "created_at",
                "action",
                "participant_id",
                "snapshot_id",
                "hyperlink",
                "name",
                "album_id",
                "sticker_id",
                "shared_user_id",
                "media_waveform",
                "quote_message_id",
                "quote_content",
                "thumb_url",
                "caption",
            ],
        ),
        spec_bool(
            "message_mention",
            "message_mentions",
            &["message_id", "conversation_id", "has_read"],
            &["has_read"],
        ),
        spec(
            "expired_message",
            "expired_messages",
            &["message_id", "expire_in", "expire_at"],
        ),
    ]
}

#[cfg(not(target_arch = "wasm32"))]
fn spec(kind: &'static str, table: &'static str, columns: &'static [&'static str]) -> RecordSpec {
    RecordSpec {
        kind,
        table,
        columns,
        boolean_columns: &[],
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spec_bool(
    kind: &'static str,
    table: &'static str,
    columns: &'static [&'static str],
    boolean_columns: &'static [&'static str],
) -> RecordSpec {
    RecordSpec {
        kind,
        table,
        columns,
        boolean_columns,
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DeviceTransferService {
    fn pool(&self) -> &sqlx::SqlitePool {
        &self.database.user_dao.0
    }

    async fn count_records(&self, spec: &RecordSpec) -> Result<u64> {
        let count = crate::db::query_scalar::<i64>(AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM {}",
            spec.table
        )))
        .fetch_one(self.pool())
        .await?;
        Ok(count.max(0) as u64)
    }

    async fn select_records(
        &self,
        spec: &RecordSpec,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Map<String, Value>>> {
        let pairs = spec
            .columns
            .iter()
            .map(|column| {
                let expression =
                    if spec.kind == "message" && matches!(*column, "media_key" | "media_digest") {
                        format!("hex(\"{column}\")")
                    } else {
                        format!("\"{column}\"")
                    };
                format!("'{column}', {expression}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT json_object({pairs}) FROM {} LIMIT ? OFFSET ?",
            spec.table
        );
        let rows = crate::db::query_scalar::<String>(AssertSqlSafe(sql))
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(self.pool())
            .await?;
        rows.into_iter()
            .map(|row| {
                let mut data = serde_json::from_str::<Value>(&row)?
                    .as_object()
                    .cloned()
                    .ok_or_else(|| anyhow!("database row is not an object"))?;
                for column in spec.boolean_columns {
                    if let Some(value) = data.get_mut(*column) {
                        if !value.is_null() {
                            *value = Value::Bool(value.as_i64().unwrap_or_default() != 0);
                        }
                    }
                }
                normalize_outgoing_record(spec.kind, &mut data);
                if matches!(spec.kind, "message" | "transcript_message") {
                    self.normalize_attachment_url(spec.kind, &mut data)?;
                }
                Ok(data)
            })
            .collect()
    }

    async fn insert_record(&self, kind: &str, data: &Map<String, Value>) -> Result<()> {
        let specs = record_specs();
        let Some(spec) = specs.iter().find(|spec| spec.kind == kind) else {
            log::warn!("ignore unsupported device transfer record type: {kind}");
            return Ok(());
        };
        if kind == "message"
            && data
                .get("category")
                .and_then(Value::as_str)
                .is_some_and(|category| category.starts_with("KRAKEN_"))
        {
            return Ok(());
        }
        let mut normalized = data.clone();
        normalize_incoming_record(kind, &mut normalized);
        if matches!(kind, "message" | "transcript_message") {
            self.normalize_attachment_url(kind, &mut normalized)?;
        }
        let values = spec
            .columns
            .iter()
            .filter_map(|column| normalized.get(*column).map(|value| (*column, value)))
            .collect::<Vec<_>>();
        if values.is_empty() {
            return Ok(());
        }
        let verb = if matches!(kind, "asset" | "token" | "sticker") {
            "INSERT OR REPLACE"
        } else {
            "INSERT OR IGNORE"
        };
        let mut builder = QueryBuilder::new(format!("{verb} INTO {} (", spec.table));
        {
            let mut columns = builder.separated(", ");
            for (column, _) in &values {
                columns.push(format!("\"{column}\""));
            }
        }
        builder.push(") VALUES (");
        {
            let mut binds = builder.separated(", ");
            for (column, value) in &values {
                if kind == "message" && matches!(*column, "media_key" | "media_digest") {
                    match value {
                        Value::Null => binds.push_bind(Option::<Vec<u8>>::None),
                        Value::String(value) => binds.push_bind(
                            Base64::decode_vec(value)
                                .with_context(|| format!("decode transferred message {column}"))?,
                        ),
                        _ => bail!("invalid transferred message {column}"),
                    };
                    continue;
                }
                match value {
                    Value::Null => binds.push_bind(Option::<String>::None),
                    Value::Bool(value) => binds.push_bind(*value),
                    Value::Number(value) if value.is_i64() => {
                        binds.push_bind(value.as_i64().unwrap())
                    }
                    Value::Number(value) if value.is_u64() => {
                        binds.push_bind(value.as_u64().unwrap() as i64)
                    }
                    Value::Number(value) => binds.push_bind(value.as_f64().unwrap()),
                    Value::String(value) => binds.push_bind(value.clone()),
                    Value::Array(_) | Value::Object(_) => {
                        binds.push_bind(serde_json::to_string(value)?)
                    }
                };
            }
        }
        builder.push(")");
        let result = builder.build().execute(self.pool()).await?;
        if kind == "message" && result.rows_affected() > 0 {
            let message_id = normalized
                .get("message_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("transferred message has no message_id"))?;
            let conversation_id = normalized
                .get("conversation_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("transferred message has no conversation_id"))?;
            if let Some(content) = transferred_message_fts_content(&normalized) {
                self.database
                    .message_fts_dao
                    .upsert(message_id, conversation_id, &content)
                    .await?;
            }
        }
        Ok(())
    }

    async fn record_by_message_id(
        &self,
        table: &str,
        message_id: &str,
    ) -> Result<Option<Map<String, Value>>> {
        let sql = if table == "transcript_messages" {
            "SELECT category, '' AS conversation_id, message_id, media_mime_type, media_name AS name FROM transcript_messages WHERE message_id = ? LIMIT 1"
        } else {
            "SELECT category, conversation_id, message_id, media_mime_type, name FROM messages WHERE message_id = ? LIMIT 1"
        };
        let row = crate::db::query(sql)
            .bind(message_id)
            .fetch_optional(self.pool())
            .await?;
        row.map(|row| {
            let mut data = Map::new();
            for column in [
                "category",
                "conversation_id",
                "message_id",
                "media_mime_type",
                "name",
            ] {
                if let Ok(value) = row.try_get::<Option<String>, _>(column) {
                    data.insert(column.to_string(), value.map_or(Value::Null, Value::String));
                }
            }
            Ok(data)
        })
        .transpose()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn transferred_message_fts_content(record: &Map<String, Value>) -> Option<String> {
    message_fts_content(&Message {
        category: record.get("category")?.as_str()?.to_string(),
        content: record
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_string),
        name: record
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: sdk::blaze_message::MessageStatus::Read,
        ..Message::default()
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_outgoing_record(kind: &str, data: &mut Map<String, Value>) {
    if kind != "expired_message" {
        for key in [
            "created_at",
            "updated_at",
            "pin_time",
            "last_use_at",
            "last_message_created_at",
            "mute_until",
            "media_created_at",
        ] {
            normalize_millis_date(data, key);
        }
    }
    if (kind == "message" || kind == "transcript_message")
        && data.get("media_status").and_then(Value::as_str) == Some("PENDING")
    {
        data.insert(
            "media_status".to_string(),
            Value::String("CANCELED".to_string()),
        );
    }
    if kind == "message" {
        for key in ["media_key", "media_digest"] {
            if let Some(Value::String(value)) = data.get(key) {
                if let Ok(bytes) = hex::decode(value) {
                    data.insert(
                        key.to_string(),
                        Value::String(Base64::encode_string(&bytes)),
                    );
                }
            }
        }
        normalize_quote_content(data);
    }
    if kind == "safe_snapshot" {
        for key in ["withdrawal", "deposit"] {
            if let Some(Value::String(value)) = data.get(key) {
                if let Ok(parsed) = serde_json::from_str(value) {
                    data.insert(key.to_string(), parsed);
                }
            }
        }
    }
    if kind == "app" {
        for key in ["capabilities", "resource_patterns"] {
            if let Some(Value::String(value)) = data.get(key) {
                if let Some(array) = parse_string_list(value) {
                    data.insert(key.to_string(), Value::Array(array));
                }
            }
        }
    }
    if kind == "token" {
        data.entry("precision".to_string())
            .or_insert(Value::Number(8.into()));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_incoming_record(kind: &str, data: &mut Map<String, Value>) {
    if kind == "conversation" {
        if let Some(Value::String(value)) = data.get("last_message_created_at") {
            if let Ok(date) = chrono::DateTime::parse_from_rfc3339(value) {
                data.insert(
                    "last_message_created_at".to_string(),
                    Value::Number(date.timestamp_millis().into()),
                );
            }
        }
    }
    if kind == "message" {
        data.insert("status".to_string(), Value::String("READ".to_string()));
    }
    if (kind == "message" || kind == "transcript_message")
        && data.get("media_status").and_then(Value::as_str) == Some("PENDING")
    {
        data.insert(
            "media_status".to_string(),
            Value::String("CANCELED".to_string()),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_millis_date(data: &mut Map<String, Value>, key: &str) {
    let Some(milliseconds) = data.get(key).and_then(Value::as_i64) else {
        return;
    };
    let Some(date) = chrono::DateTime::from_timestamp_millis(milliseconds) else {
        return;
    };
    data.insert(key.to_string(), Value::String(date.to_rfc3339()));
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_quote_content(data: &mut Map<String, Value>) {
    let Some(Value::String(content)) = data.get("quote_content") else {
        return;
    };
    let Ok(mut quote) = serde_json::from_str::<Value>(content) else {
        return;
    };
    if let Some(object) = quote.as_object_mut() {
        if let Some(mut created_at) = object
            .get("created_at")
            .or_else(|| object.get("createdAt"))
            .cloned()
        {
            if let Some(milliseconds) = created_at.as_i64() {
                if let Some(date) = chrono::DateTime::from_timestamp_millis(milliseconds) {
                    created_at = Value::String(date.to_rfc3339());
                }
            }
            object.insert("created_at".to_string(), created_at.clone());
            object.insert("createdAt".to_string(), created_at);
        }
    }
    if let Ok(content) = serde_json::to_string(&quote) {
        data.insert("quote_content".to_string(), Value::String(content));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_string_list(value: &str) -> Option<Vec<Value>> {
    if let Ok(Value::Array(values)) = serde_json::from_str(value) {
        return Some(values);
    }
    let value = value.trim().strip_prefix('[')?.strip_suffix(']')?;
    Some(
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(value.to_string()))
            .collect(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn message_from_record(record: &Map<String, Value>) -> Result<Message> {
    let get = |name: &str| record.get(name).and_then(Value::as_str).unwrap_or_default();
    Ok(Message {
        message_id: get("message_id").to_string(),
        conversation_id: get("conversation_id").to_string(),
        category: get("category").to_string(),
        media_mime_type: record
            .get("media_mime_type")
            .and_then(Value::as_str)
            .map(str::to_string),
        name: record
            .get("name")
            .or_else(|| record.get("media_name"))
            .and_then(Value::as_str)
            .map(str::to_string),
        media_status: MediaStatus::Done,
        ..Message::default()
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "device_transfer/runtime_tests.rs"]
mod runtime_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelling_registered_transfer_before_worker_poll_prevents_transfer_work() {
        let mut slot = None;
        let cancel = replace_transfer(&mut slot);
        let worker_cancel = cancel.clone();
        let (release, ready) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            ready.await.unwrap();
            worker_cancel.is_cancelled()
        });

        slot.take().expect("registered before spawning").cancel();
        release.send(()).unwrap();
        assert!(tokio::time::timeout(Duration::from_secs(1), worker)
            .await
            .unwrap()
            .unwrap());
        assert!(cancel.is_cancelled());
        assert!(slot.is_none());
    }

    #[tokio::test]
    async fn old_worker_completion_preserves_replacement_and_its_cancellation_handle() {
        let slot = Arc::new(Mutex::new(None));
        let first = replace_transfer(&mut *slot.lock().await);
        let worker_slot = slot.clone();
        let worker_cancel = first.clone();
        let (release, ready) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            ready.await.unwrap();
            finish_transfer(&mut *worker_slot.lock().await, &worker_cancel)
        });

        let replacement = replace_transfer(&mut *slot.lock().await);
        assert!(first.is_cancelled());
        release.send(()).unwrap();
        assert!(!tokio::time::timeout(Duration::from_secs(1), worker)
            .await
            .unwrap()
            .unwrap());

        let remaining = slot
            .lock()
            .await
            .take()
            .expect("replacement still owns the slot");
        assert!(Arc::ptr_eq(&remaining, &replacement));
        remaining.cancel();
        assert!(replacement.is_cancelled());
    }

    #[test]
    fn completing_current_transfer_releases_only_its_own_slot() {
        let mut slot = None;
        let cancel = replace_transfer(&mut slot);
        assert!(finish_transfer(&mut slot, &cancel));
        assert!(slot.is_none());
        assert!(!cancel.is_cancelled());
        assert!(!finish_transfer(&mut slot, &cancel));
    }

    #[test]
    fn replacing_transfer_suppresses_old_start_progress_and_speed_events() {
        let mut slot = None;
        let previous = replace_transfer(&mut slot);
        let current = replace_transfer(&mut slot);
        let events = TransferEvents::default();
        events.begin(Flow::Restore, Some(current.clone()));
        let mut receiver = events.subscribe();
        for event in [
            DeviceTransferEvent::BackupStart,
            DeviceTransferEvent::RestoreConnected,
            DeviceTransferEvent::RestoreStart,
            DeviceTransferEvent::BackupProgress(50.0),
            DeviceTransferEvent::RestoreProgress(50.0),
            DeviceTransferEvent::BackupNetworkSpeed(1024.0),
            DeviceTransferEvent::RestoreNetworkSpeed(1024.0),
        ] {
            emit_running(&events, &previous, event);
        }
        assert!(matches!(
            receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        emit_running(
            &events,
            &current,
            DeviceTransferEvent::RestoreProgress(25.0),
        );
        let DeviceTransferEvent::RestoreProgress(progress) = receiver.try_recv().unwrap() else {
            panic!("expected current transfer progress");
        };
        assert_eq!(progress, 25.0);
        current.cancel();
    }

    #[test]
    fn transfer_crypto_round_trip() {
        let key = [7u8; 32];
        let iv = [9u8; 16];
        let encrypted = encrypt_cbc(&key, &iv, b"device transfer").unwrap();
        assert_eq!(
            decrypt_cbc(&key, &iv, &encrypted).unwrap(),
            b"device transfer"
        );
    }

    #[test]
    fn hkdf_has_expected_size_and_halves() {
        let key = hkdf_sha256(&[1u8; 32], b"Mixin Device Transfer").unwrap();
        assert_ne!(&key[..32], &key[32..]);
    }
}
