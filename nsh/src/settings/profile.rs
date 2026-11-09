use super::Feedback;
use crate::{
    i18n::{self, Key},
    runtime,
    ui::parts,
    ui::{self, Typography as _},
};
use futures::StreamExt as _;
use mixin_desktop_api::AccountClient;
use nsh_core::prelude::*;
use std::sync::Arc;
pub(super) fn screen(
    account: Arc<AccountClient>,
    draft: Signal<Option<(String, String)>>,
    on_signed_out: impl Fn() + 'static,
    session_owner: Owner,
) -> impl IntoElement {
    let initial = account.profile();
    let (initial_name, initial_biography) = draft
        .get_untracked()
        .unwrap_or_else(|| (initial.full_name.clone(), initial.biography.clone()));
    let name = signal(initial_name);
    let biography = signal(initial_biography);
    let profile = signal_with(initial, SignalEquals::Never);
    let keep_draft = move || draft.set(Some((name.get_untracked(), biography.get_untracked())));
    let refresh = signal(0_u64);
    let refresh_failed = signal(false);
    let feedback = Feedback::new();
    let client = account.clone();
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(client.profile_changes()));
        while let Some(next) = changes.next().await {
            if draft.with_untracked(Option::is_none) {
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
    let save = move || {
        let client = save_account.clone();
        let next_name = name.get_untracked().trim().to_owned();
        let next_biography = biography.get_untracked().trim().to_owned();
        feedback.start();
        let task =
            runtime::commit(async move { client.update_profile(next_name, next_biography).await });
        spawn(async move {
            match task.await {
                Ok(next) => {
                    name.set(next.full_name.clone());
                    biography.set(next.biography.clone());
                    profile.set(next);
                    draft.set(None);
                    feedback.finish(true);
                }
                Err(_) => feedback.finish(false),
            }
        });
    };
    let avatar_picker = super::avatar::picker(avatar_account, profile, feedback, signing_out);
    let sign_out_surface = move || {
        sign_out_dialog(
            sign_out_account.clone(),
            sign_out.clone(),
            SignOut {
                presence,
                confirming,
                signing_out,
                feedback,
                session_owner,
                profile_owner,
            },
        )
        .into_element()
    };
    content(
        profile,
        name,
        biography,
        draft,
        feedback,
        refresh_failed,
        refresh,
        signing_out,
        confirming,
        presence,
        keep_draft,
        save,
        avatar_picker,
        sign_out_surface,
    )
}

/// A read-only row of the account's facts.
fn fact(
    label: impl Fn() -> &'static str + 'static,
    value: impl Fn() -> String + 'static,
) -> impl IntoElement {
    ui::static_row(label).child(
        text(value)
            .text_role(ui::TextRole::Body)
            .selectable(true)
            .text_color(ui::fg_secondary),
    )
}

#[derive(Clone, Copy)]
struct SignOut {
    presence: crate::motion::Presence,
    confirming: Signal<bool>,
    signing_out: Signal<bool>,
    feedback: Feedback,
    session_owner: Owner,
    profile_owner: Owner,
}

fn sign_out_dialog(
    account: Arc<AccountClient>,
    on_signed_out: Arc<impl Fn() + 'static>,
    state: SignOut,
) -> impl IntoElement {
    let SignOut {
        presence,
        confirming,
        signing_out,
        feedback,
        session_owner,
        profile_owner,
    } = state;
    let confirm = move || {
        if !presence.interactive() {
            return;
        }
        let client = account.clone();
        signing_out.set(true);
        feedback.failed.set(false);
        let on_signed_out = on_signed_out.clone();
        session_owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move { client.sign_out().await }).await;
                match result {
                    Ok(()) => on_signed_out(),
                    // The failure shows on the page beneath this dialog.
                    Err(_) if profile_owner.is_live() => {
                        signing_out.set(false);
                        confirming.set(false);
                        feedback.failed.set(true);
                    }
                    Err(_) => {}
                }
            })
        });
    };
    ui::dialog(
        presence,
        move || {
            if !signing_out.get_untracked() {
                confirming.set(false);
            }
        },
        sign_out_view(confirming, signing_out, confirm),
    )
}

fn sign_out_view(
    confirming: Signal<bool>,
    signing_out: Signal<bool>,
    confirm: impl Fn() + 'static,
) -> View {
    let i18n = i18n::current();
    ui::dialog_card(ui::DialogWidth::Regular)
        .child(ui::dialog_title(move || {
            i18n.tr(
                "Sign out and remove this account’s local data?",
                "退出登录并移除此账号的本地数据？",
            )
        }))
        .child(
            parts::dialog_actions()
                .child(
                    ui::action(
                        move || i18n.text(Key::Cancel),
                        move || confirming.set(false),
                    )
                    .disabled(signing_out),
                )
                .child(
                    ui::button(ui::ButtonTone::Danger, ui::ButtonSize::Regular)
                        .disabled(signing_out)
                        .child(text(move || i18n.tr("Sign out", "退出登录")))
                        .on_press(confirm),
                ),
        )
}
#[cfg(debug_assertions)]
pub(crate) fn preview_sign_out() -> impl IntoElement {
    sign_out_view(signal(true), signal(false), || {})
}

