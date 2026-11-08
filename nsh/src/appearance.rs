//! Session-owned presentation of the authoritative desktop chat preferences.
mod badges;
pub(crate) use badges::{delivery_icon, delivery_icon_on_media, identity_badge, status_icon};

use futures::StreamExt as _;
use mixin_desktop_api::SettingsClient;
use nsh_core::prelude::*;

use crate::runtime;

const AVATAR_KEY: &str = "desktop_ui_message_show_avatar";
const IDENTITY_KEY: &str = "desktop_ui_message_show_identity_number";
const FONT_KEY: &str = "desktop_ui_chat_font_size_delta";

#[derive(Clone, Copy)]
enum Field {
    Avatar,
    Identity,
    Font,
}
impl Field {
    fn index(self) -> usize {
        match self {
            Self::Avatar => 0,
            Self::Identity => 1,
            Self::Font => 2,
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Avatar => AVATAR_KEY,
            Self::Identity => IDENTITY_KEY,
            Self::Font => FONT_KEY,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Appearance {
    owner: Owner,
    /// `None` only in the debug preview, which keeps the defaults.
    client: Stored<Option<SettingsClient>>,
    avatar: Signal<bool>,
    identity: Signal<bool>,
    font: Signal<f64>,
    confirmed: Stored<(bool, bool, f64)>,
    loaded: Signal<[bool; 3]>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    previewing: Signal<bool>,
    pending_font: Stored<Option<f64>>,
    revision: Signal<u64>,
}

pub(crate) fn mount() -> Appearance {
    let state = Appearance::new(Some(crate::settings_client()));
    effect(move || {
        state.revision.get();
        state.loaded.set([false; 3]);
        state.failed.set(false);
        for field in [Field::Avatar, Field::Identity, Field::Font] {
            let client = state
                .client
                .with(|client| client.clone())
                .expect("mounted with a client");
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(
                    client.subscribe_setting(field.key().into()),
                ));
                while let Some(next) = changes.next().await {
                    match next {
                        Ok(next) => {
                            if let Some(value) = parse(field, next.as_deref()) {
                                state.confirm(field, value);
                                if !state.busy.get_untracked()
                                    && !(matches!(field, Field::Font)
                                        && (state.previewing.get_untracked()
                                            || state.pending_font.get().is_some()))
                                {
                                    state.display(field, value);
                                }
                            } else {
                                // Keep the last confirmed display, but allow a user
                                // change to replace a malformed local preference.
                                state.failed.set(true);
                            }
                            state.loaded.update(|loaded| loaded[field.index()] = true);
                        }
                        Err(_) => state.failed.set(true),
                    }
                }
            });
        }
    });
    state
}

fn parse(field: Field, value: Option<&str>) -> Option<f64> {
    match field {
        Field::Avatar => value.map_or(Some(1.0), |value| {
            value
                .parse::<bool>()
                .ok()
                .map(|value| u8::from(value) as f64)
        }),
        Field::Identity => value.map_or(Some(0.0), |value| {
            value
                .parse::<bool>()
                .ok()
                .map(|value| u8::from(value) as f64)
        }),
        Field::Font => value.map_or(Some(0.0), |value| {
            value
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite() && (-2.0..=4.0).contains(value))
                .map(f64::round)
        }),
    }
}

impl Appearance {
    /// The default chat appearance, for the debug preview: nothing is read or saved.
    #[cfg(debug_assertions)]
    pub(crate) fn detached() -> Self {
        let state = Self::new(None);
        state.loaded.set([true; 3]);
        state
    }

    fn new(client: Option<SettingsClient>) -> Self {
        Self {
            owner: Owner::current(),
            client: stored(client),
            avatar: signal(true),
            identity: signal(false),
            font: signal(0.0),
            confirmed: stored((true, false, 0.0)),
            loaded: signal([false; 3]),
            busy: signal(false),
            failed: signal(false),
            previewing: signal(false),
            pending_font: stored(None),
            revision: signal(0),
        }
    }

