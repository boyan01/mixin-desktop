use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use aes::cipher::{Block, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use aes::Aes256;
use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Encoding};
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::Sha256;
use sqlx::{AssertSqlSafe, QueryBuilder, Row, Sqlite};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, watch, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sdk::message_category::MessageCategory;

use crate::core::attachment::{attachment_path, transcript_attachment_path};
use crate::core::message::sender::MessageSender;
use crate::db::mixin::message::{MediaStatus, Message};
use crate::db::mixin::message_fts::message_fts_content;
use crate::db::MixinDatabase;

pub const DEVICE_TRANSFER_ACTION: &str = "DEVICE_TRANSFER";
const PROTOCOL_VERSION: i32 = 3;
const PACKET_COMMAND: u8 = 1;
const PACKET_JSON: u8 = 2;
const PACKET_FILE: u8 = 3;
const HMAC_SIZE: usize = 32;
const IV_SIZE: usize = 16;
const KEY_SIZE: usize = 64;
const JSON_PACKET_LIMIT: usize = 500 * 1024;

#[derive(Clone, Debug)]
pub struct DeviceTransferControlEvent {
    pub content: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeviceTransferEvent {
    pub kind: String,
    pub value: Option<f64>,
    pub reason: Option<String>,
}

impl DeviceTransferEvent {
    fn simple(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            value: None,
            reason: None,
        }
    }

    fn value(kind: &str, value: f64) -> Self {
        Self {
            kind: kind.to_string(),
            value: Some(value),
            reason: None,
        }
    }

    fn failed(reason: &str) -> Self {
        Self {
            kind: "connection_failed".to_string(),
            value: None,
            reason: Some(reason.to_string()),
        }
    }
}

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

fn desktop_platform() -> String {
    "desktop".to_string()
}

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

#[derive(Clone)]
struct RemotePushData {
    ip: String,
    port: u16,
    code: u16,
    key: [u8; KEY_SIZE],
}

#[derive(Default)]
struct TransferState {
    waiting_for_remote_push: bool,
    remote_push: Option<RemotePushData>,
    sender_cancel: Option<CancellationToken>,
    receiver_cancel: Option<CancellationToken>,
}

pub struct DeviceTransferService {
    database: Arc<MixinDatabase>,
    message_sender: Arc<MessageSender>,
    user_id: String,
    primary_session_id: Option<String>,
    device_id: String,
    account_data_dir: PathBuf,
    state: Mutex<TransferState>,
    events: broadcast::Sender<DeviceTransferEvent>,
    conversation_changes: watch::Sender<u64>,
}

