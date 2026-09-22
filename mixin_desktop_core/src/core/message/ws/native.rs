//! Native WebSocket and protocol keepalive.
use super::{Handshake, WsConnectError};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
pub(crate) use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue, Error};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
pub(crate) type WsSink = SplitSink<Socket, Message>;
pub(crate) type WsStream = SplitStream<Socket>;
pub(crate) async fn connect(handshake: &Handshake) -> Result<(WsSink, WsStream), WsConnectError> {
    let mut request = handshake
        .url
        .as_str()
        .into_client_request()
        .map_err(|e| WsConnectError::failed(e.to_string()))?;
    for (key, value) in [
        ("Authorization", handshake.authorization.clone()),
        ("Sec-WebSocket-Protocol", handshake.protocol.clone()),
        ("User-Agent", crate::core::user_agent::generate_user_agent()),
    ] {
        request.headers_mut().insert(
            key,
            HeaderValue::try_from(value).map_err(|e| WsConnectError::failed(e.to_string()))?,
        );
    }
    let (socket, _) = connect_async(request).await.map_err(classify_error)?;
    Ok(socket.split())
}
pub(crate) struct Keepalive {
    interval: tokio::time::Interval,
    last_pong: tokio::time::Instant,
}
impl Keepalive {
    pub fn new() -> Self {
        let now = tokio::time::Instant::now();
        let period = std::time::Duration::from_secs(10);
        Self {
            interval: tokio::time::interval_at(now + period, period),
            last_pong: now,
        }
    }
    pub fn received_pong(&mut self) {
        self.last_pong = tokio::time::Instant::now();
    }
    pub async fn tick(&mut self, sink: &mut WsSink) -> Result<(), Error> {
        self.interval.tick().await;
        if self.last_pong.elapsed() >= std::time::Duration::from_secs(30) {
            return Err(Error::ConnectionClosed);
        }
        sink.send(Message::Ping(Vec::new().into())).await
    }
}

pub(crate) fn classify_error(e: Error) -> WsConnectError {
    WsConnectError {
        unauthorized: matches!(&e,Error::Http(r) if matches!(r.status().as_u16(),401|403)),
        message: e.to_string(),
    }
}

pub(crate) fn validate_configuration() -> anyhow::Result<()> {
    Ok(())
}
