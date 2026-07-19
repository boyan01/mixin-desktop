use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::{SinkExt, StreamExt};
use futures_channel::mpsc::UnboundedSender;
use log::{error, info, warn};
use reqwest::header::HeaderValue;
use reqwest::Method;
use tokio::sync::{watch, Notify};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};

use sdk::blaze_message::{
    BlazeMessage, BlazeMessageData, ACKNOWLEDGE_MESSAGE_RECEIPT, CREATE_CALL, CREATE_KRAKEN,
    CREATE_MESSAGE,
};
use sdk::err::error_code;
use sdk::{
    BlazeAckMessage, Client, Credential, MessageStatus, ACKNOWLEDGE_MESSAGE_RECEIPTS, ERROR_ACTION,
};

use crate::core::conversation_change::ConversationChangeNotifier;
use crate::core::message::completer::Completer;
use crate::db::mixin::flood_message::FloodMessage;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::Message as StoredMessage;
use crate::db::mixin::MixinDatabase;

const WS_HOSTS: [&str; 2] = [
    "wss://blaze.mixin.one:443",
    "wss://mixin-blaze.zeromesh.net:443",
];
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const STATUS_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const PING_INTERVAL: Duration = Duration::from_secs(10);
const PONG_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
#[error("blaze authentication failed")]
pub struct BlazeAuthenticationError;

pub struct Blaze {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
    credential: Credential,
    user_id: String,
    connection: Arc<Mutex<BlazeConnection>>,
    connection_status: watch::Sender<bool>,
    transactions: Arc<Mutex<HashMap<String, Completer<BlazeMessage>>>>,
    connect_running: Arc<AtomicBool>,
    reconnect: Notify,
    pending_message_statuses: PendingMessageStatusStore,
}

#[derive(Clone, Default)]
pub struct PendingMessageStatusStore {
    statuses: Arc<tokio::sync::Mutex<HashMap<String, MessageStatus>>>,
    changes: Option<ConversationChangeNotifier>,
}

impl PendingMessageStatusStore {
    fn new(changes: Option<ConversationChangeNotifier>) -> Self {
        Self {
            statuses: Default::default(),
            changes,
        }
    }

    async fn apply(
        &self,
        database: &MixinDatabase,
        message_id: &str,
        status: MessageStatus,
    ) -> Result<()> {
        let mut pending = self.statuses.lock().await;
        if database
            .message_dao
            .advance_message_status(message_id, status)
            .await?
        {
            if let Some(changes) = &self.changes {
                if let Some(conversation_id) = database
                    .message_dao
                    .conversation_id_by_message_id(message_id)
                    .await?
                {
                    changes.notify(conversation_id);
                } else {
                    changes.notify_all();
                }
            }
            return Ok(());
        }
        if database
            .message_dao
            .is_message_exits(&message_id.to_string())
            .await?
            || database.message_history_dao.exists(message_id).await?
        {
            return Ok(());
        }

        pending
            .entry(message_id.to_string())
            .and_modify(|current| {
                if status > *current {
                    *current = status;
                }
            })
            .or_insert(status);
        Ok(())
    }

    pub async fn insert_message(
        &self,
        database: &MixinDatabase,
        message: &StoredMessage,
    ) -> Result<()> {
        let mut pending = self.statuses.lock().await;
        let mut message = message.clone();
        if let Some(status) = pending.get(&message.message_id).copied() {
            if !matches!(
                message.status,
                MessageStatus::Failed | MessageStatus::Unknown
            ) && status > message.status
            {
                message.status = status;
            }
        }
        database.message_dao.insert_message(&message).await?;
        pending.remove(&message.message_id);
        Ok(())
    }
}

struct BlazeConnection {
    sink: Option<UnboundedSender<Message>>,
}

enum SocketSessionEnd {
    Disconnected,
    AuthenticationFailed,
}

enum SocketMessageResult {
    Continue,
    AuthenticationFailed,
}

struct ConnectLoopGuard(Arc<AtomicBool>);

impl Drop for ConnectLoopGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct SocketSessionGuard {
    connection: Arc<Mutex<BlazeConnection>>,
    connection_status: watch::Sender<bool>,
    transactions: Arc<Mutex<HashMap<String, Completer<BlazeMessage>>>>,
}

