use mixin_desktop_api::{DesktopClient, SettingsClient};

use super::*;
use crate::runtime;

const SETTING: &str = "desktop_ui_sidebar_collapsed";

#[derive(Clone, Copy)]
pub(super) struct Preference {
    settings: Stored<Option<SettingsClient>>,
    nav: Navigation,
    loading: Signal<bool>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    revision: Signal<u64>,
}

impl Preference {
    pub(super) fn new(nav: Navigation) -> Self {
        let state = Self {
            settings: stored(None),
            nav,
            loading: signal(true),
            busy: signal(false),
            failed: signal(false),
            revision: signal(0),
        };
        effect(move || {
            let _revision = state.revision.get();
            state.loading.set(true);
            state.failed.set(false);
            spawn(async move {
                let settings =
                    runtime::run(async { Ok(DesktopClient::open().await?.settings()) }).await;
                match settings {
                    Ok(settings) => {
                        state.settings.replace(Some(settings.clone()));
                        let mut changes =
                            Box::pin(runtime::observe(settings.subscribe_setting(SETTING.into())));
                        while let Some(value) = changes.next().await {
                            match value {
                                Ok(value) => {
                                    state.nav.collapsed.set(value.as_deref() == Some("true"));
                                    state.failed.set(false);
                                }
                                Err(_) => state.failed.set(true),
                            }
                            state.loading.set(false);
                        }
                    }
                    Err(_) => {
                        state.loading.set(false);
                        state.failed.set(true);
                    }
                }
            });
        });
        state
    }

    pub(super) fn disabled(self) -> bool {
        self.loading.get() || self.busy.get() || self.settings.with(|settings| settings.is_none())
    }

    pub(super) fn toggle(self) {
        if self.disabled() {
            return;
        }
        let Some(settings) = self.settings.with(|settings| settings.clone()) else {
            return;
        };
        let previous = self.nav.collapsed.get_untracked();
        self.nav.collapsed.set(!previous);
        self.busy.set(true);
        self.failed.set(false);
        let command = runtime::commit(async move {
            settings
                .set_setting(SETTING.into(), Some((!previous).to_string()))
                .await
        });
        spawn(async move {
            if command.await.is_err() {
                self.nav.collapsed.set(previous);
                self.failed.set(true);
            }
            self.busy.set(false);
        });
    }

    pub(super) fn feedback(self) -> impl IntoElement {
        when(self.failed, move || {
            view()
                .px_2()
                .py_2()
                .gap_1()
                .child(
                    text(move || {
                        crate::i18n::tr("Couldn’t save sidebar settings", "侧栏设置保存失败")
                    })
                    .text_xs()
                    .text_color(move || theme().destructive),
                )
                .child(ui::action(
                    move || i18n::current().text(Key::Retry),
                    move || self.revision.update(|revision| *revision += 1),
                ))
        })
    }
}
