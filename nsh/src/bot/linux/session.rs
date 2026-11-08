//! The account anchor outlives locked windows and owns real website-data cleanup.
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::{Rc, Weak},
    time::Duration,
};

use gtk::{gio, glib, prelude::*};
use nsh_platform_linux::{ComponentContext, NativeComponent, gtk4 as gtk};
use serde_json::Value;
use sha2::{Digest, Sha256};
use webkit6::{
    CookiePersistentStorage, NetworkSession, WebContext, WebsiteDataTypes, prelude::WebViewExt,
};

use super::window::State;

thread_local! {
    static SESSIONS: RefCell<HashMap<String, Weak<Entry>>> = RefCell::default();
    static PURGING: RefCell<HashMap<PathBuf, String>> = RefCell::default();
}

pub(super) struct Entry {
    pub network: NetworkSession,
    pub web_context: WebContext,
    pub accepting: Cell<bool>,
    pub extension_ready: bool,
    context: ComponentContext,
    windows: RefCell<Vec<Weak<State>>>,
    purge: RefCell<Option<glib::JoinHandle<()>>>,
    cancellation: RefCell<Option<gio::Cancellable>>,
    operation: Cell<u64>,
    folder: PathBuf,
    key: String,
}

impl Entry {
    fn new(key: &str, context: ComponentContext) -> Result<Rc<Self>, &'static str> {
        let folder = profile_folder(key)?;
        if PURGING.with(|profiles| profiles.borrow().contains_key(&folder)) {
            return Err("Browser profile cleanup is still pending");
        }
        let extension = extension_directory();
        let cache = folder.join("cache");
        std::fs::create_dir_all(&cache).map_err(|_| "Could not create browser cache")?;
        let folder_text = folder.to_str().ok_or("Invalid browser profile path")?;
        let cache_text = cache.to_str().ok_or("Invalid browser cache path")?;
        let network = NetworkSession::new(Some(folder_text), Some(cache_text));
        let cookies = network
            .cookie_manager()
            .ok_or("Browser cookies are unavailable")?;
        cookies.set_persistent_storage(
            &folder.join("cookies.sqlite").to_string_lossy(),
            CookiePersistentStorage::Sqlite,
        );
        let web_context = WebContext::new();
        if let Ok(extension) = &extension {
            web_context.set_web_process_extensions_directory(&extension.to_string_lossy());
        }
        web_context.set_web_process_extensions_initialization_user_data(&1_u32.to_variant());
        let entry = Rc::new(Self {
            network,
            web_context,
            accepting: Cell::new(true),
            extension_ready: extension.is_ok(),
            context,
            windows: RefCell::default(),
            purge: RefCell::default(),
            cancellation: RefCell::default(),
            operation: Cell::new(0),
            folder,
            key: key.to_owned(),
        });
        let weak = Rc::downgrade(&entry);
        entry.network.connect_download_started(move |_, download| {
            let Some(entry) = weak.upgrade().filter(|entry| entry.accepting.get()) else {
                download.cancel();
                return;
            };
            let view = download.web_view();
            let owner = entry
                .windows
                .borrow()
                .iter()
                .filter_map(Weak::upgrade)
                .find(|state| !state.closed.get() && view.as_ref() == Some(&state.web));
            if let Some(owner) = owner {
                owner.downloads.start(&owner, download);
            } else {
                download.cancel();
            }
        });
        Ok(entry)
    }

    pub fn register(&self, state: &Rc<State>) {
        let mut windows = self.windows.borrow_mut();
        windows.retain(|window| window.strong_count() != 0);
        windows.push(Rc::downgrade(state));
    }

    fn close_windows(&self) {
        let windows = self
            .windows
            .borrow_mut()
            .drain(..)
            .filter_map(|window| window.upgrade())
            .collect::<Vec<_>>();
        for window in &windows {
            window.close();
        }
        // An NSH host may retain its closed State until the next unmount commit.
        // Stop every account page before clearing cookies, even during that gap.
        // Ordinary single-window close does not terminate sibling pages.
        for window in windows {
            window.web.terminate_web_process();
        }
    }

    fn dispose(&self) {
        self.accepting.set(false);
        self.operation.set(self.operation.get().wrapping_add(1));
        self.close_windows();
        if let Some(cancellation) = self.cancellation.borrow_mut().take() {
            cancellation.cancel();
        }
        if let Some(task) = self.purge.borrow_mut().take() {
            task.abort();
        }
    }

    fn purge(self: &Rc<Self>) {
        if self.purge.borrow().is_some() {
            return;
        }
        self.accepting.set(false);
        self.close_windows();
        let owns_profile = PURGING.with(|profiles| {
            profiles
                .borrow_mut()
                .entry(self.folder.clone())
                .or_insert_with(|| self.key.clone())
                == &self.key
        });
        if !owns_profile {
            self.context.emit("failed", Value::Null);
            return;
        }
        let Some(manager) = self.network.website_data_manager() else {
            self.context.emit("failed", Value::Null);
            return;
        };
        let operation = self.operation.get().wrapping_add(1);
        self.operation.set(operation);
        let cancellation = gio::Cancellable::new();
        *self.cancellation.borrow_mut() = Some(cancellation.clone());
        let (send, receive) = tokio::sync::oneshot::channel();
        manager.clear(
            WebsiteDataTypes::ALL,
            glib::TimeSpan::from_seconds(0),
            Some(&cancellation),
            move |result| {
                let _ = send.send(result);
            },
        );
        let weak = Rc::downgrade(self);
        let task = glib::MainContext::default().spawn_local(async move {
            let result = futures::future::select(
                Box::pin(receive),
                Box::pin(glib::timeout_future(Duration::from_secs(15))),
            )
            .await;
            let Some(entry) = weak
                .upgrade()
                .filter(|entry| entry.operation.get() == operation)
            else {
                return;
            };
            entry.purge.borrow_mut().take();
            entry.cancellation.borrow_mut().take();
            match result {
                futures::future::Either::Left((Ok(Ok(())), _)) => {
                    PURGING.with(|profiles| {
                        let mut profiles = profiles.borrow_mut();
                        if profiles.get(&entry.folder) == Some(&entry.key) {
                            profiles.remove(&entry.folder);
                        }
                    });
                    entry.context.emit("purged", Value::Null)
                }
                _ => {
                    cancellation.cancel();
                    entry.context.emit("failed", Value::Null);
                }
            }
        });
        *self.purge.borrow_mut() = Some(task);
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.dispose();
    }
}

