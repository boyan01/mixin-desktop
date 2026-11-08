//! Packaged Win32 toast delivery. All WinRT objects remain on one MTA worker.
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex, OnceLock, mpsc as sync_mpsc},
    time::Duration,
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::{Notify, mpsc, oneshot};
use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{
        NotificationSetting, ToastDismissalReason, ToastDismissedEventArgs, ToastFailedEventArgs,
        ToastNotification, ToastNotificationHistory, ToastNotificationManager, ToastNotifier,
    },
    core::HSTRING,
};

use super::{Availability, Error, Event, Notification, Permission};

mod activation;

const CAPACITY: usize = 512;
const SHUTDOWN_WAIT: Duration = Duration::from_secs(6);
const COMPLETION_WAIT: Duration = Duration::from_secs(5);

#[derive(Clone, PartialEq)]
struct Payload {
    id: String,
    account: String,
    conversation: String,
    message: String,
    generation: u64,
}

impl Payload {
    fn decode(source: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(source).ok()?;
        if value["mixin_nsh_notification"].as_u64()? != 1 {
            return None;
        }
        let read = |key: &str| {
            value[key]
                .as_str()
                .filter(|text| {
                    !text.is_empty()
                        && text.len() <= 128
                        && text.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_')
                        })
                })
                .map(str::to_owned)
        };
        let payload = Self {
            id: read("id")?,
            account: read("account")?,
            conversation: read("conversation")?,
            message: read("message")?,
            generation: value["generation"].as_u64()?,
        };
        (payload.id == format!("{}:{}", payload.account, payload.message)).then_some(payload)
    }

    fn encode(&self) -> String {
        json!({"mixin_nsh_notification":1, "id":self.id, "account":self.account,
            "conversation":self.conversation, "message":self.message,
            "generation":self.generation})
        .to_string()
    }

    fn event(self) -> Event {
        Event::Clicked {
            account_id: self.account,
            conversation_id: self.conversation,
            message_id: self.message,
        }
    }
}

pub(super) fn availability() -> Availability {
    if activation::identity().is_some() {
        Availability::Available
    } else {
        Availability::MissingAppBundle
    }
}

/// Run before acquiring the ordinary single-instance lock. Installation supplies
/// this fixed COM server argument; ordinary launches do no activation work here.
pub(super) fn run_activation_server() -> Result<Option<Event>, Error> {
    if !std::env::args().any(|argument| argument == activation::FLAG) {
        return Ok(None);
    }
    let app_id =
        activation::identity().ok_or(Error::Unavailable(Availability::MissingAppBundle))?;
    let _apartment = activation::Apartment::new().map_err(native_error)?;
    let (sender, receiver) = sync_mpsc::sync_channel(1);
    let _registration = activation::Registration::new(
        app_id,
        Arc::new(move |payload| sender.try_send(payload.event()).is_ok()),
    )
    .map_err(native_error)?;
    receiver
        .recv_timeout(Duration::from_secs(15))
        .map(Some)
        .map_err(|_| {
            Error::Native("Notification activation did not arrive before its deadline".into())
        })
}

#[derive(Default)]
struct Registry {
    generation: u64,
    events: Option<mpsc::UnboundedSender<Event>>,
    current: HashMap<String, u64>,
    // Every queued, shown or not-yet-retracted generation consumes one slot.
    // Dismissal does not free its slot until native retraction has completed.
    slots: HashMap<u64, Payload>,
    shutdown: bool,
}

struct Shared {
    registry: Mutex<Registry>,
    wake: Notify,
}

impl Shared {
    fn retire(&self, payload: &Payload) {
        let mut registry = self.registry.lock().expect("notification registry");
        if registry.current.get(&payload.id) == Some(&payload.generation) {
            registry.current.remove(&payload.id);
        }
        drop(registry);
        self.wake.notify_one();
    }

    fn forget(&self, generation: u64) {
        let mut registry = self.registry.lock().expect("notification registry");
        if let Some(payload) = registry.slots.remove(&generation)
            && registry.current.get(&payload.id) == Some(&generation)
        {
            registry.current.remove(&payload.id);
        }
    }

    fn stop(&self) {
        let mut registry = self.registry.lock().expect("notification registry");
        registry.shutdown = true;
        registry.current.clear();
        registry.events.take();
        drop(registry);
        self.wake.notify_one();
    }
}

type Completion<T> = oneshot::Sender<Result<T, Error>>;
enum Command {
    Permission(Completion<Permission>),
    Deliver(Box<Notification>, Payload, Completion<()>),
}

struct Worker {
    shared: Arc<Shared>,
    done: sync_mpsc::Receiver<()>,
    thread: std::thread::JoinHandle<()>,
}

