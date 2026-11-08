//! Freedesktop notification delivery. One worker owns the session-bus connection.
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use futures::StreamExt as _;
use tokio::sync::{Notify, mpsc, oneshot};
use zbus::{Connection, Message, Proxy, proxy::SignalStream, zvariant::Value};

use super::{Error, Event, Notification, Permission};

const SERVICE: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
const CAPACITY: usize = 512;
const CALL_TIMEOUT: Duration = Duration::from_secs(3);

type Completion<T> = oneshot::Sender<Result<T, Error>>;

enum Command {
    Permission(Completion<Permission>),
    Deliver(Notification, u64, Completion<()>),
}

struct Entry {
    generation: u64,
    account: String,
    conversation: String,
    message: String,
    server_id: Option<u32>,
}

#[derive(Default)]
struct Registry {
    generation: u64,
    owner: Option<String>,
    entries: HashMap<String, Entry>,
    pending_close: HashSet<u32>,
    inflight: Option<(String, u64)>,
    shutdown: bool,
}

impl Registry {
    fn reserved(&self) -> usize {
        // A canceled/replaced in-flight request keeps its slot until its
        // reply can be retracted. Failed retractions keep their slots too.
        self.entries.len()
            + self.pending_close.len()
            + usize::from(self.inflight.as_ref().is_some_and(|(id, generation)| {
                self.entries
                    .get(id)
                    .is_none_or(|entry| entry.generation != *generation)
            }))
    }
}

struct Shared {
    registry: Mutex<Registry>,
    wake: Notify,
}

struct Worker {
    shared: Arc<Shared>,
    thread: std::thread::JoinHandle<()>,
}

fn process_worker() -> &'static Mutex<Option<Worker>> {
    static WORKER: OnceLock<Mutex<Option<Worker>>> = OnceLock::new();
    WORKER.get_or_init(Mutex::default)
}

/// Called after the native event loop returns, never from a UI callback.
pub(super) fn finish_shutdown() {
    let worker = process_worker().lock().expect("notification worker").take();
    if let Some(worker) = worker {
        request_shutdown(&worker.shared);
        if worker.thread.join().is_err() {
            log::warn!("The notification worker ended unexpectedly");
        }
    }
}

pub(super) struct NativeNotifications {
    commands: mpsc::Sender<Command>,
    shared: Arc<Shared>,
}