impl DeviceTransferService {
    pub fn new(
        database: Arc<MixinDatabase>,
        message_sender: Arc<MessageSender>,
        user_id: String,
        session_id: String,
        primary_session_id: Option<String>,
        account_data_dir: PathBuf,
        conversation_changes: watch::Sender<u64>,
    ) -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        Arc::new(Self {
            database,
            message_sender,
            user_id,
            primary_session_id,
            device_id: session_id,
            account_data_dir,
            state: Mutex::new(TransferState::default()),
            events,
            conversation_changes,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DeviceTransferEvent> {
        self.events.subscribe()
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
                        self.emit(DeviceTransferEvent::failed("unknown"));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("device transfer command stream lagged by {skipped}");
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    pub async fn command(self: &Arc<Self>, command: &str) -> Result<()> {
        match command {
            "pull_to_remote" => {
                self.state.lock().await.waiting_for_remote_push = true;
                self.send_remote(TransferCommand::pull(&self.device_id))
                    .await
            }
            "push_to_remote" | "confirm_backup" => self.start_sender().await,
            "confirm_restore" => {
                let remote = self
                    .state
                    .lock()
                    .await
                    .remote_push
                    .take()
                    .ok_or_else(|| anyhow!("no pending remote transfer"))?;
                self.start_receiver(remote);
                Ok(())
            }
            "cancel_restore" => {
                let mut state = self.state.lock().await;
                state.waiting_for_remote_push = false;
                if let Some(cancel) = state.receiver_cancel.take() {
                    cancel.cancel();
                }
                Ok(())
            }
            "cancel_backup" => {
                if let Some(cancel) = self.state.lock().await.sender_cancel.take() {
                    cancel.cancel();
                }
                Ok(())
            }
            "cancel_backup_request" | "cancel_restore_request" => {
                self.state.lock().await.remote_push = None;
                self.send_remote(TransferCommand::simple(&self.device_id, "cancel"))
                    .await
            }
            _ => bail!("unknown device transfer command: {command}"),
        }
    }

    fn emit(&self, event: DeviceTransferEvent) {
        let _ = self.events.send(event);
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
        let command: TransferCommand = serde_json::from_str(content)?;
        if command.device_id == self.device_id {
            return Ok(());
        }
        if command.version != PROTOCOL_VERSION {
            self.emit(DeviceTransferEvent::failed("version_not_matched"));
            self.send_remote(TransferCommand::simple(&self.device_id, "cancel"))
                .await?;
            return Ok(());
        }
        match command.action.as_str() {
            "pull" => self.emit(DeviceTransferEvent::simple("restore_request_received")),
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
                if state.waiting_for_remote_push {
                    state.waiting_for_remote_push = false;
                    drop(state);
                    self.start_receiver(remote);
                } else {
                    state.remote_push = Some(remote);
                    drop(state);
                    self.emit(DeviceTransferEvent::simple("backup_request_received"));
                }
            }
            "cancel" => {
                let mut state = self.state.lock().await;
                if let Some(cancel) = state.sender_cancel.take() {
                    cancel.cancel();
                }
                if let Some(cancel) = state.receiver_cancel.take() {
                    cancel.cancel();
                }
                state.remote_push = None;
                state.waiting_for_remote_push = false;
            }
            other => log::warn!("unknown device transfer action: {other}"),
        }
        Ok(())
    }

    async fn start_sender(self: &Arc<Self>) -> Result<()> {
        if let Some(cancel) = self.state.lock().await.sender_cancel.take() {
            cancel.cancel();
        }
        let listener = TcpListener::bind(("0.0.0.0", 0)).await?;
        let port = listener.local_addr()?.port();
        let ip = first_ipv4_address()?;
        let code = random_u16()? % 10_000;
        let key = generate_transfer_key()?;
        let cancel = CancellationToken::new();
        self.state.lock().await.sender_cancel = Some(cancel.clone());
        self.emit(DeviceTransferEvent::simple("backup_server_created"));

        let service = self.clone();
        tokio::spawn(async move {
            let result = service.run_sender(listener, code, key, cancel).await;
            if let Err(error) = result {
                log::error!("device transfer sender failed: {error:?}");
                service.emit(DeviceTransferEvent::simple("backup_failed"));
            }
            service.state.lock().await.sender_cancel = None;
        });

        self.send_remote(TransferCommand::push(&self.device_id, ip, port, code, &key))
            .await
    }

    fn start_receiver(self: &Arc<Self>, remote: RemotePushData) {
        let service = self.clone();
        tokio::spawn(async move {
            if let Some(cancel) = service.state.lock().await.receiver_cancel.take() {
                cancel.cancel();
            }
            let cancel = CancellationToken::new();
            service.state.lock().await.receiver_cancel = Some(cancel.clone());
            let result = service.run_receiver(remote, cancel).await;
            if let Err(error) = result {
                log::error!("device transfer receiver failed: {error:?}");
                service.emit(DeviceTransferEvent::simple("restore_failed"));
            }
            service.state.lock().await.receiver_cancel = None;
        });
    }

    async fn run_sender(
        &self,
        listener: TcpListener,
        verification_code: u16,
        key: [u8; KEY_SIZE],
        cancel: CancellationToken,
    ) -> Result<()> {
        let (stream, _) = tokio::select! {
            accepted = listener.accept() => accepted?,
            _ = cancel.cancelled() => return Ok(()),
        };
        let (mut reader, mut writer) = stream.into_split();
        let packet = tokio::select! {
            packet = read_packet(&mut reader, &key) => packet?,
            _ = cancel.cancelled() => return Ok(()),
        };
        let Packet::Command(connect) = packet else {
            bail!("first transfer packet is not a command");
        };
        if connect.action != "connect" || connect.code != Some(verification_code) {
            bail!("device transfer verification failed");
        }

        self.emit(DeviceTransferEvent::simple("backup_start"));
        let specs = record_specs();
        let mut total = 0u64;
        for spec in &specs {
            if !matches!(spec.kind, "inscription_item" | "inscription_collection") {
                total += self.count_records(spec).await?;
            }
        }
        total += sqlx::query_scalar::<_, i64>(
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
                    self.emit(DeviceTransferEvent::value(
                        "backup_network_speed",
                        transfer_speed(bytes_sent, started),
                    ));
                    records_sent += 1;
                    let progress = if total == 0 {
                        0.0
                    } else {
                        (records_sent as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                    };
                    self.emit(DeviceTransferEvent::value("backup_progress", progress));
                    if matches!(spec.kind, "message" | "transcript_message") {
                        if let Some(path) = self.attachment_for_record(spec.kind, &record).await? {
                            let message_id = record
                                .get("message_id")
                                .and_then(Value::as_str)
                                .ok_or_else(|| anyhow!("attachment record has no message id"))?;
                            bytes_sent += write_file_packet(&mut writer, &key, message_id, &path)
                                .await? as u64;
                        }
                    }
                }
                if offset % 100 != 0 {
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
                packet = read_packet(&mut reader, &key) => packet?,
                _ = cancel.cancelled() => return Ok(()),
            };
            if let Packet::Command(command) = packet {
                if command.action == "progress" {
                    if let Some(progress) = command.progress {
                        self.emit(DeviceTransferEvent::value("backup_progress", progress));
                    }
                } else if command.action == "finish" {
                    self.emit(DeviceTransferEvent::simple("backup_succeed"));
                    return Ok(());
                } else if command.action == "close" || command.action == "cancel" {
                    bail!("receiver closed transfer");
                }
            }
        }
    }

    async fn run_receiver(&self, remote: RemotePushData, cancel: CancellationToken) -> Result<()> {
        let stream = tokio::select! {
            stream = tokio::time::timeout(Duration::from_secs(60), TcpStream::connect((&*remote.ip, remote.port))) => stream??,
            _ = cancel.cancelled() => return Ok(()),
        };
        let (mut reader, mut writer) = stream.into_split();
        write_command(
            &mut writer,
            &remote.key,
            &TransferCommand::connect(&self.device_id, remote.code, &self.user_id),
        )
        .await?;
        self.emit(DeviceTransferEvent::simple("restore_connected"));

        let mut total = 0u64;
        let mut progress_count = 0u64;
        let mut last_progress_sent = Instant::now() - Duration::from_secs(1);
        let started = Instant::now();
        let mut bytes_received = 0u64;
        loop {
            let packet = tokio::select! {
                packet = read_packet(&mut reader, &remote.key) => packet?,
                _ = cancel.cancelled() => {
                    let _ = write_command(&mut writer, &remote.key, &TransferCommand::simple(&self.device_id, "close")).await;
                    return Ok(());
                }
            };
            bytes_received += packet.wire_size() as u64;
            self.emit(DeviceTransferEvent::value(
                "restore_network_speed",
                transfer_speed(bytes_received, started),
            ));
            match packet {
                Packet::Command(command) if command.action == "start" => {
                    total = command.total.unwrap_or_default();
                    progress_count = 0;
                    self.emit(DeviceTransferEvent::simple("restore_start"));
                    self.emit(DeviceTransferEvent::value("restore_progress", 0.0));
                }
                Packet::Command(command) if command.action == "finish" => {
                    self.emit(DeviceTransferEvent::value("restore_progress", 100.0));
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
                    self.conversation_changes
                        .send_modify(|revision| *revision = revision.wrapping_add(1));
                    self.emit(DeviceTransferEvent::simple("restore_succeed"));
                    return Ok(());
                }
                Packet::Command(command)
                    if command.action == "close" || command.action == "cancel" =>
                {
                    bail!("sender closed transfer")
                }
                Packet::Command(_) => {}
                Packet::Json { kind, data, .. } => {
                    if let Err(error) = self.insert_record(&kind, &data).await {
                        log::error!(
                            "device transfer failed to insert {kind} record: {error:?}; data={data:?}"
                        );
                    }
                    progress_count += 1;
                    let progress = if total == 0 {
                        0.0
                    } else {
                        (progress_count as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                    };
                    self.emit(DeviceTransferEvent::value("restore_progress", progress));
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
                    message_id, bytes, ..
                } => {
                    if let Err(error) = self.insert_attachment(&message_id, &bytes).await {
                        log::error!(
                            "device transfer failed to insert attachment {message_id}: {error:?}"
                        );
                    }
                }
            }
        }
    }
}

fn transfer_speed(bytes: u64, started: Instant) -> f64 {
    bytes as f64 / started.elapsed().as_secs_f64().max(0.001)
}

fn first_ipv4_address() -> Result<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_loopback() => Ok(ip.to_string()),
        _ => bail!("no local IPv4 address"),
    }
}

fn random_u16() -> Result<u16> {
    let mut bytes = [0u8; 2];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| anyhow!("secure random failed"))?;
    Ok(u16::from_be_bytes(bytes))
}

