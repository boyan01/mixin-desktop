//! Process-scoped native shell; account actions stay with the signed-in owner.
mod instance;
mod menus;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod window_state;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as native;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as native;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as native;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::prelude::*;
use std::{
    collections::VecDeque,
    io,
    path::Path,
    rc::Rc,
    sync::{Mutex, OnceLock},
};
use tokio::sync::mpsc;

pub use instance::Launch;

const MAX_PENDING: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationTarget {
    pub account_id: String,
    pub conversation_id: String,
    pub message_id: String,
}

impl NotificationTarget {
    pub(crate) fn valid(&self) -> bool {
        [&self.account_id, &self.conversation_id, &self.message_id]
            .into_iter()
            .all(|id| {
                id.len() == 36
                    && id.bytes().enumerate().all(|(at, byte)| {
                        if matches!(at, 8 | 13 | 18 | 23) {
                            byte == b'-'
                        } else {
                            byte.is_ascii_hexdigit()
                        }
                    })
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Activate,
    Open(String),
    Notification(NotificationTarget),
}

type Events = (mpsc::Sender<Event>, Mutex<Option<mpsc::Receiver<Event>>>);
fn events() -> &'static Events {
    static EVENTS: OnceLock<Events> = OnceLock::new();
    EVENTS.get_or_init(|| {
        let (sender, receiver) = mpsc::channel(MAX_PENDING);
        (sender, Mutex::new(Some(receiver)))
    })
}

pub(super) fn accepted_url(raw: &str) -> bool {
    if raw.len() > 1024 * 1024 {
        return false;
    }
    url::Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "mixin"
            || matches!(url.scheme(), "http" | "https")
                && matches!(url.host_str(), Some("mixin.one" | "www.mixin.one"))
    })
}

/// Call before creating the desktop runtime. Retain the primary guard until exit.
pub fn prepare_launch(
    directory: &Path,
    arguments: impl IntoIterator<Item = String>,
    notification: Option<NotificationTarget>,
) -> io::Result<Launch> {
    let urls = arguments
        .into_iter()
        .filter(|argument| accepted_url(argument))
        .collect::<Vec<_>>();
    if urls.len() + usize::from(notification.is_some()) > 16
        || notification.as_ref().is_some_and(|target| !target.valid())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid application launch targets",
        ));
    }
    let mut intents = urls.into_iter().map(Event::Open).collect::<Vec<_>>();
    if let Some(target) = notification {
        intents.push(Event::Notification(target));
    }
    instance::acquire(directory, intents, events().0.clone())
}

#[derive(Clone, Copy)]
pub(crate) enum Command {
    Preferences,
    NewChat,
    NewGroup,
    NewCircle,
    QuickSearch,
    Lock,
    PreviousChat,
    NextChat,
    Mute,
    SearchConversation,
    DeleteConversation,
    Pin,
    ToggleInfo,
    #[cfg(debug_assertions)]
    Transfer,
}

type Handler = Rc<dyn Fn(Command)>;

#[derive(Clone, Copy)]
pub(crate) struct Handle {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    reference: ElementRef,
    pending: Signal<Option<String>>,
    pub(crate) notification_targets: Signal<VecDeque<NotificationTarget>>,
    badge: Signal<i64>,
    signed_in: Signal<bool>,
    blocked: Signal<bool>,
    can_lock: Signal<bool>,
    has_chat: Signal<bool>,
    muted: Signal<bool>,
    pinned: Signal<bool>,
    handler: Stored<Option<Handler>>,
}

pub(crate) fn mount() -> Handle {
    let handle = Handle {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        reference: element_ref(),
        pending: signal(None),
        notification_targets: signal(VecDeque::new()),
        badge: signal(0),
        signed_in: signal(false),
        blocked: signal(false),
        can_lock: signal(false),
        has_chat: signal(false),
        muted: signal(false),
        pinned: signal(false),
        handler: stored(None),
    };
    let receiver = events().1.lock().expect("shell event receiver").take();
    if let Some(receiver) = receiver {
        spawn(async move {
            let mut events = Box::pin(crate::runtime::observe(futures::stream::unfold(
                receiver,
                |mut receiver| async { receiver.recv().await.map(|event| (event, receiver)) },
            )));
            while let Some(event) = events.next().await {
                handle.show();
                match event {
                    Event::Activate => {}
                    Event::Open(url) => handle.pending.set(Some(url)),
                    Event::Notification(target) => handle.notification_targets.update(|targets| {
                        if targets.len() == 16 {
                            targets.pop_front();
                        }
                        targets.push_back(target);
                    }),
                }
            }
        });
    }
    menus::mount(handle);
    handle
}

