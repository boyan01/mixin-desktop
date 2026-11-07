use mixin_desktop_api::{ProxyItem, ProxySettingsItem};
use nsh_base::theme;
use nsh_core::prelude::*;

use super::{Feedback, body, client, section, toggle, tr};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

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
        body()
            .child(toggle(
                move || tr(i18n, "Proxy", "代理"),
                move || settings.with(|s| s.enabled),
                move || {
                    !ready.get() || feedback.busy.get() || settings.with(|s| s.proxies.is_empty())
                },
                move |enabled| {
                    let mut next = settings.get_untracked();
                    if enabled && next.selected_proxy_id.is_none() {
                        next.selected_proxy_id = next.proxies.first().map(|p| p.id.clone());
                    }
                    next.enabled = enabled;
                    save(next, settings, feedback);
                },
            ))
            .child(when(
                move || ready.get() && settings.with(|s| s.proxies.is_empty()),
                move || {
                    text(move || {
                        tr(
                            i18n,
                            "Add a proxy before enabling it.",
                            "请先添加代理，再开启代理连接。",
                        )
                    })
                    .text_sm()
                    .text_color(move || theme().muted_foreground)
                },
            ))
            .child(when(
                move || !ready.get() && !feedback.failed.get(),
                move || text(move || tr(i18n, "Loading proxies…", "正在加载代理…")).text_sm(),
            ))
            .child(
                ui::command()
                    .justify_start()
                    .disabled(move || !ready.get() || feedback.busy.get())
                    .child(ui::icon(ui::Icon::Add))
                    .child(text(move || tr(i18n, "Add proxy…", "添加代理…")))
                    .on_press(move || adding.set(true)),
            )
            .child(
                virtual_list(
                    move || settings.with(|s| s.proxies.len()),
                    move |at| {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                ui::command()
                                    .flex_1()
                                    .min_w_0()
                                    .justify_start()
                                    .py_3()
                                    .gap_3()
                                    .disabled(feedback.busy)
                                    .selected(move || {
                                        settings.with(|s| {
                                            s.proxies.get(at.index()).is_some_and(|p| {
                                                s.selected_proxy_id.as_ref() == Some(&p.id)
                                            })
                                        })
                                    })
                                    .selected_style(|s| s.bg(move || theme().accent))
                                    .child(view().w_5().child(text(move || {
                                        settings.with(|s| {
                                            if s.proxies.get(at.index()).is_some_and(|p| {
                                                s.selected_proxy_id.as_ref() == Some(&p.id)
                                            }) {
                                                "✓"
                                            } else {
                                                ""
                                            }
                                        })
                                    })))
                                    .child(
                                        view()
                                            .gap_1()
                                            .min_w_0()
                                            .flex_1()
                                            .child(
                                                text(move || {
                                                    settings.with(|s| {
                                                        s.proxies
                                                            .get(at.index())
                                                            .map(|p| {
                                                                format!("{}:{}", p.host, p.port)
                                                            })
                                                            .unwrap_or_default()
                                                    })
                                                })
                                                .number_of_lines(1),
                                            )
                                            .child(
                                                text(move || {
                                                    settings.with(|s| {
                                                        s.proxies
                                                            .get(at.index())
                                                            .map(|p| p.kind.to_uppercase())
                                                            .unwrap_or_default()
                                                    })
                                                })
                                                .text_xs()
                                                .text_color(move || theme().muted_foreground),
                                            ),
                                    )
                                    .on_press(move || {
                                        let mut next = settings.get_untracked();
                                        let Some(id) =
                                            next.proxies.get(at.index()).map(|p| p.id.clone())
                                        else {
                                            return;
                                        };
                                        next.selected_proxy_id = Some(id);
                                        save(next, settings, feedback);
                                    }),
                            )
                            .child(
                                ui::command()
                                    .text_color(move || theme().destructive)
                                    .disabled(feedback.busy)
                                    .child(text(move || tr(i18n, "Remove", "移除")))
                                    .on_press(move || {
                                        let mut next = settings.get_untracked();
                                        if at.index() >= next.proxies.len() {
                                            return;
                                        }
                                        let removed = next.proxies.remove(at.index());
                                        if next.selected_proxy_id.as_ref() == Some(&removed.id) {
                                            next.selected_proxy_id = None;
                                            next.enabled = false;
                                        }
                                        save(next, settings, feedback);
                                    }),
                            )
                    },
                )
                .keys(move || {
                    settings.with(|s| s.proxies.iter().map(|p| p.id.clone()).collect::<Vec<_>>())
                })
                .estimated_item_size(px(72.0))
                .h_56(),
            )
            .child(feedback.view(move || tr(i18n, "Saving…", "正在保存…")))
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

