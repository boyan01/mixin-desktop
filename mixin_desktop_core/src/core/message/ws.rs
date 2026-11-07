//! WebSocket transport for the Blaze session.
//!
//! Native delegates to `tokio-tungstenite`. wasm32 drives the browser
//! `WebSocket` API from inside the Worker: `web_sys::WebSocket` is
//! event-driven, so incoming frames arrive as `message` callbacks and are
//! forwarded through an mpsc channel that the shared select loop in
//! [`super::blaze`] consumes exactly like the native split stream.
//!
//! Native owns protocol keepalive; the browser host supplies its authenticated
//! WebSocket connector. Both deliver binary Blaze frames to the account loop.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
pub(super) mod native;

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::{connect, validate_configuration, Keepalive, Message};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{connect, validate_configuration, Keepalive, Message};

/// Everything a platform transport needs to open the Blaze session.
pub(crate) struct Handshake {
    /// `wss://` endpoint, e.g. `wss://blaze.mixin.one:443`.
    pub url: String,
    /// Signed bearer token passed to the platform connector.
    pub authorization: String,
    /// Requested subprotocol, `Mixin-Blaze-1`.
    pub protocol: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub proxy: Option<crate::network::ProxyConfig>,
}

/// Failure to open the WebSocket, classified for the connect loop.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub(crate) struct WsConnectError {
    /// True when the server rejected the handshake with 401/403. The
    /// browser hides handshake status codes, so the wasm transport always
    /// reports `false`.
    pub unauthorized: bool,
    pub message: String,
}

impl WsConnectError {
    pub(crate) fn failed(message: impl Into<String>) -> Self {
        Self {
            unauthorized: false,
            message: message.into(),
        }
    }
}

/// Transport failure surfaced to the shared select loop on wasm.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, thiserror::Error)]
#[error("blaze websocket: {0}")]
pub(crate) struct WsError(String);

#[cfg(target_arch = "wasm32")]
impl WsError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
