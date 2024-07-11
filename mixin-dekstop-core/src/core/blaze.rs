use std::error::Error;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use futures::{future, pin_mut, SinkExt, StreamExt};
use futures::stream::SplitSink;
use futures_channel::mpsc::UnboundedSender;
use reqwest::header::HeaderValue;
use reqwest::Method;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite, WebSocketStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::core::BlazeMessage;
use crate::db::MixinDatabase;
use crate::sdk::{Client, Credential};

const WS_HOST: &str = "wss://blaze.mixin.one";

type StreamWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub struct Blaze {
    database: Arc<MixinDatabase>,
    client: Client,
    credential: Credential,
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
    pub fn new(database: Arc<MixinDatabase>, client: Client, credential: Credential) -> Self {
        Blaze { database, client, credential, sender: None }
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
                let result = self.on_message(message).await;
                match result {
                    Err(err) => println!("err: {}", err),
                    Ok(_) => println!("ok")
                }
            })
        };
        sender.send_blaze_message(BlazeMessage::new_list_pending_blaze(None)).await?;
        pin_mut!(send, receive);
        future::select(send, receive).await;
        Ok(())
    }
}

impl Blaze {
    async fn on_message(&self, message: Result<Message, tungstenite::Error>) -> Result<(), Box<dyn Error>> {
        let message = message?;
        let data = message.into_data();

        let mut decoder = GzDecoder::new(Cursor::new(&data));
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        let message: BlazeMessage = serde_json::from_slice(&decompressed_data)?;
        println!("message: {:?}", message);
        Ok(())
    }
}