fn generate_transfer_key() -> Result<[u8; KEY_SIZE]> {
    let mut input = [0u8; 32];
    SystemRandom::new()
        .fill(&mut input)
        .map_err(|_| anyhow!("secure random failed"))?;
    hkdf_sha256(&input, b"Mixin Device Transfer")
}

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

enum Packet {
    Command(TransferCommand),
    Json {
        kind: String,
        data: Map<String, Value>,
        size: usize,
    },
    File {
        message_id: String,
        bytes: Vec<u8>,
        size: usize,
    },
}

impl Packet {
    fn wire_size(&self) -> usize {
        match self {
            Packet::Command(command) => serde_json::to_vec(command).map_or(0, |data| data.len()),
            Packet::Json { size, .. } | Packet::File { size, .. } => *size,
        }
    }
}

async fn write_command<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    command: &TransferCommand,
) -> Result<usize> {
    write_encrypted_packet(writer, key, PACKET_COMMAND, &serde_json::to_vec(command)?).await
}

async fn write_json_record<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    kind: &str,
    data: &Map<String, Value>,
) -> Result<usize> {
    let bytes = serde_json::to_vec(&serde_json::json!({"data": data, "type": kind}))?;
    if bytes.len() > JSON_PACKET_LIMIT {
        bail!("device transfer JSON packet exceeds 500KB");
    }
    write_encrypted_packet(writer, key, PACKET_JSON, &bytes).await
}

