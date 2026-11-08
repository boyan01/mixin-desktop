use mixin_desktop_api::{ProxyItem, ProxySettingsItem};
use nsh_core::prelude::*;

use super::{Feedback, client};
use crate::{
    i18n::{self, Key},
    runtime,
    ui::parts::{self, Prompt, confirm},
    ui::{self, Typography as _},
};

const VISIBLE_ROWS: usize = 3;

pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let settings = signal_with(
        ProxySettingsItem {
            enabled: false,
            selected_proxy_id: None,
            proxies: Vec::new(),
        },
        SignalEquals::Never,
    );
    let ready = signal(false);
    let refresh = signal(0_u64);
    let feedback = Feedback::new();
    effect(move || {
        let _revision = refresh.get();
        ready.set(false);
        feedback.failed.set(false);
        spawn(async move {
            match runtime::run(async { client().await?.proxy_settings().await }).await {
                Ok(value) => {
                    settings.set(value);
                    ready.set(true);
                }
                Err(_) => feedback.failed.set(true),
            }
        });
    });
    let adding = signal(false);
    let presence = crate::motion::Presence::modal(move || adding.get());
    scroll().flex_1().min_h_0().child(
        parts::page_body()
            .child(
                view()
                    .gap(ui::space_sm())
                    .child(ui::card().child(ui::toggle_row(
                        move || i18n.tr("Proxy", "代理"),
                        move || settings.with(|s| s.enabled),
                        move || {
                            !ready.get()
                                || feedback.busy.get()
                                || settings.with(|s| s.proxies.is_empty())
                        },
                        move |enabled| {
                            let mut next = settings.get_untracked();
                            if enabled && next.selected_proxy_id.is_none() {
                                next.selected_proxy_id = next.proxies.first().map(|p| p.id.clone());
                            }
                            next.enabled = enabled;
                            save(next, settings, feedback);
                        },
                    )))
                    .child(when(
                        move || ready.get() && settings.with(|s| s.proxies.is_empty()),
                        move || {
                            parts::note(move || {
                                i18n.tr(
                                    "Add a proxy before enabling it.",
                                    "请先添加代理，再开启代理连接。",
                                )
                            })
                        },
                    )),
            )
            .child(when(
                move || !ready.get() && !feedback.failed.get(),
                move || parts::status(move || i18n.tr("Loading proxies…", "正在加载代理…")),
            ))
            .child(
                ui::card()
                    .child(
                        virtual_list(
                            move || settings.with(|s| s.proxies.len()),
                            move |at| proxy_row(settings, feedback, move || at.index()),
                        )
                        .keys(move || {
                            settings.with(|s| {
                                s.proxies.iter().map(|p| p.id.clone()).collect::<Vec<_>>()
                            })
                        })
                        .estimated_item_size(ui::row_height())
                        .h(move || {
                            px(settings.with(|s| s.proxies.len().min(VISIBLE_ROWS)) as f32
                                * ui::row_height().0)
                        })
                        .style_when(
                            move || settings.with(|s| s.proxies.is_empty()),
                            |style| style.hidden(),
                        ),
                    )
                    .child(
                        ui::settings_row(Some(ui::Icon::Add), move || {
                            i18n.tr("Add proxy…", "添加代理…")
                        })
                        .disabled(move || !ready.get() || feedback.busy.get())
                        .on_press(move || adding.set(true)),
                    ),
            )
            .child(feedback.view(move || i18n.tr("Saving…", "正在保存…")))
            .child(when(
                move || feedback.failed.get() && !ready.get(),
                move || {
                    ui::action(
                        move || i18n.text(Key::Retry),
                        move || refresh.update(|r| *r += 1),
                    )
                },
            ))
            .child(when(
                move || presence.mounted(),
                move || form(settings, feedback, adding, presence),
            )),
    )
}

fn proxy_row(
    settings: Signal<ProxySettingsItem>,
    feedback: Feedback,
    index: impl Fn() -> usize + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let proxy = move || settings.with(|s| s.proxies.get(index()).cloned());
    let selected = move || {
        settings.with(|s| {
            s.proxies
                .get(index())
                .is_some_and(|p| s.selected_proxy_id.as_ref() == Some(&p.id))
        })
    };
    view()
        .flex_row()
        .items_center()
        .gap(ui::space_md())
        .child(
            ui::card_row()
                .flex_1()
                .min_w_0()
                .disabled(feedback.busy)
                .selected(selected)
                .child(
                    view()
                        .gap(ui::space_xs())
                        .min_w_0()
                        .flex_1()
                        .child(
                            text(move || proxy().map(|p| address(&p)).unwrap_or_default())
                                .text_role(ui::TextRole::Label)
                                .number_of_lines(1),
                        )
                        .child(
                            text(move || {
                                proxy().map(|p| p.kind.to_uppercase()).unwrap_or_default()
                            })
                            .text_role(ui::TextRole::Caption)
                            .text_color(ui::fg_secondary),
                        ),
                )
                .child(when(selected, || ui::icon(ui::Icon::Check)))
                .on_press(move || {
                    let Some(proxy) = untracked(proxy) else {
                        return;
                    };
                    let mut next = settings.get_untracked();
                    next.selected_proxy_id = Some(proxy.id);
                    save(next, settings, feedback);
                }),
        )
        .child(
            ui::button(ui::ButtonTone::Danger, ui::ButtonSize::Small)
                .mr(ui::space_md())
                .disabled(feedback.busy)
                .child(text(move || i18n.tr("Remove…", "移除…")))
                .on_press(move || {
                    if let Some(proxy) = untracked(proxy) {
                        remove(proxy, settings, feedback);
                    }
                }),
        )
}

