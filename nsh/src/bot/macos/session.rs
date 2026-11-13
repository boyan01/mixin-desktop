//! Account-isolated WebKit data. Normal exit retains cookies; account exit purges them.
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak as RcWeak},
};

use super::resources::Resources;
use block2::RcBlock;
use nsh_platform_macos::{ComponentContext, NativeComponent, objc2_app_kit::NSView};
use objc2::{
    AnyThread, MainThreadMarker, MainThreadOnly, msg_send,
    rc::{Retained, Weak},
    runtime::{AnyObject, Bool},
    sel,
};
use objc2_foundation::{NSDate, NSRect, NSString, NSUUID};
use serde_json::Value;
use sha2::{Digest, Sha256};

struct Entry {
    store: Weak<AnyObject>,
    windows: Vec<RcWeak<RefCell<Resources>>>,
    purging: bool,
    themes: HashMap<String, [u8; 3]>,
}

thread_local! {
    static STORES: RefCell<HashMap<String, Entry>> = RefCell::new(HashMap::new());
    static PURGING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub(super) fn store(key: &str) -> Option<Retained<AnyObject>> {
    STORES.with(|stores| {
        stores
            .borrow()
            .get(key)
            .filter(|entry| !entry.purging)
            .and_then(|entry| entry.store.load())
    })
}

pub(super) fn register_window(key: &str, resources: &Rc<RefCell<Resources>>) {
    STORES.with(|stores| {
        if let Some(entry) = stores.borrow_mut().get_mut(key) {
            entry.windows.retain(|window| window.strong_count() > 0);
            entry.windows.push(Rc::downgrade(resources));
        }
    });
}

pub(super) fn theme(session: &str, app: &str) -> Option<[u8; 3]> {
    STORES.with(|stores| {
        stores
            .borrow()
            .get(session)
            .and_then(|entry| entry.themes.get(app).copied())
    })
}

pub(super) fn set_theme(session: &str, app: &str, color: Option<[u8; 3]>) {
    STORES.with(|stores| {
        if let Some(entry) = stores.borrow_mut().get_mut(session) {
            if let Some(color) = color {
                if entry.themes.len() >= 128 {
                    entry.themes.clear();
                }
                entry.themes.insert(app.to_owned(), color);
            } else {
                entry.themes.remove(app);
            }
        }
    });
}

fn identifier(key: &str) -> Result<Retained<NSUUID>, ()> {
    let account = key.split_once(':').map(|(account, _)| account).ok_or(())?;
    let directory = mixin_desktop_api::data_directory().map_err(|_| ())?;
    let canonical = std::fs::canonicalize(directory).map_err(|_| ())?;
    let mut digest = Sha256::new();
    digest.update(b"one.mixin.messenger.nsh.web-profile\0");
    digest.update(canonical.as_os_str().as_encoded_bytes());
    digest.update([0]);
    digest.update(account.as_bytes());
    let digest = digest.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // UUIDv8 identifies this application's SHA-256-derived profile namespace.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let text = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    NSUUID::initWithUUIDString(NSUUID::alloc(), &NSString::from_str(&text)).ok_or(())
}

pub(super) struct Session {
    holder: Retained<NSView>,
    context: ComponentContext,
    key: Option<String>,
    store: Option<Retained<AnyObject>>,
    purging: bool,
    profile: Option<String>,
}

impl Session {
    pub(super) fn new(context: ComponentContext) -> Self {
        let mtm = MainThreadMarker::new().expect("Native factories run on AppKit's main thread");
        Self {
            holder: NSView::initWithFrame(NSView::alloc(mtm), NSRect::ZERO),
            context,
            key: None,
            store: None,
            purging: false,
            profile: None,
        }
    }

    fn purge(&mut self) {
        if self.purging {
            return;
        }
        self.purging = true;
        if let Some(profile) = &self.profile {
            PURGING.with(|profiles| profiles.borrow_mut().insert(profile.clone()));
        }
        let windows = self
            .key
            .as_ref()
            .map(|key| {
                STORES.with(|stores| {
                    let mut stores = stores.borrow_mut();
                    stores
                        .get_mut(key)
                        .map(|entry| {
                            entry.purging = true;
                            std::mem::take(&mut entry.windows)
                        })
                        .unwrap_or_default()
                })
            })
            .unwrap_or_default();
        for window in windows.into_iter().filter_map(|window| window.upgrade()) {
            window.borrow_mut().close();
        }
        let Some(store) = &self.store else {
            self.context.emit("purged", Value::Null);
            return;
        };
        let Ok(class) = super::class(c"WKWebsiteDataStore") else {
            self.context.emit("purged", Value::Null);
            return;
        };
        let types: Retained<AnyObject> = unsafe { msg_send![class, allWebsiteDataTypes] };
        let context = self.context.clone();
        let profile = self.profile.clone();
        let done = RcBlock::new(move || {
            if let Some(profile) = &profile {
                PURGING.with(|profiles| profiles.borrow_mut().remove(profile));
            }
            context.emit("purged", Value::Null);
        });
        // All views were synchronously detached and released above. WebKit's
        // completion is the boundary before another account session may start.
        unsafe {
            let _: () = msg_send![store, removeDataOfTypes: &*types, modifiedSince: &*NSDate::dateWithTimeIntervalSince1970(0.0), completionHandler: &*done];
        }
    }
}

impl NativeComponent for Session {
    fn view(&self) -> Retained<NSView> {
        self.holder.clone()
    }
    fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        if name != "session" || self.store.is_some() || self.purging {
            return;
        }
        let Some(key) = value.and_then(Value::as_str).filter(|key| !key.is_empty()) else {
            return;
        };
        let Ok(class) = super::class(c"WKWebsiteDataStore") else {
            self.context.emit("failed", Value::Null);
            return;
        };
        let persistent: Bool =
            unsafe { msg_send![class, respondsToSelector: sel!(dataStoreForIdentifier:)] };
        let store: Retained<AnyObject> = if persistent.as_bool() {
            let Ok(identifier) = identifier(key) else {
                self.context.emit("failed", Value::Null);
                return;
            };
            let profile = identifier.UUIDString().to_string();
            if PURGING.with(|profiles| profiles.borrow().contains(&profile)) {
                self.context.emit("failed", Value::Null);
                return;
            }
            self.profile = Some(profile);
            unsafe { msg_send![class, dataStoreForIdentifier: &*identifier] }
        } else {
            // macOS before 14 has no public named persistent profiles. Preserve
            // account isolation with an explicit session-only downgrade.
            self.context.emit("session-only", Value::Null);
            unsafe { msg_send![class, nonPersistentDataStore] }
        };
        STORES.with(|stores| {
            stores.borrow_mut().insert(
                key.to_owned(),
                Entry {
                    store: Weak::new(&store),
                    windows: Vec::new(),
                    purging: false,
                    themes: HashMap::new(),
                },
            );
        });
        self.key = Some(key.to_owned());
        self.store = Some(store);
    }
    fn command(&mut self, name: &str, _args: &Value) {
        if name == "purge" {
            self.purge();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            let windows = STORES.with(|stores| {
                stores
                    .borrow_mut()
                    .remove(&key)
                    .map(|entry| entry.windows)
                    .unwrap_or_default()
            });
            for window in windows.into_iter().filter_map(|window| window.upgrade()) {
                window.borrow_mut().close();
            }
        }
        self.store.take();
    }
}