    pub(crate) fn ready(self) -> bool {
        self.loaded
            .with(|loaded| loaded.iter().all(|loaded| *loaded))
    }
    pub(crate) fn busy(self) -> bool {
        self.busy.get()
    }
    pub(crate) fn failed(self) -> bool {
        self.failed.get()
    }
    pub(crate) fn retry(self) {
        self.revision.update(|revision| *revision += 1);
    }
    pub(crate) fn show_avatar(self) -> bool {
        self.avatar.get()
    }
    pub(crate) fn show_identity(self) -> bool {
        self.identity.get()
    }
    pub(crate) fn font_delta(self) -> f64 {
        self.font.get()
    }
    // Match Flutter's MessageStyle: text grows, status metadata stays at 10 pt.
    pub(crate) fn primary_size(self) -> Points {
        px((16.0 + self.font.get()) as f32)
    }
    pub(crate) fn secondary_size(self) -> Points {
        px((14.0 + self.font.get()) as f32)
    }
    pub(crate) fn tertiary_size(self) -> Points {
        px((12.0 + self.font.get()) as f32)
    }
    pub(crate) fn set_avatar(self, value: bool) {
        self.save(Field::Avatar, u8::from(value) as f64);
    }
    pub(crate) fn set_identity(self, value: bool) {
        self.save(Field::Identity, u8::from(value) as f64);
    }
    pub(crate) fn preview_font(self, value: f64) {
        if !self.ready() || !value.is_finite() {
            return;
        }
        self.previewing.set(true);
        self.font.set(value.round().clamp(-2.0, 4.0));
    }
    pub(crate) fn save_font(self, value: f64) {
        if !self.ready() || !value.is_finite() {
            return;
        }
        let value = value.round().clamp(-2.0, 4.0);
        self.previewing.set(false);
        self.font.set(value);
        // Keep only the latest completed adjustment while one setting is saving.
        self.pending_font.replace(Some(value));
        if !self.busy.get_untracked() {
            self.save(Field::Font, value);
        }
    }
    pub(crate) fn cancel_font_preview(self) {
        if self.previewing.get_untracked() {
            self.previewing.set(false);
            self.font.set(
                self.pending_font
                    .get()
                    .unwrap_or_else(|| self.confirmed.with(|value| value.2)),
            );
        }
    }
    fn display(self, field: Field, value: f64) {
        match field {
            Field::Avatar => self.avatar.set(value != 0.0),
            Field::Identity => self.identity.set(value != 0.0),
            Field::Font => self.font.set(value),
        }
    }
    fn confirm(self, field: Field, value: f64) {
        self.confirmed.with(|confirmed| match field {
            Field::Avatar => confirmed.0 = value != 0.0,
            Field::Identity => confirmed.1 = value != 0.0,
            Field::Font => confirmed.2 = value,
        });
    }
    fn confirmed_value(self, field: Field) -> f64 {
        self.confirmed.with(|confirmed| match field {
            Field::Avatar => u8::from(confirmed.0) as f64,
            Field::Identity => u8::from(confirmed.1) as f64,
            Field::Font => confirmed.2,
        })
    }
    fn save(self, field: Field, value: f64) {
        if !self.ready() || self.busy.get_untracked() {
            return;
        }
        let previous = self.confirmed_value(field);
        if !matches!(field, Field::Font) || !self.previewing.get_untracked() {
            self.display(field, value);
        }
        if previous == value {
            if matches!(field, Field::Font) {
                self.pending_font.replace(None);
            }
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        let client = self
            .client
            .with(|client| client.clone())
            .expect("the debug preview saves no settings");
        let encoded = if matches!(field, Field::Font) {
            value.to_string()
        } else {
            (value != 0.0).to_string()
        };
        let read_client = client.clone();
        let command =
            runtime::commit(
                async move { client.set_setting(field.key().into(), Some(encoded)).await },
            );
        self.owner.run(|| {
            spawn(async move {
                let saved = command.await;
                let current =
                    runtime::run(async move { read_client.setting(field.key().into()).await })
                        .await;
                if matches!(field, Field::Font) && self.pending_font.get() == Some(value) {
                    self.pending_font.replace(None);
                }
                let apply_current = !matches!(field, Field::Font)
                    || (!self.previewing.get_untracked() && self.pending_font.get().is_none());
                match current
                    .ok()
                    .and_then(|current| parse(field, current.as_deref()))
                {
                    Some(current) => {
                        self.confirm(field, current);
                        if apply_current {
                            self.display(field, current);
                        }
                    }
                    None => {
                        if apply_current {
                            self.display(field, self.confirmed_value(field));
                        }
                        self.failed.set(true);
                    }
                }
                if saved.is_err() {
                    self.failed.set(true);
                }
                batch(|| {
                    self.busy.set(false);
                    let confirmed = self.confirmed.with(|value| *value);
                    self.avatar.set(confirmed.0);
                    self.identity.set(confirmed.1);
                    if !self.previewing.get_untracked() && self.pending_font.get().is_none() {
                        self.font.set(confirmed.2);
                    }
                });
                if let Some(value) = self.pending_font.get() {
                    self.save(Field::Font, value);
                }
            })
        });
    }
}