fn address(proxy: &ProxyItem) -> String {
    format!("{}:{}", proxy.host, proxy.port)
}

fn remove(proxy: ProxyItem, settings: Signal<ProxySettingsItem>, feedback: Feedback) {
    let i18n = i18n::current();
    feedback.owner.run(|| {
        spawn(async move {
            let prompt = Prompt::new(
                i18n.tr("Remove proxy?", "移除代理？"),
                address(&proxy),
                i18n.tr("Remove", "移除"),
            );
            if !confirm(prompt).await {
                return;
            }
            let mut next = settings.get_untracked();
            next.proxies.retain(|p| p.id != proxy.id);
            if next.selected_proxy_id.as_ref() == Some(&proxy.id) {
                next.selected_proxy_id = None;
                next.enabled = false;
            }
            save(next, settings, feedback);
        })
    });
}

/// Removing a row disposes the cell that pressed it, so the page awaits the write.
fn save(next: ProxySettingsItem, settings: Signal<ProxySettingsItem>, feedback: Feedback) {
    let previous = settings.get_untracked();
    settings.set(next.clone());
    feedback.start();
    let task = runtime::commit(async move { client().await?.set_proxy_settings(next).await });
    feedback.owner.run(|| {
        spawn(async move {
            let result = task.await;
            if result.is_err() {
                settings.set(previous);
            }
            feedback.finish(result.is_ok());
        })
    });
}

fn form(
    settings: Signal<ProxySettingsItem>,
    feedback: Feedback,
    adding: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let host = signal(String::new());
    let port = signal(String::new());
    let username = signal(String::new());
    let password = signal(String::new());
    let composing = signal(false);
    let cancel = move || {
        if presence.interactive() && !feedback.busy.get_untracked() && !composing.get_untracked() {
            adding.set(false);
        }
    };
    let cancel_key = move |key: &str| {
        if key == "Escape" {
            cancel();
        }
    };
    let composition = move |event: &Composition| composing.set(event.composing);
    let add = move || {
        let port = port.get_untracked().parse::<u16>().unwrap_or_default();
        if !presence.interactive() || port == 0 {
            return;
        }
        let mut next = settings.get_untracked();
        let id = format!(
            "proxy-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        next.proxies.push(ProxyItem {
            id,
            kind: "http".into(),
            host: host.get_untracked().trim().into(),
            port,
            username: (!username.with(|s| s.is_empty())).then(|| username.get_untracked()),
            password: (!password.with(|s| s.is_empty())).then(|| password.get_untracked()),
        });
        feedback.start();
        let applied = next.clone();
        let task = runtime::commit(async move { client().await?.set_proxy_settings(next).await });
        feedback.owner.run(|| {
            spawn(async move {
                let result = task.await;
                if result.is_ok() {
                    settings.set(applied);
                    adding.set(false);
                }
                feedback.finish(result.is_ok());
            })
        });
    };
    let fields = view()
        .gap(ui::space_lg())
        .child(
            parts::labeled(move || i18n.tr("Host", "主机")).child(
                ui::field()
                    .label(move || i18n.tr("Host", "主机"))
                    .auto_focus(true)
                    .value(move || host.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| host.set(value.into())),
            ),
        )
        .child(
            parts::labeled(move || i18n.tr("Port", "端口")).child(
                ui::field()
                    .label(move || i18n.tr("Port", "端口"))
                    .value(move || port.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| port.set(value.into())),
            ),
        )
        .child(
            parts::labeled(move || i18n.tr("Username (optional)", "用户名（可选）")).child(
                ui::field()
                    .label(move || i18n.tr("Username", "用户名"))
                    .value(move || username.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| username.set(value.into())),
            ),
        )
        .child(
            parts::labeled(move || i18n.tr("Password (optional)", "密码（可选）")).child(
                ui::field()
                    .label(move || i18n.tr("Password", "密码"))
                    .masked(true)
                    .value(move || password.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| password.set(value.into())),
            ),
        );
    ui::dialog(
        presence,
        cancel,
        ui::dialog_card(ui::DialogWidth::Regular)
            .overflow_hidden()
            .child(ui::dialog_title(move || {
                i18n.tr("Add HTTP proxy", "添加 HTTP 代理")
            }))
            .child(scroll().min_h_0().flex_shrink(1.0).child(fields))
            .child(when(feedback.failed, move || {
                ui::error_text(move || i18n.text(Key::CommandFailed))
            }))
            .child(
                parts::dialog_actions()
                    .child(
                        ui::action(move || i18n.text(Key::Cancel), cancel).disabled(feedback.busy),
                    )
                    .child(
                        ui::primary()
                            .disabled(move || {
                                feedback.busy.get()
                                    || host.with(|h| h.trim().is_empty())
                                    || port.with(|p| !p.parse::<u16>().is_ok_and(|port| port > 0))
                            })
                            .child(text(move || i18n.tr("Add", "添加")))
                            .on_press(add),
                    ),
            ),
    )
}
