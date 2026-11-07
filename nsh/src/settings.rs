use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, AccountProfile};
use nsh_core::prelude::*;

use crate::{
    appearance::identity_badge,
    conversation::parts,
    i18n::{self, I18n, Key},
    runtime,
    ui::{self, Typography as _},
};

mod common;
use common::{Feedback, client};

mod about;
mod appearance;
mod avatar;
mod notifications;
mod profile;
mod proxy;
mod storage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Profile,
    Security,
    Notifications,
    Storage,
    Transfer,
    Proxy,
    Appearance,
    About,
}

impl Page {
    fn title(self, i18n: I18n) -> &'static str {
        match self {
            Self::Notifications => i18n.tr("Notifications", "通知"),
            Self::Security => i18n.tr("Security", "安全"),
            Self::Profile => i18n.tr("Edit profile", "编辑个人资料"),
            Self::Storage => i18n.tr("Data and storage", "数据与存储"),
            Self::Transfer => i18n.tr("Chat backup and restore", "聊天备份与恢复"),
            Self::Proxy => i18n.tr("Proxy", "代理"),
            Self::Appearance => i18n.tr("Appearance", "外观"),
            Self::About => i18n.tr("About", "关于"),
        }
    }

    fn icon(self) -> ui::Icon {
        match self {
            Self::Profile => ui::Icon::Person,
            Self::Notifications => ui::Icon::Bell,
            Self::Storage => ui::Icon::Database,
            Self::Transfer => ui::Icon::Upload,
            Self::Security => ui::Icon::Shield,
            Self::Proxy => ui::Icon::Globe,
            Self::Appearance => ui::Icon::Brush,
            Self::About => ui::Icon::InfoCircle,
        }
    }
}

pub(crate) fn navigation(
    account: Arc<AccountClient>,
    page: Signal<Page>,
    compact: impl Fn() -> bool + Copy + 'static,
    on_select: impl Fn() + Copy + 'static,
    on_back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let profile = signal_with(account.profile(), SignalEquals::Never);
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(account.profile_changes()));
        while let Some(next) = changes.next().await {
            profile.set(next);
        }
    });
    let row = move |value: Page| {
        ui::settings_row(Some(value.icon()), move || value.title(i18n))
            .selected(move || page.get() == value)
            .on_press(move || {
                batch(|| {
                    page.set(value);
                    on_select();
                });
            })
            .child(ui::chevron())
    };
    view()
        .flex_1()
        .min_h_0()
        .child(when(compact, move || {
            parts::page_header(
                move || i18n.text(Key::Settings),
                parts::back_button(on_back),
            )
        }))
        .child(when(move || !compact(), ui::title_bar))
        .child(
            scroll().flex_1().min_h_0().child(
                view()
                    .px(ui::space_lg())
                    .pb(ui::space_xl())
                    .gap(ui::space_lg())
                    .child(account_header(profile))
                    .child(ui::card().child(row(Page::Profile)))
                    .child(
                        ui::card().children(
                            [
                                Page::Notifications,
                                Page::Storage,
                                Page::Transfer,
                                Page::Security,
                                Page::Proxy,
                                Page::Appearance,
                                Page::About,
                            ]
                            .into_iter()
                            .filter(|value| cfg!(debug_assertions) || *value != Page::Transfer)
                            .map(row),
                        ),
                    ),
            ),
        )
}

fn account_header(profile: Signal<AccountProfile>) -> impl IntoElement {
    let name = move || profile.with(|p| p.full_name.clone());
    let badge = computed(move || profile.with(|p| (p.is_verified, p.membership.clone())));
    view()
        .items_center()
        .gap_1()
        .pb_2()
        .child(
            ui::avatar_view(
                move || profile.with(|p| p.user_id.clone()),
                move || name().into(),
                move || profile.with(|p| p.avatar_url.clone()),
                ui::AvatarSize::Medium,
            )
            .size_20()
            .mb_2(),
        )
        .child(
            view()
                .max_w(relative(1.0))
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    text(name)
                        .flex_shrink(1.0)
                        .min_w_0()
                        .number_of_lines(1)
                        .text_role(ui::TextRole::Title),
                )
                .child(dynamic(move || {
                    let (verified, membership) = badge.get();
                    untracked(|| identity_badge(false, verified, false, membership.as_deref()))
                })),
        )
        .child(
            text(move || profile.with(|p| format!("Mixin ID: {}", p.identity_number)))
                .text_role(ui::TextRole::Body)
                .selectable(true)
                .text_color(ui::fg_secondary),
        )
}

/// `profile_draft` is held by the session: this screen unmounts when a narrow window
/// returns to the settings list, and an unsaved profile edit must survive that.
pub(crate) fn screen(
    account: Arc<AccountClient>,
    page: Signal<Page>,
    profile_draft: Signal<Option<(String, String)>>,
    on_back: impl Fn() + Copy + 'static,
    on_signed_out: impl Fn() + 'static,
    session_owner: Owner,
) -> impl IntoElement {
    let i18n = i18n::current();
    let on_signed_out = Arc::new(on_signed_out);
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(ui::surface_pane)
        // Storage draws its own bar: its detail page has another title and Back.
        .child(when(
            move || page.get() != Page::Storage,
            move || parts::page_header(move || page.get().title(i18n), parts::back_button(on_back)),
        ))
        .child(dynamic(move || {
            let current = page.get();
            let account = account.clone();
            let on_signed_out = on_signed_out.clone();
            untracked(|| match current {
                Page::Profile => profile::screen(
                    account,
                    profile_draft,
                    move || on_signed_out(),
                    session_owner,
                )
                .into_element(),
                Page::Notifications => notifications::screen().into_element(),
                Page::Security => crate::security::settings(
                    use_context::<crate::security::Handle>().expect("Home provides screen privacy"),
                )
                .into_element(),
                Page::Storage => {
                    storage::screen(account, move || Page::Storage.title(i18n), on_back)
                        .into_element()
                }
                Page::Transfer => crate::transfer::screen(
                    use_context::<crate::transfer::Handle>()
                        .expect("Home provides device transfer state"),
                )
                .into_element(),
                Page::Proxy => proxy::screen().into_element(),
                Page::Appearance => appearance::screen().into_element(),
                Page::About => about::screen().into_element(),
            })
        }))
}
