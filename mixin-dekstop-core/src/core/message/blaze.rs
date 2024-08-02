use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::{future, pin_mut, SinkExt, StreamExt};
use futures_channel::mpsc::UnboundedSender;
use log::{error, info, warn};
use reqwest::header::HeaderValue;
use reqwest::Method;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use sdk::blaze_message::{
    BlazeMessage, BlazeMessageData, ACKNOWLEDGE_MESSAGE_RECEIPT, CREATE_CALL, CREATE_KRAKEN,
    CREATE_MESSAGE,
};
use sdk::err::error_code;
use sdk::{Credential, ERROR_ACTION};

use crate::core::message::completer::Completer;
use crate::db::mixin::flood_message::FloodMessage;
use crate::db::mixin::MixinDatabase;

const WS_HOST: &str = "wss://blaze.mixin.one";

pub struct Blaze {
    database: Arc<MixinDatabase>,
    credential: Credential,
    user_id: String,
    connection: Arc<Mutex<BlazeConnection>>,
    transactions: Arc<Mutex<HashMap<String, Completer<BlazeMessage>>>>,
}

struct BlazeConnection {
    sink: Option<UnboundedSender<Message>>,
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
        self.send(Message::Binary(compressed_data)).await?;
        Ok(())
    }
}

impl Blaze {
    pub fn new(database: Arc<MixinDatabase>, credential: Credential, user_id: String) -> Self {
        Blaze {
            database,
            credential,
            connection: Arc::new(Mutex::new(BlazeConnection { sink: None })),
            user_id,
            transactions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        {
            let connection = self.connection.lock().unwrap();
            if connection.sink.is_some() {
                warn!("already connected");
                return Ok(());
            }
        }
        let token = self
            .credential
            .sign_authentication_token(&Method::GET, &"/".to_string(), [])
            .map_err(|e| anyhow!("can not sign request: {}", e))?;

        let (mut sender, receiver) = futures_channel::mpsc::unbounded();

        {
            let mut connection = self.connection.lock().unwrap();
            connection.sink = Some(sender.clone());
        }

        let mut request = WS_HOST.into_client_request()?;
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::try_from("Mixin-Blaze-1")?,
        );
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::try_from(format!("Bearer {}", token))?,
        );

        let (ws_stream, _) = connect_async(request).await?;
        let (sink, stream) = ws_stream.split();

        let send = receiver.map(Ok).forward(sink);
        let receive = {
            stream.for_each(|message| async {
                let message = match message {
                    Ok(m) => m,
                    Err(err) => {
                        error!("socket error : {}", err);
                        return;
                    }
                };
                if message.is_ping() || message.is_pong() {
                    return;
                }
                let result = self.on_socket_message(message).await;
                if let Err(err) = result {
                    error!("failed to handle socket message: {:?}", err)
                }
            })
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
        pin_mut!(send, receive);
        future::select(send, receive).await;
        Ok(())
    }
}

impl Blaze {
    async fn on_socket_message(&self, message: Message) -> Result<()> {
        let data = message.into_data();

        let mut decoder = GzDecoder::new(Cursor::new(&data));
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        let message: BlazeMessage = serde_json::from_slice(&decompressed_data)?;

        if message.action == ERROR_ACTION
            && message
                .error
                .as_ref()
                .is_some_and(|e| e.code == error_code::AUTHENTICATION)
        {
            // TODO: reconnect
            return Ok(());
        }

        {
            let mut transactions = self.transactions.lock().unwrap();
            if let Some(transaction) = transactions.get(&message.id) {
                transaction.complete(Ok(message.clone()));
                transactions.remove(&message.id);
            }
        }

        if message.data.is_some()
            && (message.action == ACKNOWLEDGE_MESSAGE_RECEIPT
                || message.action == CREATE_MESSAGE
                || message.action == CREATE_CALL
                || message.action == CREATE_KRAKEN)
        {
            if let Err(err) = self.handle_receive_message(message).await {
                error!("failed to handle_receive_message, error: {:?} ", err);
            }
        }
        Ok(())
    }

    async fn handle_receive_message(&self, message: BlazeMessage) -> Result<()> {
        let data = message.data.ok_or(anyhow!("blaze message no data"))?;
        let data: BlazeMessageData = serde_json::from_value(data)?;
        info!("handle receive message: {}", message.action);
        if message.action == ACKNOWLEDGE_MESSAGE_RECEIPT {
            warn!("todo: handle action ACKNOWLEDGE_MESSAGE_RECEIPT");
        } else if message.action == CREATE_MESSAGE {
            if data.user_id == self.user_id
                && (data.category.is_empty() || data.conversation_id.is_empty())
            {
                warn!("todo: handle mark status");
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
            warn!("TODO: notify read");
        } else {
            warn!("TODO: mark deleverd");
        }
        Ok(())
    }

    pub fn try_get_sender(&self) -> Result<UnboundedSender<Message>> {
        let connection = self.connection.try_lock().map_err(|e| anyhow!("{e}"))?;
        connection.sink.clone().ok_or(anyhow!("not connected"))
    }

    pub async fn send_message(&self, message: BlazeMessage) -> Result<BlazeMessage> {
        let mut sender = self.try_get_sender()?;
        let completer = Completer::default();
        {
            let mut transactions = self.transactions.lock().unwrap();
            transactions.insert(message.id.clone(), completer.clone());
        }
        sender.send_blaze_message(message).await?;
        completer.await
    }
}