impl Handle {
    fn dispatch(self, command: Command) {
        if self.blocked.get_untracked() && !matches!(command, Command::Lock) {
            return;
        }
        if let Some(handler) = self.handler.with(|handler| handler.clone()) {
            self.show();
            handler(command);
        }
    }
    pub(crate) fn show(self) {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        self.reference.command("show", serde_json::Value::Null);
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        nsh_core::element::desktop::main_window().show();
    }
    fn hide(self) {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        self.reference.command("hide", serde_json::Value::Null);
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        nsh_core::element::desktop::main_window().hide();
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn quit(self) {
        self.reference
            .command("request-quit", serde_json::Value::Null);
    }
}

/// Attach account presentation state; dispose this with the signed-in session.
pub(crate) fn attach(
    handle: Handle,
    account: std::sync::Arc<AccountClient>,
    selected: Signal<Option<ConversationListData>>,
    blocked: impl Fn() -> bool + Copy + 'static,
    can_lock: impl Fn() -> bool + Copy + 'static,
    on_command: impl Fn(Command) + Copy + 'static,
) {
    let owner = Owner::current();
    let navigation =
        use_context::<crate::navigation::Navigation>().expect("Home provides sidebar navigation");
    handle.signed_in.set(true);
    handle.handler.replace(Some(Rc::new(move |command| {
        owner.run(|| on_command(command))
    })));
    cleanup(move || {
        handle.handler.replace(None);
        batch(|| {
            handle.signed_in.set(false);
            handle.has_chat.set(false);
            handle.can_lock.set(false);
            handle.blocked.set(false);
            handle.badge.set(0);
        });
    });
    effect(move || {
        batch(|| {
            handle.blocked.set(blocked());
            handle.can_lock.set(can_lock());
            handle
                .has_chat
                .set(!navigation.settings.get() && selected.with(Option::is_some));
        });
    });
    let menu_account = account.clone();
    effect(move || {
        let row = selected.get();
        handle
            .muted
            .set(row.as_ref().is_some_and(|row| row.is_muted));
        handle
            .pinned
            .set(row.as_ref().is_some_and(|row| row.is_pinned));
        let Some(id) = row.map(|row| row.conversation_id) else {
            return;
        };
        let client = menu_account.clone();
        spawn(async move {
            let mut changes = Box::pin(crate::runtime::observe(client.conversation_changes()));
            loop {
                let query_client = client.clone();
                let requested_id = id.clone();
                let result = crate::runtime::run(async move {
                    query_client
                        .conversation()
                        .conversation_items_by_ids(vec![requested_id])
                        .await
                })
                .await;
                if !selected.with(|row| row.as_ref().is_some_and(|row| row.conversation_id == id)) {
                    return;
                }
                if let Ok(rows) = result {
                    let row = rows.first();
                    batch(|| {
                        handle.muted.set(row.is_some_and(|row| row.is_muted));
                        handle.pinned.set(row.is_some_and(|row| row.is_pinned));
                    });
                }
                loop {
                    let Some(event) = changes.next().await else {
                        return;
                    };
                    if event.reload_all || event.conversation_ids.contains(&id) {
                        break;
                    }
                }
            }
        });
    });
    let links = use_context::<crate::links::LinkNavigation>().expect("Home provides link routing");
    effect(move || {
        if blocked() {
            return;
        }
        if let Some(url) = handle.pending.get() {
            handle.pending.set(None);
            links.open(url, None);
        }
    });
    spawn(async move {
        let mut counts = Box::pin(crate::runtime::observe(
            account.unseen_message_count_changes(),
        ));
        while let Some(count) = counts.next().await {
            handle.badge.set(count.max(0));
        }
    });
}

pub(crate) fn surface(handle: Handle) -> impl IntoElement {
    #[cfg(target_os = "macos")]
    {
        native("MixinShell")
            .element_ref(&handle.reference)
            .prop("badge", move || handle.badge.get())
            .prop("blocked", move || handle.blocked.get())
            .on_native_event(move |event: &NativeEvent| {
                if event.name.as_ref() == "sheet" && handle.blocked.get_untracked() {
                    handle
                        .reference
                        .command("cancel-sheet", serde_json::Value::Null);
                }
            })
            .w_0()
            .h_0()
            .into_element()
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        window_state::surface(handle)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = handle;
        view().w_0().h_0().into_element()
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn register_native_components(components: &nsh_platform_macos::Components) {
    native::register(components);
}

#[cfg(target_os = "windows")]
pub(crate) fn register_native_components(components: &nsh_platform_windows::Components) {
    native::register(components);
}

#[cfg(target_os = "linux")]
pub(crate) fn register_native_components(components: &nsh_platform_linux::Components) {
    native::register(components);
}