impl NativeNotifications {
    pub(super) fn new() -> Result<(Self, mpsc::UnboundedReceiver<Event>), Error> {
        let mut process = process_worker().lock().expect("notification worker");
        if process
            .as_ref()
            .is_some_and(|worker| !worker.thread.is_finished())
        {
            return Err(Error::Native(
                "A notification worker is already registered".into(),
            ));
        }
        if let Some(previous) = process.take() {
            let _ = previous.thread.join();
        }
        let (commands, receiver) = mpsc::channel(32);
        let (events, received) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared {
            registry: Mutex::default(),
            wake: Notify::new(),
        });
        let state = shared.clone();
        let thread = std::thread::Builder::new()
            .name("mixin-notifications".into())
            .spawn(move || {
                match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime.block_on(worker(state, receiver, events)),
                    Err(error) => log::warn!("Notification worker could not start: {error}"),
                }
            })
            .map_err(|error| {
                Error::Native(format!("Could not start notification worker: {error}"))
            })?;
        *process = Some(Worker {
            shared: shared.clone(),
            thread,
        });
        Ok((Self { commands, shared }, received))
    }

    pub(super) fn permission(
        &self,
    ) -> impl Future<Output = Result<Permission, Error>> + Send + 'static + use<> {
        let (completion, receiver) = oneshot::channel();
        let accepted = self.commands.try_send(Command::Permission(completion));
        async move {
            accepted.map_err(|_| unavailable())?;
            receiver.await.map_err(|_| unavailable())?
        }
    }

    pub(super) fn deliver(
        &self,
        notification: Notification,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'static + use<> {
        let (completion, receiver) = oneshot::channel();
        let accepted = (|| {
            let permit = self.commands.try_reserve().map_err(|_| unavailable())?;
            let mut state = self.shared.registry.lock().expect("notification registry");
            let additional = state.entries.get(&notification.id).map_or(1, |entry| {
                usize::from(
                    entry.server_id.is_some()
                        || state.inflight.as_ref().is_some_and(|(id, generation)| {
                            id == &notification.id && *generation == entry.generation
                        }),
                )
            });
            if state.shutdown || state.reserved() + additional > CAPACITY {
                return Err(unavailable());
            }
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            if let Some(previous) = state.entries.remove(&notification.id)
                && let Some(id) = previous.server_id
            {
                state.pending_close.insert(id);
            }
            state.entries.insert(
                notification.id.clone(),
                Entry {
                    generation,
                    account: notification.account_id.clone(),
                    conversation: notification.conversation_id.clone(),
                    message: notification.message_id.clone(),
                    server_id: None,
                },
            );
            permit.send(Command::Deliver(notification, generation, completion));
            self.shared.wake.notify_one();
            Ok(())
        })();
        async move {
            accepted?;
            receiver.await.map_err(|_| unavailable())?
        }
    }

    pub(super) fn dismiss(&self, ids: &[String]) {
        let mut state = self.shared.registry.lock().expect("notification registry");
        for id in ids {
            if let Some(entry) = state.entries.remove(id)
                && let Some(id) = entry.server_id
            {
                state.pending_close.insert(id);
            }
        }
        drop(state);
        // Retraction is state, not a fallible try_send on a full delivery queue.
        self.shared.wake.notify_one();
    }

    pub(super) fn existing_ids(
        &self,
        account_id: String,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send + 'static + use<> {
        let state = self.shared.registry.lock().expect("notification registry");
        let ids = state
            .entries
            .iter()
            .filter(|(_, entry)| entry.account == account_id && entry.server_id.is_some())
            .map(|(id, _)| id.clone())
            .collect();
        async move { Ok(ids) }
    }

    pub(super) fn shutdown(&self) {
        request_shutdown(&self.shared);
    }
}

fn request_shutdown(shared: &Shared) {
    let mut state = shared.registry.lock().expect("notification registry");
    state.shutdown = true;
    let entries = std::mem::take(&mut state.entries);
    state
        .pending_close
        .extend(entries.into_values().filter_map(|entry| entry.server_id));
    drop(state);
    shared.wake.notify_one();
}

fn unavailable() -> Error {
    Error::Native("The notification service is unavailable or busy. Try again.".into())
}

struct Bus {
    proxy: Proxy<'static>,
    owner: String,
    actions: SignalStream<'static>,
    closed: SignalStream<'static>,
    capabilities: Vec<String>,
}

impl Bus {
    async fn connect() -> Result<Self, Error> {
        bounded(async {
            let connection = Connection::session().await?;
            let bus = zbus::fdo::DBusProxy::new(&connection).await?;
            let owner = bus.get_name_owner(SERVICE.try_into()?).await?.to_string();
            // Pin this server's unique bus name. A restarted daemon can reuse
            // numeric IDs; its actions must never navigate an older account.
            let proxy = Proxy::new_owned(connection, owner.clone(), PATH, SERVICE).await?;
            let capabilities = proxy.call("GetCapabilities", &()).await?;
            // Install both subscriptions before the first Notify call.
            let actions = proxy.receive_signal("ActionInvoked").await?;
            let closed = proxy.receive_signal("NotificationClosed").await?;
            Ok(Self {
                proxy,
                owner,
                actions,
                closed,
                capabilities,
            })
        })
        .await
    }

    async fn deliver(&self, notification: &Notification) -> Result<u32, Error> {
        let actions = if self.capabilities.iter().any(|value| value == "actions") {
            vec!["default", ""]
        } else {
            Vec::new()
        };
        let body: String = notification.body.chars().take(4096).collect();
        let body = if self.capabilities.iter().any(|value| value == "body-markup") {
            body.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        } else {
            body
        };
        let title: String = notification.title.chars().take(256).collect();
        let hints = HashMap::from([
            ("desktop-entry", Value::from("one.mixin.messenger.nsh")),
            ("category", Value::from("im.received")),
            ("sound-name", Value::from("message-new-instant")),
        ]);
        bounded(async {
            self.proxy
                .call(
                    "Notify",
                    &(
                        "Mixin Messenger",
                        0_u32,
                        "",
                        title,
                        body,
                        actions,
                        hints,
                        -1_i32,
                    ),
                )
                .await
        })
        .await
    }