impl Drop for SocketSessionGuard {
    fn drop(&mut self) {
        self.connection_status.send_replace(false);
        self.connection.lock().unwrap().sink = None;
        fail_transactions(&self.transactions, "blaze disconnected");
    }
}

fn fail_transactions(transactions: &Mutex<HashMap<String, Completer<BlazeMessage>>>, reason: &str) {
    let pending = transactions.lock().unwrap().drain().collect::<Vec<_>>();
    for (_, transaction) in pending {
        transaction.complete(Err(anyhow!(reason.to_string())));
    }
}

fn is_authentication_handshake_error(error: &WebSocketError) -> bool {
    matches!(
        error,
        WebSocketError::Http(response) if matches!(response.status().as_u16(), 401 | 403)
    )
}

fn is_api_authentication_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<sdk::ApiError>().is_some_and(|error| {
        matches!(
            error,
            sdk::ApiError::Server(sdk::Error {
                code: error_code::AUTHENTICATION,
                ..
            })
        )
    })
}

async fn await_transaction(
    transactions: &Mutex<HashMap<String, Completer<BlazeMessage>>>,
    message_id: &str,
    completer: Completer<BlazeMessage>,
    timeout: Duration,
) -> Result<BlazeMessage> {
    match tokio::time::timeout(timeout, completer).await {
        Ok(result) => result,
        Err(_) => {
            transactions.lock().unwrap().remove(message_id);
            Err(anyhow!("blaze request {message_id} timed out"))
        }
    }
}

trait SendBlazeMessage {
    async fn send_blaze_message(&mut self, message: BlazeMessage) -> Result<()>;
}

impl SendBlazeMessage for UnboundedSender<Message> {
    async fn send_blaze_message(&mut self, message: BlazeMessage) -> Result<()> {
        let bytes = serde_json::to_vec(&message)?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&bytes)?;
        let compressed_data = encoder.finish()?;
        self.send(Message::Binary(compressed_data.into())).await?;
        Ok(())
    }
}

impl Blaze {
    pub fn new(
        database: Arc<MixinDatabase>,
        client: Arc<Client>,
        credential: Credential,
        user_id: String,
        changes: Option<ConversationChangeNotifier>,
    ) -> Self {
        let (connection_status, _) = watch::channel(false);
        Blaze {
            database,
            client,
            credential,
            connection: Arc::new(Mutex::new(BlazeConnection { sink: None })),
            connection_status,
            user_id,
            transactions: Arc::new(Mutex::new(HashMap::new())),
            connect_running: Arc::new(AtomicBool::new(false)),
            reconnect: Notify::new(),
            pending_message_statuses: PendingMessageStatusStore::new(changes),
        }
    }

    pub fn pending_message_statuses(&self) -> PendingMessageStatusStore {
        self.pending_message_statuses.clone()
    }

    pub fn subscribe_connection_status(&self) -> watch::Receiver<bool> {
        self.connection_status.subscribe()
    }

    pub fn retry_connection(&self) {
        self.reconnect.notify_one();
    }

    pub async fn connect(&self) -> Result<()> {
        if self
            .connect_running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(anyhow!("blaze connection loop is already running"));
        }
        let _connect_guard = ConnectLoopGuard(self.connect_running.clone());