fn worker_slot() -> &'static Mutex<Option<Worker>> {
    static WORKER: OnceLock<Mutex<Option<Worker>>> = OnceLock::new();
    WORKER.get_or_init(Mutex::default)
}

/// Called once after the UI loop. A stalled OS RPC cannot block process exit.
pub(super) fn finish_shutdown() {
    let worker = worker_slot().lock().expect("notification worker").take();
    if let Some(worker) = worker {
        worker.shared.stop();
        if worker.done.recv_timeout(SHUTDOWN_WAIT).is_ok() {
            let _ = worker.thread.join();
        } else {
            log::warn!("The system notification worker did not finish before shutdown");
        }
    }
}

pub(super) struct NativeNotifications {
    commands: mpsc::Sender<Command>,
    shared: Arc<Shared>,
}

impl NativeNotifications {
    pub(super) fn new() -> Result<(Self, mpsc::UnboundedReceiver<Event>), Error> {
        let app_id =
            activation::identity().ok_or(Error::Unavailable(Availability::MissingAppBundle))?;
        let mut slot = worker_slot().lock().expect("notification worker");
        if slot
            .as_ref()
            .is_some_and(|worker| !worker.thread.is_finished())
        {
            return Err(unavailable());
        }
        if let Some(previous) = slot.take() {
            let _ = previous.thread.join();
        }
        let (events, received) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared {
            registry: Mutex::new(Registry {
                events: Some(events),
                ..Registry::default()
            }),
            wake: Notify::new(),
        });
        let (commands, receiver) = mpsc::channel(32);
        let (done, completed) = sync_mpsc::sync_channel(1);
        let state = shared.clone();
        let thread = std::thread::Builder::new()
            .name("mixin-notifications".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match result {
                    Ok(runtime) => runtime.block_on(worker(app_id, state.clone(), receiver)),
                    Err(error) => log::warn!("Could not start notification runtime: {error}"),
                }
                state.stop();
                let _ = done.send(());
            })
            .map_err(|error| {
                Error::Native(format!("Could not start notification worker: {error}"))
            })?;
        *slot = Some(Worker {
            shared: shared.clone(),
            done: completed,
            thread,
        });
        Ok((Self { commands, shared }, received))
    }

    pub(super) fn permission(
        &self,
    ) -> impl Future<Output = Result<Permission, Error>> + Send + 'static + use<> {
        let (reply, received) = oneshot::channel();
        let accepted = if self
            .shared
            .registry
            .lock()
            .expect("notification registry")
            .shutdown
        {
            Err(unavailable())
        } else {
            self.commands
                .try_send(Command::Permission(reply))
                .map_err(|_| unavailable())
        };
        async move {
            accepted?;
            tokio::time::timeout(COMPLETION_WAIT, received)
                .await
                .map_err(|_| deadline())?
                .map_err(|_| unavailable())?
        }
    }

    pub(super) fn deliver(
        &self,
        notification: Notification,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'static + use<> {
        let (reply, received) = oneshot::channel();
        let accepted = (|| {
            let permit = self.commands.try_reserve().map_err(|_| unavailable())?;
            let mut registry = self.shared.registry.lock().expect("notification registry");
            if registry.shutdown || registry.slots.len() >= CAPACITY {
                return Err(unavailable());
            }
            registry.generation = registry.generation.checked_add(1).ok_or_else(unavailable)?;
            let payload = Payload {
                id: notification.id.clone(),
                account: notification.account_id.clone(),
                conversation: notification.conversation_id.clone(),
                message: notification.message_id.clone(),
                generation: registry.generation,
            };
            if Payload::decode(&payload.encode()).is_none() {
                return Err(Error::Native("Invalid notification target".into()));
            }
            registry
                .current
                .insert(payload.id.clone(), payload.generation);
            registry.slots.insert(payload.generation, payload.clone());
            permit.send(Command::Deliver(
                Box::new(notification),
                payload.clone(),
                reply,
            ));
            Ok(payload)
        })();
        self.shared.wake.notify_one();
        let shared = self.shared.clone();
        async move {
            let payload = accepted?;
            match tokio::time::timeout(COMPLETION_WAIT, received).await {
                Ok(Ok(result)) => result,
                result => {
                    // Retire this identity synchronously. A blocked native Show
                    // may still complete; its worker must then retract the late
                    // toast before releasing the generation's reserved slot.
                    shared.retire(&payload);
                    Err(if result.is_err() {
                        deadline()
                    } else {
                        unavailable()
                    })
                }
            }
        }
    }

    pub(super) fn dismiss(&self, ids: &[String]) {
        let mut registry = self.shared.registry.lock().expect("notification registry");
        for id in ids {
            registry.current.remove(id);
        }
        drop(registry);
        self.shared.wake.notify_one();
    }

    pub(super) fn existing_ids(
        &self,
        account: String,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send + 'static + use<> {
        let registry = self.shared.registry.lock().expect("notification registry");
        let ids = registry
            .current
            .iter()
            .filter_map(|(id, generation)| {
                registry
                    .slots
                    .get(generation)
                    .filter(|payload| payload.account == account)
                    .map(|_| id.clone())
            })
            .collect();
        async move { Ok(ids) }
    }

    pub(super) fn shutdown(&self) {
        self.shared.stop();
    }
}

