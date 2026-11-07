//! Persistent account profile ownership; purge completion comes from WebView2.
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::{Rc, Weak},
};

use nsh_platform_windows::{ComponentContext, NativeComponent};
use serde_json::Value;
use sha2::{Digest, Sha256};
use webview2_com::{
    ClearBrowsingDataCompletedHandler, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, Microsoft::Web::WebView2::Win32::*,
};
use windows::core::{HSTRING, Interface};

use super::{dispatch, window::State};

type Waiting = Box<dyn FnOnce(Option<ICoreWebView2Environment>)>;

thread_local! {
    static SESSIONS: RefCell<HashMap<String, Weak<Entry>>> = RefCell::default();
    static PURGING: RefCell<HashMap<PathBuf, String>> = RefCell::default();
}

pub(super) struct Entry {
    pub accepting: Cell<bool>,
    context: ComponentContext,
    anchor: Rc<dispatch::Window>,
    environment: RefCell<Option<ICoreWebView2Environment>>,
    controller: RefCell<Option<ICoreWebView2Controller>>,
    profile: RefCell<Option<ICoreWebView2Profile2>>,
    waiting: RefCell<Vec<Waiting>>,
    windows: RefCell<Vec<Weak<State>>>,
    alive: Cell<bool>,
    clearing: Cell<bool>,
    purge_requested: Cell<bool>,
    initializing: Cell<bool>,
    operation: Cell<u64>,
    folder: PathBuf,
    key: String,
}

