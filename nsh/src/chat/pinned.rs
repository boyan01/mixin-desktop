//! The latest pinned-message preview and its conversation-scoped dismissal.

use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, PinMessagePreviewItem, SettingsClient};
use nsh_core::prelude::*;
use serde_json::Value;

use crate::{
    i18n::{self, Key},
    navigation, presentation, runtime,
    ui::{self, Typography},
};

/// The latest pin of a conversation: loaded once per chat, shown by [`Pinned::view`]
/// wherever the chat's chrome has room for it.
#[derive(Clone, Copy)]
pub(super) struct Pinned {
    i18n: i18n::I18n,
    owner: Owner,
    preview: Signal<Option<(PinMessagePreviewItem, Option<String>)>>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    preference_failed: Signal<bool>,
    retry: Signal<u64>,
    setting_key: Stored<String>,
    /// None in the debug preview, where dismissal does nothing.
    settings: Stored<Option<SettingsClient>>,
    content: Memo<Option<String>>,
    presence: crate::motion::Presence,
    target: Signal<Option<String>>,
    side: Signal<Option<navigation::Panel>>,
}

impl Pinned {
    pub(super) fn mount(
        account: Arc<AccountClient>,
        conversation_id: String,
        target: Signal<Option<String>>,
        side: Signal<Option<navigation::Panel>>,
    ) -> Self {
        let i18n = i18n::current();
        let owner = Owner::current();
        let preview = signal_with(
            None::<(PinMessagePreviewItem, Option<String>)>,
            SignalEquals::Never,
        );
        let shown = signal(false);
        let busy = signal(false);
        let failed = signal(false);
        let preference_failed = signal(false);
        let retry = signal(0_u64);
        let settings = crate::settings_client();
        let setting_key = stored(SettingsClient::pinned_preview_key(
            &account.account_id(),
            &conversation_id,
        ));
        let preferences = settings.clone();
        effect(move || {
            let request = retry.get();
            let settings = preferences.clone();
            let key = setting_key.with(|key| key.clone());
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(settings.subscribe_setting(key)));
                while let Some(value) = changes.next().await {
                    if !owner.is_live() || retry.get_untracked() != request {
                        return;
                    }
                    match value {
                        Ok(value) => batch(|| {
                            shown.set(value.as_deref() != Some("false"));
                            preference_failed.set(false);
                        }),
                        Err(error) => {
                            preference_failed.set(true);
                            log::warn!("Could not load pinned-message visibility: {error}");
                        }
                    }
                }
            });
        });
        effect(move || {
            let request = retry.get();
            let account = account.clone();
            let conversation_id = conversation_id.clone();
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
                loop {
                    let client = account.clone();
                    let id = conversation_id.clone();
                    let result = runtime::run(async move {
                        let Some(preview) = client.message().pin_message_preview(id).await? else {
                            return Ok(None);
                        };
                        let minimal = serde_json::from_str::<Value>(&preview.content).ok();
                        let resolved_text = if let Some(minimal) = minimal
                            && minimal["category"]
                                .as_str()
                                .is_some_and(|category| category.ends_with("_TEXT"))
                        {
                            let content =
                                minimal["content"].as_str().unwrap_or_default().to_owned();
                            client
                                .user()
                                .replace_mentions(vec![content])
                                .await?
                                .into_iter()
                                .next()
                        } else {
                            None
                        };
                        Ok(Some((preview, resolved_text)))
                    })
                    .await;
                    if !owner.is_live() || retry.get_untracked() != request {
                        return;
                    }
                    match result {
                        Ok(value) => batch(|| {
                            preview.set(value);
                            failed.set(false);
                        }),
                        Err(error) => {
                            failed.set(true);
                            log::warn!("Could not load pinned-message preview: {error}");
                        }
                    }
                    loop {
                        let Some(change) = changes.next().await else {
                            return;
                        };
                        if change.reload_all || change.conversation_ids.contains(&conversation_id) {
                            break;
                        }
                    }
                }
            });
        });
        let content = computed(move || {
            preview.with(|preview| {
                let (preview, resolved_text) = preview.as_ref()?;
                let minimal = serde_json::from_str::<Value>(&preview.content).ok()?;
                let category = minimal["category"].as_str()?;
                let message = presentation::preview(
                    i18n,
                    category,
                    resolved_text
                        .as_deref()
                        .or_else(|| minimal["content"].as_str())
                        .unwrap_or_default(),
                );
                Some(i18n.format(
                    "{name} pinned {message}",
                    "{name}置顶了{message}",
                    &[("name", &preview.sender_name), ("message", &message)],
                ))
            })
        });
        let presence =
            crate::motion::Presence::fade(move || shown.get() && content.with(Option::is_some));
        Self {
            i18n,
            owner,
            preview,
            busy,
            failed,
            preference_failed,
            retry,
            setting_key,
            settings: stored(Some(settings)),
            content,
            presence,
            target,
            side,
        }
    }

    /// A fixed pin, for the debug preview: no account, and no dismissal.
    #[cfg(debug_assertions)]
    pub(crate) fn detached(
        text: Signal<Option<String>>,
        side: Signal<Option<navigation::Panel>>,
    ) -> Self {
        let content = computed(move || text.get());
        Self {
            i18n: i18n::current(),
            owner: Owner::current(),
            preview: signal_with(None, SignalEquals::Never),
            busy: signal(false),
            failed: signal(false),
            preference_failed: signal(false),
            retry: signal(0),
            setting_key: stored(String::new()),
            settings: stored(None),
            content,
            presence: crate::motion::Presence::fade(move || content.with(Option::is_some)),
            target: signal(None),
            side,
        }
    }

    /// Whether the pin or its error has anything to show.
    pub(super) fn visible(self) -> bool {
        self.presence.mounted() || self.failed.get() || self.preference_failed.get()
    }

    /// A glass capsule: dismiss, the pinned message (a press jumps to it), and the
    /// pinned list. `lines` is 1 in the title row, 2 in a row of its own.
    pub(super) fn view(self, lines: u32) -> View {
        let Self {
            i18n,
            owner,
            preview,
            busy,
            failed,
            preference_failed,
            retry,
            setting_key,
            settings,
            content,
            presence,
            target,
            side,
            ..
        } = self;
        view()
            .min_w_0()
            .flex_shrink(1.0)
            .max_w(ui::glass_pin_max_width())
            .gap_2()
            .style_when(move || !self.visible(), |style| style.hidden())
            .child(when(
                move || presence.mounted(),
                move || {
                    presence.surface(
                        ui::glass_pill()
                            .flex_shrink(1.0)
                            .min_w_0()
                            .px_1()
                            .gap_1()
                            .child(
                                ui::round_button()
                                    .size_8()
                                    .disabled(busy)
                                    .accessibility_label(move || {
                                        Some(i18n.tr("Close", "关闭").into())
                                    })
                                    .on_press(move || {
                                        if busy.get_untracked() {
                                            return;
                                        }
                                        let Some(settings) =
                                            settings.with(|settings| settings.clone())
                                        else {
                                            return;
                                        };
                                        let key = setting_key.with(|key| key.clone());
                                        busy.set(true);
                                        preference_failed.set(false);
                                        let command = runtime::commit(async move {
                                            settings.set_setting(key, Some("false".into())).await
                                        });
                                        spawn(async move {
                                            let result = command.await;
                                            if owner.is_live() {
                                                busy.set(false);
                                                preference_failed.set(result.is_err());
                                            }
                                        });
                                    })
                                    .child(ui::icon(ui::Icon::Close).size(ui::icon_sm())),
                            )
                            .child(
                                ui::command()
                                    .flex_1()
                                    .min_w_0()
                                    .min_h_0()
                                    .px_1()
                                    .py_1()
                                    .rounded(ui::radius_full())
                                    .justify_start()
                                    .accessibility_label(move || {
                                        Some(i18n.text(Key::Pinned).into())
                                    })
                                    .on_press(move || {
                                        if let Some(id) = preview.with(|preview| {
                                            preview
                                                .as_ref()
                                                .map(|(item, _)| item.message_id.clone())
                                        }) {
                                            target.set(Some(id));
                                        }
                                    })
                                    .child(
                                        text(move || content.get().unwrap_or_default())
                                            .flex_1()
                                            .min_w_0()
                                            .text_role(ui::TextRole::Body)
                                            .number_of_lines(lines),
                                    ),
                            )
                            .child(
                                ui::round_button()
                                    .size_8()
                                    .accessibility_label(move || {
                                        Some(i18n.tr("Pinned messages", "置顶消息").into())
                                    })
                                    .selected(move || side.get() == Some(navigation::Panel::Pinned))
                                    .on_press(move || {
                                        side.set(
                                            if side.get_untracked()
                                                == Some(navigation::Panel::Pinned)
                                            {
                                                None
                                            } else {
                                                Some(navigation::Panel::Pinned)
                                            },
                                        );
                                    })
                                    .child(ui::icon(ui::Icon::Pin)),
                            ),
                    )
                },
            ))
            .child(when(
                move || failed.get() || preference_failed.get(),
                move || {
                    ui::glass_pill().px_3().child(load_error(move || {
                        retry.update(|revision| *revision = revision.wrapping_add(1))
                    }))
                },
            ))
    }
}

pub(crate) fn load_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(ui::error_text(move || i18n.text(Key::CommandFailed)))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}