        let mut host_index = 0;
        loop {
            let host = WS_HOSTS[host_index];
            match self.connect_once(host).await {
                Ok(SocketSessionEnd::AuthenticationFailed) => {
                    return Err(BlazeAuthenticationError.into());
                }
                Ok(SocketSessionEnd::Disconnected) => {
                    warn!("blaze disconnected from {host}");
                }
                Err(err) => {
                    warn!("blaze connection to {host} failed: {err:?}");
                }
            }

            host_index = (host_index + 1) % WS_HOSTS.len();
            tokio::select! {
                _ = tokio::time::sleep(RECONNECT_DELAY) => {}
                _ = self.reconnect.notified() => {}
            }
        }
    }

    async fn connect_once(&self, host: &str) -> Result<SocketSessionEnd> {
        let token = self
            .credential
            .sign_authentication_token(&Method::GET, &"/".to_string(), [])
            .map_err(|e| anyhow!("can not sign request: {}", e))?;

        let mut request = host.into_client_request()?;
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::try_from("Mixin-Blaze-1")?,
        );
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::try_from(format!("Bearer {}", token))?,
        );

        let connection = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(request))
            .await
            .map_err(|_| anyhow!("blaze connection timed out"))?;
        let (ws_stream, _) = match connection {
            Ok(connection) => connection,
            Err(err) if is_authentication_handshake_error(&err) => {
                return Ok(SocketSessionEnd::AuthenticationFailed);
            }
            Err(err) => return Err(err.into()),
        };
        let (mut socket_sink, mut socket_stream) = ws_stream.split();
        let (mut sender, mut receiver) = futures_channel::mpsc::unbounded();

        self.connection.lock().unwrap().sink = Some(sender.clone());
        self.connection_status.send_replace(true);
        let _session_guard = SocketSessionGuard {
            connection: self.connection.clone(),
            connection_status: self.connection_status.clone(),
            transactions: self.transactions.clone(),
        };

        let offset = self
            .database
            .flood_message_dao
            .latest_flood_message_created_at()
            .await?;
        info!("latest flood message created at offset: {:?}", offset);
        sender
            .send_blaze_message(BlazeMessage::new_list_pending_blaze(
                offset.map(|e| e.and_utc().to_rfc3339()),
            ))
            .await?;
        let mut heartbeat = tokio::time::interval(PING_INTERVAL);
        heartbeat.tick().await;
        let mut last_pong = tokio::time::Instant::now();
        let mut status_refresh = Box::pin(self.refresh_message_status_offset());
        let mut status_refresh_complete = false;

        loop {
            tokio::select! {
                result = status_refresh.as_mut(), if !status_refresh_complete => {
                    status_refresh_complete = true;
                    if let Err(err) = result {
                        if is_api_authentication_error(&err) {
                            return Ok(SocketSessionEnd::AuthenticationFailed);
                        }
                        warn!("failed to refresh message status offset: {err}");
                    }
                }
                _ = heartbeat.tick() => {
                    if last_pong.elapsed() >= PONG_TIMEOUT {
                        return Ok(SocketSessionEnd::Disconnected);
                    }
                    socket_sink.send(Message::Ping(Vec::new().into())).await?;
                }
                outgoing = receiver.next() => {
                    let Some(outgoing) = outgoing else {
                        return Ok(SocketSessionEnd::Disconnected);
                    };
                    socket_sink.send(outgoing).await?;
                }
                incoming = socket_stream.next() => {
                    let Some(incoming) = incoming else {
                        return Ok(SocketSessionEnd::Disconnected);
                    };
                    let incoming = incoming?;
                    match incoming {
                        Message::Ping(data) => socket_sink.send(Message::Pong(data)).await?,
                        Message::Pong(_) => last_pong = tokio::time::Instant::now(),
                        Message::Close(_) => return Ok(SocketSessionEnd::Disconnected),
                        message => {
                            match self.on_socket_message(message).await? {
                                SocketMessageResult::Continue => {}
                                SocketMessageResult::AuthenticationFailed => {
                                    return Ok(SocketSessionEnd::AuthenticationFailed);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Blaze {
    async fn on_socket_message(&self, message: Message) -> Result<SocketMessageResult> {
        let data = message.into_data();

        let mut decoder = GzDecoder::new(Cursor::new(&data));
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        let message: BlazeMessage = serde_json::from_slice(&decompressed_data)?;

        let authentication_failed = message.action == ERROR_ACTION
            && message
                .error
                .as_ref()
                .is_some_and(|e| e.code == error_code::AUTHENTICATION);

        {
            let mut transactions = self.transactions.lock().unwrap();
            if let Some(transaction) = transactions.remove(&message.id) {
                transaction.complete(Ok(message.clone()));
            }
        }

        if authentication_failed {
            return Ok(SocketMessageResult::AuthenticationFailed);
        }

        if message.data.is_some()
            && (message.action == ACKNOWLEDGE_MESSAGE_RECEIPT
                || message.action == CREATE_MESSAGE
                || message.action == CREATE_CALL
                || message.action == CREATE_KRAKEN)
        {
            if let Err(err) = self.handle_receive_message(message).await {
                if is_api_authentication_error(&err) {
                    return Ok(SocketMessageResult::AuthenticationFailed);
                }
                error!("failed to handle_receive_message, error: {:?} ", err);
            }
        }
        Ok(SocketMessageResult::Continue)
    }

    async fn handle_receive_message(&self, message: BlazeMessage) -> Result<()> {
        let data = message.data.ok_or(anyhow!("blaze message no data"))?;
        let data: BlazeMessageData = serde_json::from_value(data)?;
        info!("handle receive message: {}", message.action);
        if message.action == ACKNOWLEDGE_MESSAGE_RECEIPT {
            self.apply_message_status(&data).await?;
        } else if message.action == CREATE_MESSAGE {
            if data.user_id == self.user_id
                && (data.category.is_empty() || data.conversation_id.is_empty())
            {
                self.pending_message_statuses
                    .apply(&self.database, &data.message_id, data.status)
                    .await?;
            } else {
                let data_str = serde_json::to_string(&data)?;
                let flood_message = FloodMessage {
                    message_id: data.message_id,
                    data: data_str,
                    created_at: data.created_at.naive_utc(),
                };
                self.database
                    .flood_message_dao
                    .insert_flood_message(flood_message)
                    .await?;
            }
        } else if message.action == CREATE_CALL || message.action == CREATE_KRAKEN {
            self.acknowledge_message(&data.message_id, MessageStatus::Read)
                .await?;
        } else {
            self.acknowledge_message(&data.message_id, MessageStatus::Delivered)
                .await?;
        }
        Ok(())
    }

    async fn apply_message_status(&self, data: &BlazeMessageData) -> Result<()> {
        self.pending_message_statuses
            .apply(&self.database, &data.message_id, data.status)
            .await?;
        self.database
            .offset_dao
            .save_message_status_offset(data.updated_at)
            .await?;
        Ok(())
    }

    async fn refresh_message_status_offset(&self) -> Result<()> {
        let timestamp = self
            .database
            .offset_dao
            .message_status_offset()
            .await?
            .unwrap_or_else(chrono::Utc::now);
        let mut offset = timestamp
            .timestamp_nanos_opt()
            .ok_or_else(|| anyhow!("message status offset is outside the nanosecond range"))?;

        loop {
            let messages = tokio::time::timeout(
                STATUS_REQUEST_TIMEOUT,
                self.client.message_api.message_status_offset(offset),
            )
            .await
            .map_err(|_| anyhow!("message status offset request timed out"))??;
            if messages.is_empty() {
                return Ok(());
            }

            let last_offset = messages
                .last()
                .and_then(|message| message.updated_at.timestamp_nanos_opt())
                .ok_or_else(|| anyhow!("message status response has an invalid offset"))?;
            for message in &messages {
                self.apply_message_status(message).await?;
            }
            if last_offset <= offset {
                return Ok(());
            }
            offset = last_offset;
        }
    }

    async fn acknowledge_message(&self, message_id: &str, status: MessageStatus) -> Result<()> {
        let status: &str = status.into();
        let job = Job::create_ack_job(ACKNOWLEDGE_MESSAGE_RECEIPTS, message_id, status, None);
        self.database.job_dao.insert_job(&job).await?;

        let acknowledgement = BlazeAckMessage {
            message_id: message_id.to_string(),
            status: status.to_string(),
            expire_at: None,
        };
        let client = self.client.clone();
        let database = self.database.clone();
        let job_id = job.job_id;
        tokio::spawn(async move {
            match client
                .message_api
                .acknowledgements(std::slice::from_ref(&acknowledgement))
                .await
            {
                Ok(()) => {
                    if let Err(err) = database.job_dao.delete_job_by_id(&job_id).await {
                        warn!("failed to remove acknowledged message job {job_id}: {err}");
                    }
                }
                Err(err) => {
                    warn!(
                        "failed to acknowledge message {}, queued for retry: {err}",
                        acknowledgement.message_id
                    );
                }
            }
        });
        Ok(())
    }

    pub fn try_get_sender(&self) -> Result<UnboundedSender<Message>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow!("blaze connection lock is poisoned"))?;
        connection.sink.clone().ok_or(anyhow!("not connected"))
    }

    async fn get_sender(&self) -> Result<UnboundedSender<Message>> {
        let mut connection_status = self.connection_status.subscribe();
        loop {
            if let Ok(sender) = self.try_get_sender() {
                return Ok(sender);
            }
            connection_status
                .wait_for(|connected| *connected)
                .await
                .map_err(|_| anyhow!("blaze connection stopped"))?;
        }
    }

    pub async fn send_message(&self, message: BlazeMessage) -> Result<BlazeMessage> {
        let mut sender = self.get_sender().await?;
        let completer = Completer::default();
        let message_id = message.id.clone();
        {
            let mut transactions = self.transactions.lock().unwrap();
            transactions.insert(message_id.clone(), completer.clone());
        }
        if let Err(err) = sender.send_blaze_message(message).await {
            self.transactions.lock().unwrap().remove(&message_id);
            return Err(err);
        }

        await_transaction(&self.transactions, &message_id, completer, REQUEST_TIMEOUT).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn disconnect_clears_sender_and_fails_pending_transactions() {
        let (sender, _receiver) = futures_channel::mpsc::unbounded();
        let connection = Arc::new(Mutex::new(BlazeConnection { sink: Some(sender) }));
        let (connection_status, _) = watch::channel(true);
        let transactions = Arc::new(Mutex::new(HashMap::new()));
        let completer = Completer::<BlazeMessage>::default();
        transactions
            .lock()
            .unwrap()
            .insert("request-id".to_string(), completer.clone());

        let guard = SocketSessionGuard {
            connection: connection.clone(),
            connection_status,
            transactions: transactions.clone(),
        };
        drop(guard);

        assert!(connection.lock().unwrap().sink.is_none());
        assert!(transactions.lock().unwrap().is_empty());
        assert_eq!(
            completer.await.unwrap_err().to_string(),
            "blaze disconnected"
        );
    }

    #[tokio::test]
    async fn request_timeout_removes_pending_transaction() {
        let transactions = Mutex::new(HashMap::new());
        let completer = Completer::<BlazeMessage>::default();
        transactions
            .lock()
            .unwrap()
            .insert("request-id".to_string(), completer.clone());

        let result = await_transaction(
            &transactions,
            "request-id",
            completer,
            Duration::from_millis(1),
        )
        .await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "blaze request request-id timed out"
        );
        assert!(transactions.lock().unwrap().is_empty());
    }

    #[test]
    fn unauthorized_handshake_is_an_authentication_error() {
        for status in [401, 403] {
            let response = tokio_tungstenite::tungstenite::http::Response::builder()
                .status(status)
                .body(None)
                .unwrap();
            assert!(is_authentication_handshake_error(&WebSocketError::Http(
                Box::new(response)
            )));
        }

        let api_error = anyhow::Error::new(sdk::ApiError::Server(sdk::Error {
            status: 401,
            code: error_code::AUTHENTICATION,
            description: "unauthorized".to_string(),
        }));
        assert!(is_api_authentication_error(&api_error));
    }

    #[tokio::test]
    async fn status_received_before_insert_is_applied_after_insert() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('conversation-id', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let pending = PendingMessageStatusStore::default();

        pending
            .apply(&database, "message-id", MessageStatus::Delivered)
            .await
            .unwrap();
        pending
            .apply(&database, "message-id", MessageStatus::Read)
            .await
            .unwrap();
        pending
            .insert_message(
                &database,
                &StoredMessage {
                    message_id: "message-id".to_string(),
                    conversation_id: "conversation-id".to_string(),
                    user_id: "user-id".to_string(),
                    category: "PLAIN_TEXT".to_string(),
                    status: MessageStatus::Sent,
                    created_at: Utc::now().naive_utc(),
                    ..StoredMessage::default()
                },
            )
            .await
            .unwrap();

        let message = database
            .message_dao
            .find_message_by_id(&"message-id".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(message.status, MessageStatus::Read);
        assert!(pending.statuses.lock().await.is_empty());

        pending
            .apply(&database, "failed-message", MessageStatus::Read)
            .await
            .unwrap();
        pending
            .insert_message(
                &database,
                &StoredMessage {
                    message_id: "failed-message".to_string(),
                    conversation_id: "conversation-id".to_string(),
                    user_id: "user-id".to_string(),
                    category: "SIGNAL_TEXT".to_string(),
                    status: MessageStatus::Failed,
                    created_at: Utc::now().naive_utc(),
                    ..StoredMessage::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(
            database
                .message_dao
                .find_message_by_id(&"failed-message".to_string())
                .await
                .unwrap()
                .unwrap()
                .status,
            MessageStatus::Failed
        );
    }
}