struct Shown {
    toast: ToastNotification,
    tag: HSTRING,
    group: HSTRING,
    dismissed: i64,
    failed: i64,
}

impl Drop for Shown {
    fn drop(&mut self) {
        let _ = self.toast.RemoveDismissed(self.dismissed);
        let _ = self.toast.RemoveFailed(self.failed);
    }
}

struct Backend {
    _registration: activation::Registration,
    notifier: ToastNotifier,
    history: ToastNotificationHistory,
    shown: HashMap<u64, Shown>,
}

impl Backend {
    fn new(app_id: &str, shared: Arc<Shared>) -> Result<Self, Error> {
        let active = shared.clone();
        let registration = activation::Registration::new(
            app_id.into(),
            Arc::new(move |payload| {
                let mut registry = active.registry.lock().expect("notification registry");
                if !registry.shutdown
                    && registry.current.get(&payload.id) == Some(&payload.generation)
                    && registry.slots.get(&payload.generation) == Some(&payload)
                {
                    registry.current.remove(&payload.id);
                    if let Some(events) = registry.events.as_ref() {
                        let _ = events.send(payload.event());
                    }
                    active.wake.notify_one();
                }
                true
            }),
        )
        .map_err(native_error)?;
        let notifier = ToastNotificationManager::CreateToastNotifier().map_err(native_error)?;
        let history = ToastNotificationManager::History().map_err(native_error)?;
        // This API scopes history to this validated package's current application.
        // Clear prior-process notifications as the reference does on initialization.
        history.Clear().map_err(native_error)?;
        Ok(Self {
            _registration: registration,
            notifier,
            history,
            shown: HashMap::new(),
        })
    }

    fn permission(&self) -> Result<Permission, Error> {
        Ok(match self.notifier.Setting().map_err(native_error)? {
            NotificationSetting::Enabled => Permission::Authorized,
            NotificationSetting::DisabledForApplication
            | NotificationSetting::DisabledForUser
            | NotificationSetting::DisabledByGroupPolicy
            | NotificationSetting::DisabledByManifest => Permission::Denied,
            _ => Permission::Unknown,
        })
    }

    fn show(
        &mut self,
        notification: Notification,
        payload: &Payload,
        shared: Arc<Shared>,
    ) -> Result<(), Error> {
        let tag = short_hash(&payload.id);
        let group = short_hash(&format!("{}:{}", payload.account, payload.conversation));
        // A failed old retraction must not later remove a new toast carrying
        // the same native tag/group. Wait for that reservation to be released.
        if self
            .shown
            .values()
            .any(|shown| shown.tag == tag && shown.group == group)
        {
            return Err(unavailable());
        }
        if !self.permission()?.permits_delivery() {
            return Err(Error::Native(
                "System notification delivery is disabled".into(),
            ));
        }
        let xml = XmlDocument::new().map_err(native_error)?;
        xml.LoadXml(&HSTRING::from(format!("<toast duration='short' launch='{}'><visual><binding template='ToastGeneric'><text>{}</text><text>{}</text></binding></visual></toast>",
            escape(&payload.encode(),4096),escape(&notification.title,256),escape(&notification.body,4096)))).map_err(native_error)?;
        let toast = ToastNotification::CreateToastNotification(&xml).map_err(native_error)?;
        toast.SetTag(&tag).map_err(native_error)?;
        toast.SetGroup(&group).map_err(native_error)?;
        let target = payload.clone();
        let dismissed_state = shared.clone();
        let dismissed = toast
            .Dismissed(&TypedEventHandler::<
                ToastNotification,
                ToastDismissedEventArgs,
            >::new(move |_, args| {
                if args.as_ref().and_then(|args| args.Reason().ok())
                    != Some(ToastDismissalReason::TimedOut)
                {
                    dismissed_state.retire(&target);
                }
                Ok(())
            }))
            .map_err(native_error)?;
        let target = payload.clone();
        let failed = match toast.Failed(
            &TypedEventHandler::<ToastNotification, ToastFailedEventArgs>::new(move |_, args| {
                if let Some(args) = args.as_ref()
                    && let Ok(error) = args.ErrorCode()
                {
                    log::warn!("System notification failed: {error:?}");
                }
                shared.retire(&target);
                Ok(())
            }),
        ) {
            Ok(value) => value,
            Err(error) => {
                let _ = toast.RemoveDismissed(dismissed);
                return Err(native_error(error));
            }
        };
        let shown = Shown {
            toast,
            tag,
            group,
            dismissed,
            failed,
        };
        // Keep the object even if Show reports an RPC error: the system may
        // already have accepted it. The failed generation is then retracted.
        self.shown.insert(payload.generation, shown);
        self.notifier
            .Show(&self.shown[&payload.generation].toast)
            .map_err(native_error)
    }

