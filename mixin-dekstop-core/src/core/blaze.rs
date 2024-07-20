use std::error::Error;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use futures::{future, pin_mut, SinkExt, StreamExt};
use futures::future::err;
use futures::stream::SplitSink;
use futures_channel::mpsc::UnboundedSender;
use log::{debug, error, info, warn};
use reqwest::header::HeaderValue;
use reqwest::Method;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite, WebSocketStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use crate::db::mixin::flood_message::FloodMessage;
use crate::db::mixin::MixinDatabase;
use crate::sdk::{Client, Credential};
use crate::sdk::blaze_message::{ACKNOWLEDGE_MESSAGE_RECEIPT, BlazeMessage, BlazeMessageData, CREATE_CALL, CREATE_KRAKEN, CREATE_MESSAGE, LIST_PENDING_MESSAGE};

const WS_HOST: &str = "wss://blaze.mixin.one";

type StreamWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub struct Blaze {
    database: Arc<MixinDatabase>,
    client: Client,
    credential: Credential,
    user_id: String,
    sender: Option<UnboundedSender<Message>>,
}

trait SendBlazeMessage {
    async fn send_blaze_message(&mut self, message: BlazeMessage) -> Result<(), Box<dyn Error>>;
}

impl SendBlazeMessage for UnboundedSender<Message> {
    async fn send_blaze_message(&mut self, message: BlazeMessage) -> Result<(), Box<dyn Error>> {
        let bytes = serde_json::to_vec(&message)?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&bytes)?;
        let compressed_data = encoder.finish()?;
        self.send(Message::Binary(compressed_data)).await?;
        Ok(())
    }
}

impl Blaze {
    pub fn new(database: Arc<MixinDatabase>, client: Client, credential: Credential, user_id: String) -> Self {
        Blaze { database, client, credential, sender: None, user_id }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let token = self.credential.sign_authentication_token(&Method::GET, &"/".to_string(), [])?;

        let (mut sender, receiver) = futures_channel::mpsc::unbounded();
        self.sender = Some(sender.clone());

        let mut request = WS_HOST.into_client_request()?;
        request.headers_mut().insert("Sec-WebSocket-Protocol", HeaderValue::try_from("Mixin-Blaze-1")?);
        request.headers_mut().insert("Authorization", HeaderValue::try_from(format!("Bearer {}", token))?);

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
                let result = self.on_socket_message(message.clone()).await;
                match result {
                    Err(err) => error!("failed to handle socket message: {:?}", err),
                    _ => {}
                }
            })
        };
        let offset = self.database.latest_flood_message_created_at().await?;
        sender.send_blaze_message(BlazeMessage::new_list_pending_blaze(offset.map(|e| e.and_utc().to_rfc3339()))).await?;
        pin_mut!(send, receive);
        future::select(send, receive).await;
        Ok(())
    }
}

impl Blaze {
    async fn on_socket_message(&self, message: Message) -> Result<(), Box<dyn Error>> {
        let data = message.into_data();

        let mut decoder = GzDecoder::new(Cursor::new(&data));
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        let message: BlazeMessage = serde_json::from_slice(&decompressed_data)?;
        self.handle_receive_message(&message).await.or_else(|e| {
            error!("failed to handle_receive_message: {:?}, {:?} ", e, message);
            Ok(())
        })
    }

    async fn handle_receive_message(&self, message: &BlazeMessage) -> Result<(), Box<dyn Error>> {
        let data = message.data.clone().ok_or("blaze message no data")?;
        let data: BlazeMessageData = serde_json::from_value(data)?;
        debug!("handle receive message: {}", message.action);
        if message.action == ACKNOWLEDGE_MESSAGE_RECEIPT {
            warn!("todo: handle action ACKNOWLEDGE_MESSAGE_RECEIPT");
        } else if message.action == CREATE_MESSAGE {
            if data.user_id == self.user_id
                && (data.category.is_none() || data.conversation_id.is_empty()) {
                warn!("todo: handle mark status");
            } else {
                let data_str = serde_json::to_string(&data)?;
                let flood_message = FloodMessage {
                    message_id: data.message_id,
                    data: data_str,
                    created_at: data.created_at.naive_utc(),
                };
                self.database.insert_flood_message(flood_message).await?;
            }
        } else if message.action == CREATE_CALL || message.action == CREATE_KRAKEN {
            warn!("TODO: notify read");
        } else {
            warn!("TODO: mark deleverd");
        }
        Ok(())
    }
}