async fn write_encrypted_packet<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    packet_type: u8,
    data: &[u8],
) -> Result<usize> {
    let mut iv = [0u8; IV_SIZE];
    SystemRandom::new()
        .fill(&mut iv)
        .map_err(|_| anyhow!("secure random failed"))?;
    let encrypted = encrypt_cbc(&key[..32], &iv, data)?;
    let mut body = Vec::with_capacity(IV_SIZE + encrypted.len());
    body.extend_from_slice(&iv);
    body.extend_from_slice(&encrypted);
    write_packet_body(writer, key, packet_type, &body).await
}

async fn write_file_packet<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    message_id: &str,
    path: &Path,
) -> Result<usize> {
    let file = tokio::fs::read(path)
        .await
        .with_context(|| format!("read transfer attachment {}", path.display()))?;
    if file.is_empty() {
        return Ok(0);
    }
    let mut iv = [0u8; IV_SIZE];
    SystemRandom::new()
        .fill(&mut iv)
        .map_err(|_| anyhow!("secure random failed"))?;
    let encrypted = encrypt_cbc(&key[..32], &iv, &file)?;
    let mut body = Vec::with_capacity(16 + IV_SIZE + encrypted.len());
    body.extend_from_slice(Uuid::parse_str(message_id)?.as_bytes());
    body.extend_from_slice(&iv);
    body.extend_from_slice(&encrypted);
    write_packet_body(writer, key, PACKET_FILE, &body).await
}

