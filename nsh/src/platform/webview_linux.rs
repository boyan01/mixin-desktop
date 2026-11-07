//! Account-isolated GTK/WebKit windows with a native web-process bridge.
use std::rc::Rc;

use nsh_platform_linux::Components;

#[path = "webview_linux/chrome.rs"]
mod chrome;
#[path = "webview_linux/dialogs.rs"]
mod dialogs;
#[path = "webview_linux/downloads.rs"]
mod downloads;
#[path = "webview_linux/events.rs"]
mod events;
#[path = "webview_linux/host.rs"]
mod host;
#[path = "webview_linux/protocol.rs"]
mod protocol;
#[path = "webview_linux/session.rs"]
mod session;
#[path = "webview_linux/window.rs"]
mod window;

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinBotSession",
        Rc::new(|context| Box::new(session::Session::new(context))),
    );
    components.register(
        "MixinBotWindow",
        Rc::new(|context| Box::new(host::Host::new(context))),
    );
}
