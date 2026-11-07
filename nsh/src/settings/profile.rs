use super::{Feedback, body, section, tr};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};
use futures::StreamExt as _;
use mixin_desktop_api::AccountClient;
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;
pub(super) fn screen(
    account: Arc<AccountClient>,
    on_signed_out: impl Fn() + 'static,
    session_owner: Owner,
) -> impl IntoElement {
    let i18n = i18n::current();
    let initial = account.profile();
    let name = signal(initial.full_name.clone());
    let biography = signal(initial.biography.clone());
    let profile = signal_with(initial, SignalEquals::Never);
    let edited = signal(false);
    let refresh = signal(0_u64);
    let refresh_failed = signal(false);
    let feedback = Feedback::new();
    let client = account.clone();
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(client.profile_changes()));
        while let Some(next) = changes.next().await {
            if !edited.get_untracked() {
                name.set(next.full_name.clone());
                biography.set(next.biography.clone());
            }
            profile.set(next);
        }
    });
    let refresh_account = account.clone();
    effect(move || {
        let _revision = refresh.get();
        let client = refresh_account.clone();
        refresh_failed.set(false);
        spawn(async move {
            refresh_failed.set(
                runtime::run(async move { client.refresh_profile().await })
                    .await
                    .is_err(),
            );
        });
    });
    let avatar_account = account.clone();
    let save_account = account.clone();
    let sign_out_account = account;
    let sign_out = Arc::new(on_signed_out);
    let profile_owner = Owner::current();
    let confirming = signal(false);
    let signing_out = signal(false);
    let presence = crate::motion::Presence::modal(move || confirming.get());
    scroll().flex_1().min_h_0().child(
        body()
            .child(
                ui::avatar_view(
                    move || profile.with(|p| p.full_name.clone().into()),
                    move || profile.with(|p| p.avatar_url.clone()),
                )
                .size_20(),
            )
            .child(super::avatar::picker(
                avatar_account,
                profile,
                feedback,
                signing_out,
            ))
            .child(
                text(move || profile.with(|p| format!("Mixin ID: {}", p.identity_number)))
                    .text_sm()
                    .selectable(true)
                    .text_color(move || theme().muted_foreground),
            )
            .child(when(refresh_failed, move || {
                view()
                    .gap_2()
                    .child(
                        text(move || {
                            tr(
                                i18n,
                                "Couldn’t refresh your profile. Your saved profile is shown.",
                                "无法刷新个人资料，当前显示已保存的资料。",
                            )
                        })
                        .text_sm()
                        .text_color(move || theme().destructive),
                    )
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || refresh.update(|r| *r += 1),
                    ))
            }))
            .child(
                section(move || i18n.text(Key::Name)).child(
                    ui::field()
                        .label(move || i18n.text(Key::Name))
                        .value(move || name.get())
                        .max_length(40)
                        .readonly(move || feedback.busy.get() || signing_out.get())
                        .on_change(move |value: &str| {
                            edited.set(true);
                            name.set(value.into());
                        }),
                ),
            )
            .child(
                section(move || tr(i18n, "Biography", "个人简介")).child(
                    ui::field()
                        .label(move || tr(i18n, "Biography", "个人简介"))
                        .multiline(true)
                        .min_h_20()
                        .value(move || biography.get())
                        .max_length(140)
                        .readonly(move || feedback.busy.get() || signing_out.get())
                        .on_change(move |value: &str| {
                            edited.set(true);
                            biography.set(value.into());
                        }),
                ),
            )
            .child(
                section(move || tr(i18n, "Phone number", "手机号码")).child(
                    text(move || profile.with(|p| p.phone.clone()))
                        .text_sm()
                        .selectable(true),
                ),
            )
            .child(
                text(move || {
                    profile.with(|p| {
                        let date = chrono::DateTime::parse_from_rfc3339(&p.created_at)
                            .map(|date| {
                                date.with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d")
                                    .to_string()
                            })
                            .unwrap_or_else(|_| p.created_at.clone());
                        i18n.format("Joined {date}", "加入时间：{date}", &[("date", &date)])
                    })
                })
                .text_sm()
                .text_color(move || theme().muted_foreground),
            )
            .child(
                ui::primary()
                    .disabled(move || {
                        feedback.busy.get()
                            || signing_out.get()
                            || name.with(|n| n.trim().is_empty())
                    })
                    .child(text(move || tr(i18n, "Save", "保存")))
                    .on_press(move || {
                        let client = save_account.clone();
                        let next_name = name.get_untracked().trim().to_owned();
                        let next_biography = biography.get_untracked().trim().to_owned();
                        feedback.start();
                        spawn(async move {
                            match runtime::commit(async move {
                                client.update_profile(next_name, next_biography).await
                            })
                            .await
                            {
                                Ok(next) => {
                                    name.set(next.full_name.clone());
                                    biography.set(next.biography.clone());
                                    profile.set(next);
                                    edited.set(false);
                                    feedback.finish(true);
                                }
                                Err(_) => feedback.finish(false),
                            }
                        });
                    }),
            )
            .child(feedback.view(move || tr(i18n, "Saving…", "正在保存…")))
            .child(section(move || i18n.text(Key::Language)).child(i18n.picker()))
            .child(
                ui::command()
                    .text_color(move || theme().destructive)
                    .disabled(move || feedback.busy.get() || signing_out.get())
                    .child(text(move || tr(i18n, "Sign out…", "退出登录…")))
                    .on_press(move || confirming.set(true)),
            )
            .child(when(
                move || presence.mounted(),
                move || {
                    let client = sign_out_account.clone();
                    let sign_out = sign_out.clone();
                    ui::modal(
                        presence,
                        move || {
                            if !signing_out.get_untracked() {
                                confirming.set(false);
                            }
                        },
                        view()
                            .w_96()
                            .max_w(relative(1.0))
                            .gap_3()
                            .p_6()
                            .rounded_lg()
                            .bg(move || theme().popover)
                            .child(
                                text(move || {
                                    tr(
                                        i18n,
                                        "Sign out and remove this account’s local data?",
                                        "退出登录并移除此账号的本地数据？",
                                    )
                                })
                                .text_sm(),
                            )
                            .child(
                                view()
                                    .flex_row()
                                    .gap_3()
                                    .child(
                                        ui::action(
                                            move || i18n.text(Key::Cancel),
                                            move || confirming.set(false),
                                        )
                                        .disabled(signing_out),
                                    )
                                    .child(
                                        ui::command()
                                            .disabled(signing_out)
                                            .text_color(move || theme().destructive)
                                            .child(text(move || tr(i18n, "Sign out", "退出登录")))
                                            .on_press(move || {
                                                if !presence.interactive() {
                                                    return;
                                                }
                                                let client = client.clone();
                                                signing_out.set(true);
                                                feedback.failed.set(false);
                                                let sign_out = sign_out.clone();
                                                session_owner.run(|| {
                                                    spawn(async move {
                                                        match runtime::commit(async move {
                                                            client.sign_out().await
                                                        })
                                                        .await
                                                        {
                                                            Ok(()) => sign_out(),
                                                            Err(_) => {
                                                                if profile_owner.is_live() {
                                                                    signing_out.set(false);
                                                                    feedback.failed.set(true);
                                                                }
                                                            }
                                                        }
                                                    })
                                                });
                                            }),
                                    ),
                            ),
                    )
                },
            )),
    )
}