async fn write_packet_body<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    packet_type: u8,
    body: &[u8],
) -> Result<usize> {
    let body_length = i32::try_from(body.len()).context("transfer packet too large")?;
    let mut hmac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[32..])
        .map_err(|_| anyhow!("invalid transfer HMAC key"))?;
    hmac.update(body);
    let digest = hmac.finalize().into_bytes();
    writer.write_u8(packet_type).await?;
    writer.write_i32(body_length).await?;
    writer.write_all(body).await?;
    writer.write_all(&digest).await?;
    writer.flush().await?;
    Ok(5 + body.len() + HMAC_SIZE)
}

async fn read_packet<R: AsyncRead + Unpin>(reader: &mut R, key: &[u8; KEY_SIZE]) -> Result<Packet> {
    let packet_type = reader.read_u8().await?;
    let body_length = reader.read_i32().await?;
    if body_length <= 0 {
        bail!("invalid transfer packet length");
    }
    let body_length = body_length as usize;
    let mut body = vec![0u8; body_length];
    reader.read_exact(&mut body).await?;
    let mut expected = [0u8; HMAC_SIZE];
    reader.read_exact(&mut expected).await?;
    let mut hmac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[32..])
        .map_err(|_| anyhow!("invalid transfer HMAC key"))?;
    hmac.update(&body);
    hmac.verify_slice(&expected)
        .map_err(|_| anyhow!("transfer packet HMAC mismatch"))?;
    match packet_type {
        PACKET_COMMAND | PACKET_JSON => {
            if body.len() < IV_SIZE {
                bail!("encrypted transfer packet is truncated");
            }
            let decrypted = decrypt_cbc(&key[..32], &body[..IV_SIZE], &body[IV_SIZE..])?;
            if packet_type == PACKET_COMMAND {
                Ok(Packet::Command(serde_json::from_slice(&decrypted)?))
            } else {
                let wrapper: Value = serde_json::from_slice(&decrypted)?;
                let kind = wrapper
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("transfer JSON packet has no type"))?
                    .to_string();
                let data = wrapper
                    .get("data")
                    .and_then(Value::as_object)
                    .cloned()
                    .ok_or_else(|| anyhow!("transfer JSON packet has no data"))?;
                Ok(Packet::Json {
                    kind,
                    data,
                    size: 5 + body_length + HMAC_SIZE,
                })
            }
        }
        PACKET_FILE => {
            if body.len() < 16 + IV_SIZE {
                bail!("attachment transfer packet is truncated");
            }
            let message_id = Uuid::from_slice(&body[..16])?.to_string();
            let bytes = decrypt_cbc(&key[..32], &body[16..32], &body[32..])?;
            Ok(Packet::File {
                message_id,
                bytes,
                size: 5 + body_length + HMAC_SIZE,
            })
        }
        _ => bail!("unknown transfer packet type: {packet_type}"),
    }
}

fn encrypt_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256::new_from_slice(key).map_err(|_| anyhow!("invalid AES key"))?;
    let padding = 16 - data.len() % 16;
    let mut padded = Vec::with_capacity(data.len() + padding);
    padded.extend_from_slice(data);
    padded.resize(data.len() + padding, padding as u8);
    let mut previous: [u8; 16] = iv.try_into().map_err(|_| anyhow!("invalid AES IV"))?;
    for chunk in padded.chunks_exact_mut(16) {
        for (byte, previous) in chunk.iter_mut().zip(previous) {
            *byte ^= previous;
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(chunk);
        let mut block = Block::<Aes256>::from(bytes);
        cipher.encrypt_block(&mut block);
        chunk.copy_from_slice(&block);
        previous.copy_from_slice(chunk);
    }
    Ok(padded)
}

fn decrypt_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() || data.len() % 16 != 0 {
        bail!("invalid AES ciphertext length");
    }
    let cipher = Aes256::new_from_slice(key).map_err(|_| anyhow!("invalid AES key"))?;
    let mut output = data.to_vec();
    let mut previous: [u8; 16] = iv.try_into().map_err(|_| anyhow!("invalid AES IV"))?;
    for chunk in output.chunks_exact_mut(16) {
        let encrypted: [u8; 16] = chunk.try_into().expect("AES block length");
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(chunk);
        let mut block = Block::<Aes256>::from(bytes);
        cipher.decrypt_block(&mut block);
        for (byte, previous) in block.iter_mut().zip(previous) {
            *byte ^= previous;
        }
        chunk.copy_from_slice(&block);
        previous = encrypted;
    }
    let padding = *output.last().unwrap() as usize;
    if padding == 0
        || padding > 16
        || !output[output.len() - padding..]
            .iter()
            .all(|byte| *byte as usize == padding)
    {
        bail!("invalid AES padding");
    }
    output.truncate(output.len() - padding);
    Ok(output)
}

