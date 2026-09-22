//! wasm32 transport backed by the browser `WebSocket` API.
//!
//! `web_sys::WebSocket` is event-driven: `message` and `close` callbacks
//! push frames into an mpsc channel that [`WsStream`] exposes as a normal
//! `Stream`. [`WsSink`] writes binary frames synchronously; the browser
//! answers protocol-level pings itself and reports dead connections with a
//! `close` event, which drops the last channel sender and ends the stream.
//!
//! Authentication is supplied by the host connector; no query-token protocol
//! is assumed. Configure it before starting an account.

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use futures::{Sink, Stream};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, MessageEvent, WebSocket};

use super::{Handshake, WsConnectError, WsError};

/// Frames the shared Blaze loop exchanges with the browser transport. The
/// browser hides `Ping`/`Pong`/`Close` frames (it implements the close
/// handshake itself and ends [`WsStream`] instead), so only binary payload
/// frames surface here.
#[derive(Debug)]
pub(crate) enum Message {
    Binary(Vec<u8>),
}

impl Message {
    pub(crate) fn into_data(self) -> Vec<u8> {
        match self {
            Message::Binary(data) => data,
        }
    }
}

type Slot = Rc<RefCell<Option<UnboundedSender<Result<Message, WsError>>>>>;

/// Registers `onopen`/`onerror`, and clears both handlers when dropped so a
/// timed-out handshake cannot invoke a released Rust closure.
struct PendingConnect {
    ws: Rc<WebSocket>,
    _on_open: Closure<dyn FnMut()>,
    _on_error: Closure<dyn FnMut()>,
}

impl Drop for PendingConnect {
    fn drop(&mut self) {
        self.ws.set_onopen(None);
        self.ws.set_onerror(None);
        if self.ws.ready_state() == WebSocket::CONNECTING {
            let _ = self.ws.close();
        }
    }
}

pub(crate) async fn connect(handshake: &Handshake) -> Result<(WsSink, WsStream), WsConnectError> {
    let ws = Rc::new(
        create_socket(
            &handshake.url,
            &handshake.authorization,
            &handshake.protocol,
        )
        .map_err(|_| WsConnectError::failed("browser Blaze connector failed"))?,
    );
    ws.set_binary_type(BinaryType::Arraybuffer);

    let (open_sender, open_receiver) = oneshot::channel::<Result<(), String>>();
    let open_slot = Rc::new(RefCell::new(Some(open_sender)));
    let on_open = Closure::wrap(Box::new({
        let open_slot = open_slot.clone();
        move || {
            let pending = open_slot.borrow_mut().take();
            if let Some(pending) = pending {
                let _ = pending.send(Ok(()));
            }
        }
    }) as Box<dyn FnMut()>);
    let on_error = Closure::wrap(Box::new({
        let open_slot = open_slot;
        move || {
            let pending = open_slot.borrow_mut().take();
            if let Some(pending) = pending {
                let _ = pending.send(Err("websocket failed before open".to_owned()));
            }
        }
    }) as Box<dyn FnMut()>);
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    let pending = PendingConnect {
        ws: ws.clone(),
        _on_open: on_open,
        _on_error: on_error,
    };
    let outcome = open_receiver
        .await
        .map_err(|_| WsConnectError::failed("websocket closed before open"))?;
    drop(pending);
    outcome.map_err(WsConnectError::failed)?;

    let (sender, receiver) = unbounded::<Result<Message, WsError>>();
    let slot: Slot = Rc::new(RefCell::new(Some(sender)));
    let on_message = Closure::wrap(Box::new({
        let slot = slot.clone();
        move |event: MessageEvent| {
            let frame = match event.data().dyn_into::<js_sys::ArrayBuffer>() {
                Ok(buffer) => Ok(Message::Binary(js_sys::Uint8Array::new(&buffer).to_vec())),
                Err(other) => Err(match other.dyn_into::<js_sys::JsString>() {
                    Ok(text) => format!("unexpected text frame: {text}"),
                    Err(_) => "unexpected non-binary websocket frame".to_owned(),
                }),
            };
            let sender = slot.borrow();
            if let Some(sender) = sender.as_ref() {
                let _ = sender.unbounded_send(match frame {
                    Ok(message) => Ok(message),
                    Err(detail) => Err(WsError::new(detail)),
                });
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    let on_close = Closure::wrap(Box::new({
        let slot = slot;
        move || {
            // Dropping the last sender ends the stream, which the shared
            // select loop already treats as a disconnect.
            *slot.borrow_mut() = None;
        }
    }) as Box<dyn FnMut()>);
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

    Ok((
        WsSink {
            ws,
            _on_message: on_message,
            _on_close: on_close,
        },
        WsStream { receiver },
    ))
}

/// Sending side of the session. Dropping it closes the browser socket and
/// releases the event handlers, which in turn drops the last channel sender.
pub(crate) struct WsSink {
    ws: Rc<WebSocket>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut()>,
}

impl Drop for WsSink {
    fn drop(&mut self) {
        self.ws.set_onmessage(None);
        self.ws.set_onclose(None);
        let _ = self.ws.close();
    }
}

impl Sink<Message> for WsSink {
    type Error = WsError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, message: Message) -> Result<(), Self::Error> {
        let Message::Binary(data) = message;
        self.ws
            .send_with_u8_array(&data)
            .map_err(|error| WsError::new(format!("websocket send failed: {error:?}")))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let _ = self.ws.close();
        Poll::Ready(Ok(()))
    }
}

/// Receiving side of the session: `message` callbacks feed the channel, and
/// a `close` event drops the sender so the stream simply ends.
pub(crate) struct WsStream {
    receiver: UnboundedReceiver<Result<Message, WsError>>,
}

impl Stream for WsStream {
    type Item = Result<Message, WsError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
    }
}

// Browser protocol pongs are managed by the user agent. Application requests
// retain their shared response deadlines.
pub(crate) struct Keepalive;
impl Keepalive {
    pub fn new() -> Self {
        Self
    }
    pub async fn tick(&mut self, _sink: &mut WsSink) -> Result<(), WsError> {
        std::future::pending().await
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(module = "/web/platform.js")]
extern "C" {
    #[wasm_bindgen(catch,js_name=createBlazeSocket)]
    fn create_socket(
        endpoint: &str,
        authorization: &str,
        protocol: &str,
    ) -> Result<WebSocket, wasm_bindgen::JsValue>;
    #[wasm_bindgen(catch,js_name=validateBlazeConnector)]
    fn validate_connector() -> Result<(), wasm_bindgen::JsValue>;
}
pub(crate) fn validate_configuration() -> anyhow::Result<()> {
    validate_connector().map_err(|_| anyhow::anyhow!("browser Blaze connector is not configured"))
}
