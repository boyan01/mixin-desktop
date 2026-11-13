//! Root-owned native delivery with account-scoped subscriptions and routing.

use std::collections::VecDeque;

use futures::StreamExt as _;
use mixin_desktop_api::DesktopClient;
use nsh_core::{platform::Lifecycle, prelude::*};

pub(crate) mod native;

use native::{Availability, NativeNotifications, Permission};

use crate::{runtime, shell::NotificationTarget};

mod session;
pub(crate) use session::attach;

const PREVIEW_SETTING: &str = "desktop_notification_preview";
const MAX_PENDING_CLICKS: usize = 16;

#[derive(Clone, Copy)]
pub(crate) struct Handle {
    owner: Owner,
    native: Stored<Option<NativeNotifications>>,
    clicks: Signal<VecDeque<NotificationTarget>>,
    pub(crate) availability: Availability,
    pub(crate) permission: Signal<Option<Permission>>,
    pub(crate) busy: Signal<bool>,
    pub(crate) failed: Signal<bool>,
    preview_retry: Signal<u64>,
    pub(crate) preview: Signal<bool>,
    pub(crate) preview_ready: Signal<bool>,
    pub(crate) preview_busy: Signal<bool>,
    pub(crate) preview_failed: Signal<bool>,
}

pub(crate) fn mount() -> Handle {
    let availability = native::availability();
    let initialized = NativeNotifications::new();
    let failed = initialized.is_err() && availability == Availability::Available;
    let (native, events) = match initialized {
        Ok((native, events)) => (Some(native), Some(events)),
        Err(error) => {
            log::info!("Native notification availability: {error}");
            (None, None)
        }
    };
    let handle = Handle {
        owner: Owner::current(),
        native: stored(native),
        clicks: signal(VecDeque::new()),
        availability,
        permission: signal(None),
        busy: signal(false),
        failed: signal(failed),
        preview_retry: signal(0_u64),
        preview: signal(false),
        preview_ready: signal(false),
        preview_busy: signal(false),
        preview_failed: signal(false),
    };
    if let Some(mut events) = events {
        spawn(async move {
            while let Some(native::Event::Clicked {
                account_id,
                conversation_id,
                message_id,
            }) = events.recv().await
            {
                handle.enqueue_notification(NotificationTarget {
                    account_id,
                    conversation_id,
                    message_id,
                });
            }
        });
    }
    let shell = use_context::<crate::shell::Handle>().expect("App provides native shell");
    effect(move || {
        let targets = shell.notification_targets.get();
        if targets.is_empty() {
            return;
        }
        shell.notification_targets.set(VecDeque::new());
        for target in targets {
            handle.enqueue_notification(target);
        }
    });
    handle.refresh_permission();
    effect(move || {
        let _revision = handle.preview_retry.get();
        if mixin_desktop_api::data_directory().is_err() {
            handle.preview_failed.set(true);
            return;
        }
        handle.preview_failed.set(false);
        spawn(async move {
            let settings =
                match runtime::run(async { Ok(DesktopClient::open().await?.settings()) }).await {
                    Ok(settings) => settings,
                    Err(_) => {
                        handle.preview_failed.set(true);
                        return;
                    }
                };
            let mut changes = Box::pin(runtime::observe(
                settings.subscribe_setting(PREVIEW_SETTING.into()),
            ));
            while let Some(next) = changes.next().await {
                match next {
                    Ok(value) => {
                        handle.preview.set(value.as_deref() != Some("false"));
                        handle.preview_ready.set(true);
                        handle.preview_failed.set(false);
                    }
                    Err(_) => {
                        handle.preview.set(false);
                        handle.preview_ready.set(false);
                        handle.preview_failed.set(true);
                    }
                }
            }
        });
    });
    // Re-read permissions after returning from the system's settings window.
    effect(move || {
        if lifecycle() == Lifecycle::Active {
            handle.refresh_permission();
        }
    });
    handle
}

impl Handle {
    fn enqueue_notification(self, target: NotificationTarget) {
        self.clicks.update(|clicks| {
            if clicks.len() == MAX_PENDING_CLICKS {
                clicks.pop_front();
            }
            clicks.push_back(target);
        });
    }

    pub(crate) fn refresh_permission(self) {
        if self.busy.get_untracked() {
            return;
        }
        let completion = self
            .native
            .with(|native| native.as_ref().map(NativeNotifications::permission));
        let Some(completion) = completion else {
            return;
        };
        self.busy.set(true);
        self.owner.run(|| {
            spawn(async move {
                match completion.await {
                    Ok(permission) => {
                        self.permission.set(Some(permission));
                        self.failed.set(false);
                    }
                    Err(error) => {
                        log::warn!("Reading notification permission failed: {error}");
                        self.failed.set(true);
                    }
                }
                self.busy.set(false);
            })
        });
    }

    pub(crate) fn request_permission(self) {
        if self.busy.get_untracked() {
            return;
        }
        let completion = self
            .native
            .with(|native| native.as_ref().map(NativeNotifications::request_permission));
        let Some(completion) = completion else {
            return;
        };
        self.busy.set(true);
        self.failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                match completion.await {
                    Ok(permission) => {
                        self.permission.set(Some(permission));
                        self.failed.set(false);
                    }
                    Err(error) => {
                        log::warn!("Requesting notification permission failed: {error}");
                        self.failed.set(true);
                    }
                }
                self.busy.set(false);
            })
        });
    }

    pub(crate) fn refresh_preview(self) {
        self.preview_retry.update(|revision| *revision += 1);
    }

    pub(crate) fn set_preview(self, preview: bool) {
        if !self.preview_ready.get_untracked() || self.preview_busy.get_untracked() {
            return;
        }
        let previous = self.preview.get_untracked();
        self.preview.set(preview);
        self.preview_busy.set(true);
        self.preview_failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move {
                    DesktopClient::open()
                        .await?
                        .settings()
                        .set_setting(PREVIEW_SETTING.into(), Some(preview.to_string()))
                        .await
                })
                .await;
                if result.is_err() {
                    self.preview.set(previous);
                    self.preview_failed.set(true);
                }
                self.preview_busy.set(false);
            })
        });
    }

    fn dismiss(self, ids: &[String]) {
        self.native.with(|native| {
            if let Some(native) = native {
                native.dismiss(ids);
            }
        });
    }

    fn clear_account(self, account_id: String) {
        self.clicks
            .update(|clicks| clicks.retain(|click| click.account_id != account_id));
        let completion = self.native.with(|native| {
            native
                .as_ref()
                .map(|native| native.existing_ids(account_id))
        });
        if let Some(completion) = completion {
            self.owner.run(|| {
                spawn(async move {
                    match completion.await {
                        Ok(ids) => self.dismiss(&ids),
                        Err(error) => log::warn!("Clearing account notifications failed: {error}"),
                    }
                })
            });
        }
    }
}

#[cfg(debug_assertions)]
pub(crate) fn detached(failed: Signal<bool>) -> Handle {
    Handle {
        owner: Owner::current(),
        native: stored(None),
        clicks: signal(VecDeque::new()),
        availability: Availability::Available,
        permission: signal(Some(Permission::Authorized)),
        busy: signal(false),
        failed: signal(false),
        preview_retry: signal(0),
        preview: signal(true),
        preview_ready: signal(true),
        preview_busy: signal(false),
        preview_failed: failed,
    }
}