struct RecordSpec {
    kind: &'static str,
    table: &'static str,
    columns: &'static [&'static str],
    boolean_columns: &'static [&'static str],
}

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

fn spec(kind: &'static str, table: &'static str, columns: &'static [&'static str]) -> RecordSpec {
    RecordSpec {
        kind,
        table,
        columns,
        boolean_columns: &[],
    }
}

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

impl DeviceTransferService {
    fn pool(&self) -> &sqlx::SqlitePool {
        &self.database.user_dao.0
    }

    async fn count_records(&self, spec: &RecordSpec) -> Result<u64> {
        let count = sqlx::query_scalar::<_, i64>(AssertSqlSafe(format!(
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
        let rows = sqlx::query_scalar::<_, String>(AssertSqlSafe(sql))
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
        let mut builder = QueryBuilder::<Sqlite>::new(format!("{verb} INTO {} (", spec.table));
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

    async fn attachment_for_record(
        &self,
        kind: &str,
        record: &Map<String, Value>,
    ) -> Result<Option<PathBuf>> {
        let message = message_from_record(record)?;
        if !message.category.is_attachment() {
            return Ok(None);
        }
        let media_status = record
            .get("media_status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let transferable = if kind == "transcript_message" {
            media_status == "DONE"
        } else {
            matches!(media_status, "DONE" | "READ")
        };
        if !transferable {
            return Ok(None);
        }
        let path = if kind == "transcript_message" {
            transcript_attachment_path(&self.account_data_dir, &message)?
        } else {
            attachment_path(&self.account_data_dir, &message)?
        };
        Ok(tokio::fs::metadata(&path)
            .await
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.len() > 0)
            .map(|_| path))
    }

    async fn insert_attachment(&self, message_id: &str, bytes: &[u8]) -> Result<()> {
        if let Some(record) = self
            .record_by_message_id("transcript_messages", message_id)
            .await?
        {
            let message = message_from_record(&record)?;
            let path = transcript_attachment_path(&self.account_data_dir, &message)?;
            write_attachment_if_absent(&path, bytes).await?;
        }
        if let Some(record) = self.record_by_message_id("messages", message_id).await? {
            let message = message_from_record(&record)?;
            let path = attachment_path(&self.account_data_dir, &message)?;
            write_attachment_if_absent(&path, bytes).await?;
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
        let row = sqlx::query(sql)
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
    if kind == "message" || kind == "transcript_message" {
        if data.get("media_status").and_then(Value::as_str) == Some("PENDING") {
            data.insert(
                "media_status".to_string(),
                Value::String("CANCELED".to_string()),
            );
        }
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
    if kind == "message" || kind == "transcript_message" {
        if data.get("media_status").and_then(Value::as_str) == Some("PENDING") {
            data.insert(
                "media_status".to_string(),
                Value::String("CANCELED".to_string()),
            );
        }
    }
}

fn normalize_millis_date(data: &mut Map<String, Value>, key: &str) {
    let Some(milliseconds) = data.get(key).and_then(Value::as_i64) else {
        return;
    };
    let Some(date) = chrono::DateTime::from_timestamp_millis(milliseconds) else {
        return;
    };
    data.insert(key.to_string(), Value::String(date.to_rfc3339()));
}

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

async fn write_attachment_if_absent(path: &Path, bytes: &[u8]) -> Result<()> {
    if tokio::fs::metadata(path).await.is_ok() {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("attachment path has no parent"))?;
    tokio::fs::create_dir_all(parent).await?;
    let temp = path.with_extension(format!("transfer-{}", Uuid::new_v4()));
    tokio::fs::write(&temp, bytes).await?;
    if let Err(error) = tokio::fs::rename(&temp, path).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