fn save(next: ProxySettingsItem, settings: Signal<ProxySettingsItem>, feedback: Feedback) {
    let previous = settings.get_untracked();
    settings.set(next.clone());
    feedback.start();
    spawn(async move {
        let result =
            runtime::commit(async move { client().await?.set_proxy_settings(next).await }).await;
        if result.is_err() {
            settings.set(previous);
        }
        feedback.finish(result.is_ok());
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
    let fields = view()
        .gap_4()
        .child(
            section(move || tr(i18n, "Host", "主机")).child(
                ui::field()
                    .label(move || tr(i18n, "Host", "主机"))
                    .auto_focus(true)
                    .value(move || host.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| host.set(value.into())),
            ),
        )
        .child(
            section(move || tr(i18n, "Port", "端口")).child(
                ui::field()
                    .label(move || tr(i18n, "Port", "端口"))
                    .value(move || port.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| port.set(value.into())),
            ),
        )
        .child(
            section(move || tr(i18n, "Username (optional)", "用户名（可选）")).child(
                ui::field()
                    .label(move || tr(i18n, "Username", "用户名"))
                    .value(move || username.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| username.set(value.into())),
            ),
        )
        .child(
            section(move || tr(i18n, "Password (optional)", "密码（可选）")).child(
                ui::field()
                    .label(move || tr(i18n, "Password", "密码"))
                    .masked(true)
                    .value(move || password.get())
                    .readonly(feedback.busy)
                    .on_key_press(cancel_key)
                    .on_composition_change(composition)
                    .on_change(move |value: &str| password.set(value.into())),
            ),
        );
    ui::modal(
        presence,
        cancel,
        view()
            .w_96()
            .max_w(relative(1.0))
            .max_h(relative(1.0))
            .min_h_0()
            .overflow_hidden()
            .rounded_lg()
            .bg(move || theme().popover)
            .child(
                view().flex_none().px_6().pt_6().pb_4().child(
                    text(move || tr(i18n, "Add HTTP proxy", "添加 HTTP 代理"))
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD),
                ),
            )
            .child(
                scroll()
                    .min_h_0()
                    .flex_shrink(1.0)
                    .child(fields.px_6().py_2()),
            )
            .child(
                view()
                    .flex_none()
                    .px_6()
                    .pt_4()
                    .pb_6()
                    .gap_3()
                    .child(
                        view()
                            .flex_row()
                            .justify_end()
                            .gap_3()
                            .child(
                                ui::action(move || i18n.text(Key::Cancel), cancel)
                                    .disabled(feedback.busy),
                            )
                            .child(
                                ui::primary()
                                    .disabled(move || {
                                        feedback.busy.get()
                                            || host.with(|h| h.trim().is_empty())
                                            || port.with(|p| {
                                                !p.parse::<u16>().is_ok_and(|port| port > 0)
                                            })
                                    })
                                    .child(text(move || tr(i18n, "Add", "添加")))
                                    .on_press(move || {
                                        if !presence.interactive() {
                                            return;
                                        }
                                        let Ok(port) = port.get_untracked().parse::<u16>() else {
                                            return;
                                        };
                                        if port == 0 {
                                            return;
                                        }
                                        let mut next = settings.get_untracked();
                                        let id = format!(
                                            "proxy-{}",
                                            chrono::Utc::now()
                                                .timestamp_nanos_opt()
                                                .unwrap_or_default()
                                        );
                                        next.proxies.push(ProxyItem {
                                            id,
                                            kind: "http".into(),
                                            host: host.get_untracked().trim().into(),
                                            port,
                                            username: (!username.with(|s| s.is_empty()))
                                                .then(|| username.get_untracked()),
                                            password: (!password.with(|s| s.is_empty()))
                                                .then(|| password.get_untracked()),
                                        });
                                        feedback.start();
                                        spawn(async move {
                                            let applied = next.clone();
                                            let result = runtime::commit(async move {
                                                client().await?.set_proxy_settings(next).await
                                            })
                                            .await;
                                            if result.is_ok() {
                                                settings.set(applied);
                                                adding.set(false);
                                            }
                                            feedback.finish(result.is_ok());
                                        });
                                    }),
                            ),
                    )
                    .child(when(feedback.failed, move || {
                        text(move || i18n.text(Key::CommandFailed))
                            .text_sm()
                            .text_color(move || theme().destructive)
                    })),
            ),
    )
}
