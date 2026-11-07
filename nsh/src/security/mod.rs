//! Session-owned presentation for Rust's authoritative screen privacy state.

use std::{future::Future, sync::Arc, time::Duration};

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ClientResult, ScreenLockClient, ScreenLockState};
use nsh_core::{platform::Lifecycle, prelude::*};

use crate::{i18n, runtime};

mod passcode;
mod settings;
mod surface;
pub(crate) use settings::settings;
pub(crate) use surface::surface;

#[derive(Clone, Copy)]
pub(crate) struct Handle {
    owner: Owner,
    client: Stored<ScreenLockClient>,
    state: Signal<Option<ScreenLockState>>,
    revision: Signal<u64>,
    load_failed: Signal<bool>,
    lock_pending: Signal<bool>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    cooldown_seconds: Signal<u64>,
}

pub(crate) fn mount(account: Arc<AccountClient>) -> Handle {
    let handle = Handle {
        owner: Owner::current(),
        client: stored(account.screen_lock()),
        state: signal(None),
        revision: signal(0),
        load_failed: signal(false),
        lock_pending: signal(false),
        busy: signal(false),
        failed: signal(false),
        cooldown_seconds: signal(0),
    };
    effect(move || {
        let _revision = handle.revision.get();
        let client = handle.client();
        handle.load_failed.set(false);
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(client.changes()));
            while let Some(next) = changes.next().await {
                match next {
                    Ok(state) => {
                        batch(|| {
                            handle.state.set(Some(state));
                            handle.load_failed.set(false);
                        });
                    }
                    Err(_) => {
                        handle.state.set(None);
                        handle.load_failed.set(true);
                    }
                }
            }
        });
    });
    effect(move || {
        let inactive = lifecycle() != Lifecycle::Active;
        let state = handle.state.get();
        let Some(state) = state.filter(|state| {
            state.enabled && !state.locked && state.auto_lock_minutes > 0 && inactive
        }) else {
            return;
        };
        spawn(async move {
            timer(Duration::from_secs(u64::from(state.auto_lock_minutes) * 60)).await;
            if lifecycle() != Lifecycle::Active {
                handle.lock();
            }
        });
    });
    effect(move || {
        let millis = handle
            .state
            .with(|state| state.as_ref().map_or(0, |state| state.retry_after_millis));
        handle.cooldown_seconds.set(millis.div_ceil(1000));
        if millis == 0 {
            return;
        }
        spawn(async move {
            let mut remaining = millis;
            while remaining > 0 {
                let step = remaining.min(1000);
                timer(Duration::from_millis(step)).await;
                remaining -= step;
                handle.cooldown_seconds.set(remaining.div_ceil(1000));
            }
        });
    });
    on_key("mod+shift+l", move |_| handle.lock());
    handle
}

impl Handle {
    pub(crate) fn blocked(self) -> bool {
        self.lock_pending.get()
            || self
                .state
                .with(|state| state.as_ref().is_none_or(|state| state.locked))
    }
    pub(crate) fn enabled(self) -> bool {
        self.state
            .with(|state| state.as_ref().is_some_and(|state| state.enabled))
    }
    pub(crate) fn lock(self) {
        if !self.enabled() || self.lock_pending.get_untracked() {
            return;
        }
        self.lock_pending.set(true);
        self.submit_lock();
    }
    fn submit_lock(self) {
        let client = self.client();
        self.load_failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move { client.lock().await }).await;
                let client = self.client();
                let state = runtime::run(async move { client.state().await }).await;
                match (result, state) {
                    (Ok(()), Ok(state)) => batch(|| {
                        self.state.set(Some(state));
                        self.lock_pending.set(false);
                        self.load_failed.set(false);
                    }),
                    _ => self.load_failed.set(true),
                }
            })
        });
    }
    fn client(self) -> ScreenLockClient {
        self.client.with(|client| client.clone())
    }
    fn retry(self) {
        if self.lock_pending.get_untracked() {
            self.submit_lock();
        } else {
            self.revision.update(|revision| *revision += 1);
        }
    }
    fn command(self, future: impl Future<Output = ClientResult<()>> + Send + 'static) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::commit(future).await;
                self.finish(result).await;
            })
        });
    }
    fn unlock(self, passcode: String) {
        if self.cooldown_seconds.get_untracked() > 0 {
            return;
        }
        let client = self.client();
        self.command(async move { client.unlock(passcode).await });
    }
    fn authenticate(self) {
        if self.busy.get_untracked() {
            return;
        }
        let client = self.client();
        let i18n = i18n::current();
        let reason = tr(i18n, "Unlock Mixin Messenger", "解锁 Mixin Messenger").to_owned();
        self.busy.set(true);
        self.failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                // Cancellation invalidates the native authentication context so
                // quitting cannot wait forever for an abandoned system prompt.
                let result =
                    runtime::run(async move { client.unlock_with_biometrics(reason).await }).await;
                self.finish(result).await;
            })
        });
    }
    async fn finish(self, result: ClientResult<()>) {
        let client = self.client();
        match runtime::run(async move { client.state().await }).await {
            Ok(state) => self.state.set(Some(state)),
            Err(_) => {
                self.state.set(None);
                self.load_failed.set(true);
            }
        }
        self.failed.set(result.is_err());
        self.busy.set(false);
    }
}

fn tr(i18n: i18n::I18n, en: &'static str, zh: &'static str) -> &'static str {
    i18n.tr(en, zh)
}