pub(super) fn get(key: &str) -> Option<Rc<Entry>> {
    SESSIONS
        .with(|sessions| sessions.borrow().get(key).and_then(Weak::upgrade))
        .filter(|entry| {
            entry.accepting.get()
                && !PURGING.with(|profiles| profiles.borrow().contains_key(&entry.folder))
        })
}

fn profile_folder(key: &str) -> Result<PathBuf, &'static str> {
    let account = key
        .split_once(':')
        .map(|(account, _)| account)
        .filter(|account| !account.is_empty())
        .ok_or("Missing browser account")?;
    let root =
        std::env::var_os("MIXIN_DESKTOP_DATA_DIR").ok_or("Missing application data directory")?;
    let root = std::fs::canonicalize(root).map_err(|_| "Invalid application data directory")?;
    let mut hash = Sha256::new();
    hash.update(b"one.mixin.messenger.nsh.web-profile\0");
    hash.update(root.as_os_str().as_encoded_bytes());
    hash.update([0]);
    hash.update(account.as_bytes());
    let name: String = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let folder = root.join("WebViewProfiles").join(name);
    std::fs::create_dir_all(&folder).map_err(|_| "Could not create browser profile")?;
    let folder = std::fs::canonicalize(folder).map_err(|_| "Invalid browser profile directory")?;
    if !folder.starts_with(root) {
        return Err("Browser profile is outside the application data directory");
    }
    Ok(folder)
}

fn extension_directory() -> Result<PathBuf, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "Could not locate browser extension")?;
    let directory = executable
        .parent()
        .ok_or("Could not locate browser extension")?
        .join("mixin-webkit-extension");
    let directory =
        std::fs::canonicalize(directory).map_err(|_| "The browser extension is not installed")?;
    if !directory.join("libmixin-webkit-extension.so").is_file() {
        return Err("The browser extension is not installed");
    }
    Ok(directory)
}

pub(super) struct Session {
    holder: gtk::Box,
    context: ComponentContext,
    key: Option<String>,
    entry: Option<Rc<Entry>>,
}

impl Session {
    pub fn new(context: ComponentContext) -> Self {
        Self {
            holder: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            context,
            key: None,
            entry: None,
        }
    }
    fn dispose(&mut self) {
        if let Some(entry) = self.entry.take() {
            if let Some(key) = &self.key {
                SESSIONS.with(|sessions| {
                    let mut sessions = sessions.borrow_mut();
                    if sessions
                        .get(key)
                        .is_some_and(|current| current.ptr_eq(&Rc::downgrade(&entry)))
                    {
                        sessions.remove(key);
                    }
                });
            }
            entry.dispose();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl NativeComponent for Session {
    fn widget(&self) -> gtk::Widget {
        self.holder.clone().upcast()
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        if name != "session" {
            return;
        }
        let Some(key) = value.and_then(Value::as_str).filter(|key| !key.is_empty()) else {
            return;
        };
        if self.key.as_deref() == Some(key) {
            return;
        }
        self.dispose();
        self.key = Some(key.into());
        match Entry::new(key, self.context.clone()) {
            Ok(entry) => {
                SESSIONS.with(|sessions| {
                    sessions
                        .borrow_mut()
                        .insert(key.into(), Rc::downgrade(&entry));
                });
                self.entry = Some(entry);
            }
            Err(_) => self.context.emit("failed", Value::Null),
        }
    }
    fn command(&mut self, name: &str, _: &Value) {
        if name != "purge" {
            return;
        }
        if self.entry.is_none()
            && let Some(key) = &self.key
        {
            match Entry::new(key, self.context.clone()) {
                Ok(entry) => self.entry = Some(entry),
                Err(_) => {
                    self.context.emit("failed", Value::Null);
                    return;
                }
            }
        }
        if let Some(entry) = &self.entry {
            entry.purge();
        } else {
            self.context.emit("failed", Value::Null);
        }
    }
}
