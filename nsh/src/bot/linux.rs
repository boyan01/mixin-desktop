//! Account-isolated GTK/WebKit windows with a native web-process bridge.
use std::rc::Rc;

use nsh_platform_linux::Components;

mod chrome;
mod dialogs;
mod downloads;
mod events;
mod host;
mod protocol;
mod session;
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
