//! The latest pinned-message preview and its conversation-scoped dismissal.

use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, PinMessagePreviewItem, SettingsClient};
use nsh_core::{element::ImageSrc, prelude::*};
use serde_json::Value;

use crate::{
    i18n::{self, Key},
    inspector, presentation, runtime,
    ui::{self, Typography},
};

pub(super) fn banner(
    account: Arc<AccountClient>,
    conversation_id: String,
    target: Signal<Option<String>>,
    side: Signal<Option<inspector::Mode>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let owner = Owner::current();
    let pinned = signal(false);
    let preview = signal_with(
        None::<(PinMessagePreviewItem, Option<String>)>,
        SignalEquals::Never,
    );
    let shown = signal(false);
    let busy = signal(false);
    let failed = signal(false);
    let preference_failed = signal(false);
    let retry = signal(0_u64);
    let settings = use_context::<Stored<Option<SettingsClient>>>()
        .expect("App provides settings")
        .with(|settings| settings.clone())
        .expect("Desktop settings initialized");
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
                        let content = minimal["content"].as_str().unwrap_or_default().to_owned();
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
                        pinned.set(value.is_some());
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
    let settings = stored(settings);
    view()
        .absolute()
        .top(ui::space_md())
        .left(ui::pin_banner_inset())
        .right(ui::space_md())
        .gap_2()
        .style_when(
            move || !pinned.get() && !failed.get() && !preference_failed.get(),
            |style| style.hidden(),
        )
        .child(when(
            move || presence.mounted(),
            move || {
                presence.surface(
                    view()
                        .relative()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .h(ui::pin_banner_height())
                        .px(ui::space_md())
                        .rounded(ui::radius_md())
                        .bg(ui::bubble_in)
                        .shadow_xs()
                        .shadow_color(ui::bubble_shadow())
                        .child(
                            ui::icon_button()
                                .size(ui::pin_close_size())
                                .px_0()
                                .py_0()
                                .disabled(busy)
                                .accessibility_label(move || Some(i18n.tr("Close", "关闭").into()))
                                .on_press(move || {
                                    if busy.get_untracked() {
                                        return;
                                    }
                                    let settings = settings.with(|settings| settings.clone());
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
                                .h_full()
                                .justify_start()
                                .accessibility_label(move || Some(i18n.text(Key::Pinned).into()))
                                .on_press(move || {
                                    if let Some(id) = preview.with(|preview| {
                                        preview.as_ref().map(|(item, _)| item.message_id.clone())
                                    }) {
                                        target.set(Some(id));
                                    }
                                })
                                .child(
                                    text(move || content.get().unwrap_or_default())
                                        .text_role(ui::TextRole::Body)
                                        .number_of_lines(2),
                                ),
                        )
                        .child(
                            ui::round_button()
                                .size(ui::pin_button_size())
                                .accessibility_label(move || Some(i18n.text(Key::Pinned).into()))
                                .on_press(move || {
                                    side.set(
                                        if side.get_untracked() == Some(inspector::Mode::Pinned) {
                                            None
                                        } else {
                                            Some(inspector::Mode::Pinned)
                                        },
                                    );
                                })
                                .child(ui::icon(ui::Icon::Pin)),
                        )
                        .child(
                            svg(ImageSrc::svg_markup(include_str!(
                                "../../assets/message/nip_left.svg"
                            )))
                            .absolute()
                            .left(-ui::pin_nip_width())
                            .bottom(ui::space_md())
                            .w(ui::pin_nip_width())
                            .h(ui::pin_nip_height())
                            .text_color(ui::bubble_in)
                            .accessibility_elements_hidden(true),
                        ),
                )
            },
        ))
        .child(when(
            move || failed.get() || preference_failed.get(),
            move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::error_text(move || i18n.text(Key::CommandFailed)))
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || retry.update(|revision| *revision = revision.wrapping_add(1)),
                    ))
            },
        ))
}