#[allow(clippy::too_many_arguments)]
fn content<E: IntoElement>(
    profile: Signal<mixin_desktop_api::AccountProfile>,
    name: Signal<String>,
    biography: Signal<String>,
    draft: Signal<Option<(String, String)>>,
    feedback: Feedback,
    refresh_failed: Signal<bool>,
    refresh: Signal<u64>,
    signing_out: Signal<bool>,
    confirming: Signal<bool>,
    presence: crate::motion::Presence,
    keep_draft: impl Fn() + Copy + 'static,
    save: impl Fn() + 'static,
    avatar_picker: E,
    sign_out_surface: impl Fn() -> AnyElement + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    scroll().flex_1().min_h_0().child(
        parts::page_body()
            .child(
                view()
                    .items_center()
                    .gap_2()
                    .child(
                        ui::avatar_view(
                            move || profile.with(|p| p.user_id.clone()),
                            move || profile.with(|p| p.full_name.clone().into()),
                            move || profile.with(|p| p.avatar_url.clone()),
                            ui::AvatarSize::Medium,
                        )
                        .size_20(),
                    )
                    .child(avatar_picker),
            )
            .child(when(refresh_failed, move || {
                parts::failure(
                    move || {
                        i18n.tr(
                            "Couldn’t refresh your profile. Your saved profile is shown.",
                            "无法刷新个人资料，当前显示已保存的资料。",
                        )
                    },
                    move || refresh.update(|r| *r += 1),
                )
            }))
            .child(
                view()
                    .gap_3()
                    .child(
                        ui::card()
                            .p_4()
                            .gap_4()
                            .child(
                                parts::labeled(move || i18n.text(Key::Name)).child(
                                    ui::field()
                                        .label(move || i18n.text(Key::Name))
                                        .value(move || name.get())
                                        .max_length(40)
                                        .readonly(move || feedback.busy.get() || signing_out.get())
                                        .on_change(move |value: &str| {
                                            name.set(value.into());
                                            keep_draft();
                                        }),
                                ),
                            )
                            .child(
                                parts::labeled(move || i18n.tr("Biography", "个人简介")).child(
                                    ui::textarea()
                                        .label(move || i18n.tr("Biography", "个人简介"))
                                        .min_h_20()
                                        .value(move || biography.get())
                                        .max_length(140)
                                        .readonly(move || feedback.busy.get() || signing_out.get())
                                        .on_change(move |value: &str| {
                                            biography.set(value.into());
                                            keep_draft();
                                        }),
                                ),
                            ),
                    )
                    .child(
                        ui::primary()
                            .self_end()
                            .disabled(move || {
                                feedback.busy.get()
                                    || signing_out.get()
                                    || draft.with(Option::is_none)
                                    || name.with(|n| n.trim().is_empty())
                            })
                            .child(text(move || i18n.tr("Save", "保存")))
                            .on_press(save),
                    )
                    .child(feedback.view(move || i18n.tr("Saving…", "正在保存…"))),
            )
            .child(
                ui::card()
                    .child(fact(
                        move || i18n.tr("Phone number", "手机号码"),
                        move || profile.with(|p| p.phone.clone()),
                    ))
                    .child(fact(
                        || "Mixin ID",
                        move || profile.with(|p| p.identity_number.clone()),
                    ))
                    .child(fact(
                        move || i18n.tr("Joined", "加入时间"),
                        move || {
                            profile.with(|p| {
                                chrono::DateTime::parse_from_rfc3339(&p.created_at)
                                    .map(|date| {
                                        date.with_timezone(&chrono::Local)
                                            .format("%Y-%m-%d")
                                            .to_string()
                                    })
                                    .unwrap_or_else(|_| p.created_at.clone())
                            })
                        },
                    )),
            )
            .child(
                ui::card().child(
                    ui::settings_row(None, move || i18n.tr("Sign out…", "退出登录…"))
                        .text_color(ui::danger)
                        .disabled(move || feedback.busy.get() || signing_out.get())
                        .on_press(move || confirming.set(true)),
                ),
            )
            .child(when(move || presence.mounted(), sign_out_surface)),
    )
}
#[cfg(debug_assertions)]
pub(crate) fn preview(
    profile: mixin_desktop_api::AccountProfile,
    failed: Signal<bool>,
) -> impl IntoElement {
    let name = signal(profile.full_name.clone());
    let biography = signal(profile.biography.clone());
    let draft = signal(None);
    content(
        signal_with(profile, SignalEquals::Never),
        name,
        biography,
        draft,
        Feedback::new(),
        failed,
        signal(0),
        signal(false),
        signal(false),
        crate::motion::Presence::modal(|| false),
        move || draft.set(Some((name.get_untracked(), biography.get_untracked()))),
        || {},
        super::avatar::preview(),
        || view().hidden().into_element(),
    )
}