    fn sweep(&mut self, shared: &Shared) {
        let inactive: Vec<_> = {
            let registry = shared.registry.lock().expect("notification registry");
            registry
                .slots
                .iter()
                .filter(|(generation, payload)| {
                    registry.current.get(&payload.id) != Some(generation)
                })
                .map(|(generation, _)| *generation)
                .collect()
        };
        for generation in inactive {
            if let Some(shown) = self.shown.get(&generation) {
                let hidden = self.notifier.Hide(&shown.toast);
                let removed = self.history.RemoveGroupedTag(&shown.tag, &shown.group);
                if hidden.is_err() || removed.is_err() {
                    log::warn!(
                        "Could not retract a system notification; retaining its reservation"
                    );
                    continue;
                }
            }
            self.shown.remove(&generation);
            shared.forget(generation);
        }
    }
}

async fn worker(app_id: String, shared: Arc<Shared>, mut commands: mpsc::Receiver<Command>) {
    let _apartment = match activation::Apartment::new() {
        Ok(value) => value,
        Err(error) => {
            log::warn!("Could not initialize notification apartment: {error}");
            return;
        }
    };
    let mut backend = None::<Backend>;
    loop {
        if let Some(backend) = &mut backend {
            backend.sweep(&shared);
        }
        if shared
            .registry
            .lock()
            .expect("notification registry")
            .shutdown
        {
            return;
        }
        let command = tokio::select! {biased;_ =shared.wake.notified()=>continue,command=commands.recv()=>command};
        let Some(command) = command else {
            shared.stop();
            continue;
        };
        if backend.is_none() {
            match Backend::new(&app_id, shared.clone()) {
                Ok(value) => backend = Some(value),
                Err(error) => {
                    match command {
                        Command::Permission(reply) => {
                            let _ = reply.send(Err(error));
                        }
                        Command::Deliver(_, payload, reply) => {
                            shared.forget(payload.generation);
                            let _ = reply.send(Err(error));
                        }
                    }
                    continue;
                }
            }
        }
        let backend = backend.as_mut().expect("initialized notification backend");
        match command {
            Command::Permission(reply) => {
                if !reply.is_closed() {
                    let _ = reply.send(backend.permission());
                }
            }
            Command::Deliver(notification, payload, reply) => {
                let active = {
                    let registry = shared.registry.lock().expect("notification registry");
                    !registry.shutdown
                        && registry.current.get(&payload.id) == Some(&payload.generation)
                        && registry.slots.contains_key(&payload.generation)
                };
                let result = if active {
                    backend.show(*notification, &payload, shared.clone())
                } else {
                    Err(unavailable())
                };
                if result.is_err() {
                    if backend.shown.contains_key(&payload.generation) {
                        shared.retire(&payload);
                    } else {
                        shared.forget(payload.generation);
                    }
                }
                let _ = reply.send(result);
            }
        }
    }
}

fn unavailable() -> Error {
    Error::Native("The notification service is unavailable or busy. Try again.".into())
}
fn native_error(error: windows::core::Error) -> Error {
    Error::Native(format!("System notification error: {error}"))
}

fn escape(value: &str, limit: usize) -> String {
    value
        .chars()
        .take(limit)
        .filter(|value| {
            matches!(value, '\t' | '\r' | '\n')
                || (*value >= ' ' && *value != '\u{fffe}' && *value != '\u{ffff}')
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

fn short_hash(value: &str) -> HSTRING {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(16);
    for byte in Sha256::digest(value.as_bytes()).iter().take(8) {
        write!(&mut output, "{byte:02x}").expect("String formatting cannot fail");
    }
    output.into()
}

fn deadline() -> Error {
    Error::Native(
        "The system notification service did not respond before its deadline. Try again.".into(),
    )
}