impl Entry {
    fn start(key: &str, context: ComponentContext) -> Result<Rc<Self>, &'static str> {
        let folder = profile_folder(key)?;
        if PURGING.with(|profiles| profiles.borrow().contains_key(&folder)) {
            return Err("Browser profile cleanup is still pending");
        }
        let anchor = dispatch::Window::new("Mixin browser session", false)
            .map_err(|_| "Could not create browser session")?;
        let entry = Rc::new(Self {
            accepting: Cell::new(true),
            context,
            anchor,
            environment: RefCell::default(),
            controller: RefCell::default(),
            profile: RefCell::default(),
            waiting: RefCell::default(),
            windows: RefCell::default(),
            alive: Cell::new(true),
            clearing: Cell::new(false),
            purge_requested: Cell::new(false),
            initializing: Cell::new(false),
            operation: Cell::new(0),
            folder,
            key: key.to_owned(),
        });
        entry.initialize();
        Ok(entry)
    }

    fn initialize(self: &Rc<Self>) {
        self.release_browser();
        self.initializing.set(true);
        let operation = self.operation.get().wrapping_add(1);
        self.operation.set(operation);
        let weak = Rc::downgrade(self);
        self.anchor.timeout(move || {
            if let Some(entry) = weak.upgrade().filter(|entry| entry.current(operation)) {
                entry.fail();
            }
        });
        let weak = Rc::downgrade(self);
        let done = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |result, environment| {
                let Some(entry) = weak.upgrade().filter(|entry| entry.current(operation)) else {
                    return Ok(());
                };
                let Some(environment) = result.ok().and(environment) else {
                    entry.fail();
                    return Ok(());
                };
                *entry.environment.borrow_mut() = Some(environment.clone());
                let weak = Rc::downgrade(&entry);
                let ready = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                    move |result, controller| {
                        let Some(entry) = weak.upgrade().filter(|entry| entry.current(operation))
                        else {
                            if let Some(controller) = controller {
                                unsafe {
                                    let _ = controller.Close();
                                }
                            }
                            return Ok(());
                        };
                        let Some(controller) = result.ok().and(controller) else {
                            entry.fail();
                            return Ok(());
                        };
                        // Keep this hidden controller alive throughout ClearBrowsingDataAll.
                        let initialized = (|| unsafe {
                            controller.SetIsVisible(false)?;
                            let web = controller.CoreWebView2()?;
                            web.cast::<ICoreWebView2_13>()?
                                .Profile()?
                                .cast::<ICoreWebView2Profile2>()
                        })();
                        match initialized {
                            Ok(profile) => {
                                *entry.profile.borrow_mut() = Some(profile);
                                *entry.controller.borrow_mut() = Some(controller);
                                entry.initializing.set(false);
                                entry.anchor.clear_timeout();
                                let environment = entry.environment.borrow().clone();
                                let waiting =
                                    entry.waiting.borrow_mut().drain(..).collect::<Vec<_>>();
                                for waiting in waiting {
                                    waiting(
                                        entry
                                            .accepting
                                            .get()
                                            .then(|| environment.clone())
                                            .flatten(),
                                    );
                                }
                                if entry.purge_requested.get() {
                                    entry.purge();
                                }
                            }
                            Err(_) => {
                                unsafe {
                                    let _ = controller.Close();
                                }
                                entry.fail();
                            }
                        }
                        Ok(())
                    },
                ));
                if unsafe {
                    environment.CreateCoreWebView2Controller(entry.anchor.hwnd.get(), &ready)
                }
                .is_err()
                {
                    entry.fail();
                }
                Ok(())
            },
        ));
        let folder = HSTRING::from(self.folder.to_string_lossy().as_ref());
        if unsafe {
            CreateCoreWebView2EnvironmentWithOptions(dispatch::null(), &folder, None, &done)
        }
        .is_err()
        {
            self.fail();
        }
    }

    fn current(&self, operation: u64) -> bool {
        self.alive.get() && self.operation.get() == operation
    }

    fn release_browser(&self) {
        self.profile.borrow_mut().take();
        let controller = self.controller.borrow_mut().take();
        if let Some(controller) = controller {
            unsafe {
                let _ = controller.Close();
            }
        }
        self.environment.borrow_mut().take();
    }

    fn fail(&self) {
        self.accepting.set(false);
        self.initializing.set(false);
        self.clearing.set(false);
        self.operation.set(self.operation.get().wrapping_add(1));
        self.anchor.clear_timeout();
        self.release_browser();
        let waiting = self.waiting.borrow_mut().drain(..).collect::<Vec<_>>();
        for waiting in waiting {
            waiting(None);
        }
        self.context.emit("failed", Value::Null);
    }

    pub fn environment(&self, waiting: impl FnOnce(Option<ICoreWebView2Environment>) + 'static) {
        if !self.alive.get() || !self.accepting.get() {
            waiting(None);
        } else if self.profile.borrow().is_some() {
            waiting(self.environment.borrow().clone());
        } else if self.waiting.borrow().len() < 32 {
            self.waiting.borrow_mut().push(Box::new(waiting));
        } else {
            waiting(None);
        }
    }

    pub fn register(&self, window: &Rc<State>) {
        let mut windows = self.windows.borrow_mut();
        windows.retain(|window| window.strong_count() != 0);
        windows.push(Rc::downgrade(window));
    }

    fn close_windows(&self) {
        let windows = self.windows.borrow_mut().drain(..).collect::<Vec<_>>();
        for window in windows.into_iter().filter_map(|window| window.upgrade()) {
            window.close();
        }
    }

    fn purge(self: &Rc<Self>) {
        self.purge_requested.set(true);
        self.accepting.set(false);
        self.close_windows();
        if !self.alive.get() {
            self.context.emit("failed", Value::Null);
            return;
        }
        if self.clearing.get() {
            return;
        }
        let owns_profile = PURGING.with(|profiles| {
            let mut profiles = profiles.borrow_mut();
            profiles
                .entry(self.folder.clone())
                .or_insert_with(|| self.key.clone())
                == &self.key
        });
        if !owns_profile {
            self.context.emit("failed", Value::Null);
            return;
        }
        let profile = self.profile.borrow().clone();
        let Some(profile) = profile else {
            // A failed creation has no pending callback. A retry must create a
            // real cleanup anchor instead of waiting forever for that callback.
            if !self.initializing.get() {
                self.initialize();
            }
            return;
        };
        self.clearing.set(true);
        let operation = self.operation.get().wrapping_add(1);
        self.operation.set(operation);
        let weak = Rc::downgrade(self);
        let done = ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
            let Some(entry) = weak.upgrade().filter(|entry| entry.current(operation)) else {
                return Ok(());
            };
            entry.clearing.set(false);
            entry.anchor.clear_timeout();
            if result.is_ok() {
                PURGING.with(|profiles| {
                    let mut profiles = profiles.borrow_mut();
                    if profiles.get(&entry.folder) == Some(&entry.key) {
                        profiles.remove(&entry.folder);
                    }
                });
                entry.context.emit("purged", Value::Null);
            } else {
                entry.fail();
            }
            Ok(())
        }));
        let weak = Rc::downgrade(self);
        self.anchor.timeout(move || {
            if let Some(entry) = weak.upgrade().filter(|entry| entry.current(operation)) {
                // Closing an unresponsive cleanup anchor invalidates its late
                // callback. Retry builds another anchor and clears the same
                // verified account profile; it never deletes a guessed folder.
                entry.fail();
            }
        });
        if unsafe { profile.ClearBrowsingDataAll(&done) }.is_err() {
            self.fail();
        }
    }

    fn close(&self) {
        self.alive.set(false);
        self.accepting.set(false);
        self.close_windows();
        let waiting = self.waiting.borrow_mut().drain(..).collect::<Vec<_>>();
        for waiting in waiting {
            waiting(None);
        }
        self.operation.set(self.operation.get().wrapping_add(1));
        self.release_browser();
        self.anchor.close();
    }
}