    async fn close(&self, ids: Vec<u32>) -> Vec<u32> {
        use futures::{StreamExt, stream};
        stream::iter(ids.into_iter().map(|id| async move {
            let result: Result<(), _> = bounded(self.proxy.call("CloseNotification", &(id,))).await;
            result.err().map(|_| id)
        }))
        .buffer_unordered(8)
        .filter_map(async move |id| id)
        .collect()
        .await
    }
}

async fn bounded<T>(future: impl Future<Output = zbus::Result<T>>) -> Result<T, Error> {
    tokio::time::timeout(CALL_TIMEOUT, future)
        .await
        .map_err(|_| Error::Native("The notification service did not respond. Try again.".into()))?
        .map_err(|error| Error::Native(format!("Notification service error: {error}")))
}

enum Input {
    Command(Option<Command>),
    Action(Option<Message>),
    Closed(Option<Message>),
    Wake,
}

async fn worker(
    shared: Arc<Shared>,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::UnboundedSender<Event>,
) {
    let mut bus = None::<Bus>;
    loop {
        let (stopping, closing) = {
            let state = shared.registry.lock().expect("notification registry");
            (
                state.shutdown,
                state.pending_close.iter().copied().collect::<Vec<_>>(),
            )
        };
        let mut closing = closing;
        if !closing.is_empty() && bus.is_none() {
            match Bus::connect().await {
                Ok(connected) => {
                    let mut state = shared.registry.lock().expect("notification registry");
                    if state.owner.as_deref() != Some(&connected.owner) {
                        // Server-owned IDs are meaningless after its owner changes.
                        state.entries.retain(|_, entry| entry.server_id.is_none());
                        state.pending_close.clear();
                        closing.clear();
                        state.owner = Some(connected.owner.clone());
                    }
                    bus = Some(connected);
                }
                Err(error) => log::warn!("Could not reconnect to retract notifications: {error}"),
            }
        }
        if !closing.is_empty()
            && let Some(bus) = &bus
        {
            let retry = closing.clone();
            let failed = tokio::time::timeout(CALL_TIMEOUT, bus.close(closing.clone()))
                .await
                .unwrap_or(retry);
            let mut state = shared.registry.lock().expect("notification registry");
            for id in closing {
                if !failed.contains(&id) {
                    state.pending_close.remove(&id);
                }
            }
            if !failed.is_empty() {
                log::warn!("Could not retract {} system notifications", failed.len());
            }
        }
        if stopping {
            return;
        }
        let input = if let Some(bus) = &mut bus {
            tokio::select! {
                biased;
                _ = shared.wake.notified() => Input::Wake,
                command = commands.recv() => Input::Command(command),
                event = bus.actions.next() => Input::Action(event),
                event = bus.closed.next() => Input::Closed(event),
            }
        } else {
            tokio::select! {
                _ = shared.wake.notified() => Input::Wake,
                command = commands.recv() => Input::Command(command),
            }
        };
        match input {
            Input::Wake => {}
            Input::Action(Some(message)) => {
                if let Ok((server_id, action)) = message.body().deserialize::<(u32, String)>()
                    && action == "default"
                {
                    let state = shared.registry.lock().expect("notification registry");
                    if !state.shutdown
                        && let Some(entry) = state
                            .entries
                            .values()
                            .find(|entry| entry.server_id == Some(server_id))
                    {
                        let _ = events.send(Event::Clicked {
                            account_id: entry.account.clone(),
                            conversation_id: entry.conversation.clone(),
                            message_id: entry.message.clone(),
                        });
                    }
                }
            }
            Input::Closed(Some(message)) => {
                if let Ok((id, _reason)) = message.body().deserialize::<(u32, u32)>() {
                    let mut state = shared.registry.lock().expect("notification registry");
                    state.entries.retain(|_, entry| entry.server_id != Some(id));
                    state.pending_close.remove(&id);
                }
            }
            Input::Action(None) | Input::Closed(None) => bus = None,
            Input::Command(None) => {
                let mut state = shared.registry.lock().expect("notification registry");
                state.shutdown = true;
                let entries = std::mem::take(&mut state.entries);
                state
                    .pending_close
                    .extend(entries.into_values().filter_map(|entry| entry.server_id));
            }
            Input::Command(Some(command)) => {
                if bus.is_none() {
                    match Bus::connect().await {
                        Ok(connected) => {
                            let mut state = shared.registry.lock().expect("notification registry");
                            if state.owner.as_deref() != Some(&connected.owner) {
                                state.entries.retain(|_, entry| entry.server_id.is_none());
                                state.pending_close.clear();
                                state.owner = Some(connected.owner.clone());
                            }
                            bus = Some(connected);
                        }
                        Err(error) => {
                            match command {
                                Command::Permission(reply) => {
                                    let _ = reply.send(Err(error));
                                }
                                Command::Deliver(notification, generation, reply) => {
                                    remove_generation(&shared, &notification.id, generation);
                                    let _ = reply.send(Err(error));
                                }
                            }
                            continue;
                        }
                    }
                }
                if shared
                    .registry
                    .lock()
                    .expect("notification registry")
                    .shutdown
                {
                    match command {
                        Command::Permission(reply) => {
                            let _ = reply.send(Err(unavailable()));
                        }
                        Command::Deliver(_, _, reply) => {
                            let _ = reply.send(Err(unavailable()));
                        }
                    }
                    continue;
                }
                let connected = bus.as_ref().expect("connected notification service");
                match command {
                    Command::Permission(reply) => {
                        let checked: Result<Vec<String>, _> =
                            bounded(connected.proxy.call("GetCapabilities", &())).await;
                        match checked {
                            Ok(capabilities) => {
                                bus.as_mut()
                                    .expect("connected notification service")
                                    .capabilities = capabilities;
                                let _ = reply.send(Ok(Permission::NotRequired));
                            }
                            Err(error) => {
                                let _ = reply.send(Err(error));
                                bus = None;
                            }
                        }
                    }
                    Command::Deliver(notification, generation, reply) => {
                        let current = {
                            let mut state = shared.registry.lock().expect("notification registry");
                            let current = !state.shutdown
                                && state
                                    .entries
                                    .get(&notification.id)
                                    .is_some_and(|entry| entry.generation == generation);
                            if current {
                                state.inflight = Some((notification.id.clone(), generation));
                            }
                            current
                        };
                        if !current {
                            let _ = reply.send(Err(unavailable()));
                            continue;
                        }
                        let result = connected.deliver(&notification).await;
                        match result {
                            Ok(id) if id != 0 => {
                                let mut state =
                                    shared.registry.lock().expect("notification registry");
                                state.inflight = None;
                                let stopping = state.shutdown;
                                if let Some(entry) = state.entries.get_mut(&notification.id)
                                    && entry.generation == generation
                                    && !stopping
                                {
                                    entry.server_id = Some(id);
                                    let _ = reply.send(Ok(()));
                                } else {
                                    state.pending_close.insert(id);
                                    let _ = reply.send(Err(unavailable()));
                                }
                            }
                            Ok(_) => {
                                remove_generation(&shared, &notification.id, generation);
                                let _ = reply.send(Err(Error::Native(
                                    "The notification server returned an invalid ID".into(),
                                )));
                            }
                            Err(error) => {
                                remove_generation(&shared, &notification.id, generation);
                                let _ = reply.send(Err(error));
                                bus = None;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn remove_generation(shared: &Shared, id: &str, generation: u64) {
    let mut state = shared.registry.lock().expect("notification registry");
    if state
        .inflight
        .as_ref()
        .is_some_and(|(inflight, version)| inflight == id && *version == generation)
    {
        state.inflight = None;
    }
    if state
        .entries
        .get(id)
        .is_some_and(|entry| entry.generation == generation)
    {
        state.entries.remove(id);
    }
}
