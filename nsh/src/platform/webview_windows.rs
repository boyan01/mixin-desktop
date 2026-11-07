//! Account-isolated WebView2 windows on the native STA thread.
use std::rc::Rc;

use nsh_platform_windows::Components;

#[path = "webview_windows/chrome.rs"]
mod chrome;
#[path = "webview_windows/dispatch.rs"]
mod dispatch;
#[path = "webview_windows/downloads.rs"]
mod downloads;
#[path = "webview_windows/events.rs"]
mod events;
#[path = "webview_windows/focus.rs"]
mod focus;
#[path = "webview_windows/session.rs"]
mod session;
#[path = "webview_windows/window.rs"]
mod window;

pub(crate) fn register(components: &Components) {
    components.register(
        "MixinBotSession",
        Rc::new(|context| Box::new(session::Session::new(context))),
    );
    components.register(
        "MixinBotWindow",
        Rc::new(|context| Box::new(window::Host::new(context))),
    );
}