pub(super) fn get(key: &str) -> Option<Rc<Entry>> {
    let entry = SESSIONS
        .with(|sessions| sessions.borrow().get(key).and_then(Weak::upgrade))
        .filter(|entry| entry.alive.get() && !entry.purge_requested.get())?;
    if !entry.accepting.get() && !entry.initializing.get() {
        entry.accepting.set(true);
        entry.initialize();
    }
    Some(entry)
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
    let mut digest = Sha256::new();
    digest.update(b"one.mixin.messenger.nsh.web-profile\0");
    digest.update(root.as_os_str().as_encoded_bytes());
    digest.update([0]);
    digest.update(account.as_bytes());
    let digest = digest.finalize();
    let identifier: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let folder = root.join("WebViewProfiles").join(identifier);
    std::fs::create_dir_all(&folder).map_err(|_| "Could not create browser profile")?;
    let folder = std::fs::canonicalize(folder).map_err(|_| "Invalid browser profile directory")?;
    if !folder.starts_with(&root) {
        return Err("Browser profile is outside the application data directory");
    }
    Ok(folder)
}

pub(super) struct Session {
    context: ComponentContext,
    key: Option<String>,
    entry: Option<Rc<Entry>>,
}

impl Session {
    pub fn new(context: ComponentContext) -> Self {
        Self {
            context,
            key: None,
            entry: None,
        }
    }
}

impl NativeComponent for Session {
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        if name != "session" || self.key.is_some() {
            return;
        }
        let Some(key) = value.and_then(Value::as_str).filter(|key| !key.is_empty()) else {
            return;
        };
        self.key = Some(key.to_owned());
        match Entry::start(key, self.context.clone()) {
            Ok(entry) => {
                SESSIONS.with(|sessions| {
                    sessions
                        .borrow_mut()
                        .insert(key.to_owned(), Rc::downgrade(&entry));
                });
                self.entry = Some(entry);
            }
            Err(_) => self.context.emit("failed", Value::Null),
        }
    }
    fn command(&mut self, name: &str, _: &Value) {
        if name == "purge" {
            if self.entry.is_none()
                && let Some(key) = self.key.as_deref()
                && let Ok(entry) = Entry::start(key, self.context.clone())
            {
                SESSIONS.with(|sessions| {
                    sessions
                        .borrow_mut()
                        .insert(key.to_owned(), Rc::downgrade(&entry));
                });
                self.entry = Some(entry);
            }
            if let Some(entry) = &self.entry {
                entry.purge();
            } else {
                self.context.emit("failed", Value::Null);
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            SESSIONS.with(|sessions| {
                sessions.borrow_mut().remove(&key);
            });
        }
        if let Some(entry) = self.entry.take() {
            entry.close();
        }
    }
}
