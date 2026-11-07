use std::sync::Arc;

use mixin_desktop_api::AccountClient;
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    i18n::{self, I18n, Key},
    ui,
};

mod common;
use common::{Feedback, body, client, section, toggle, tr};

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
            Self::Notifications => tr(i18n, "Notifications", "通知"),
            Self::Security => tr(i18n, "Privacy and security", "隐私与安全"),
            Self::Profile => tr(i18n, "Edit profile", "编辑个人资料"),
            Self::Storage => tr(i18n, "Data and storage", "数据与存储"),
            Self::Transfer => tr(i18n, "Chat backup and restore", "聊天备份与恢复"),
            Self::Proxy => tr(i18n, "Proxy", "代理"),
            Self::Appearance => tr(i18n, "Appearance", "外观"),
            Self::About => tr(i18n, "About", "关于"),
        }
    }
}

pub(crate) fn navigation(
    page: Signal<Page>,
    compact: impl Fn() -> bool + Copy + 'static,
    on_select: impl Fn() + Copy + 'static,
    on_back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    view()
        .flex_1()
        .min_h_0()
        .child(when(compact, move || {
            header(move || i18n.text(Key::Settings), on_back)
        }))
        .child(
            scroll().flex_1().min_h_0().child(
                view().gap_1().px_3().py_4().children(
                    [
                        Page::Profile,
                        Page::Security,
                        Page::Notifications,
                        Page::Storage,
                        Page::Transfer,
                        Page::Proxy,
                        Page::Appearance,
                        Page::About,
                    ]
                    .into_iter()
                    .filter(|value| cfg!(debug_assertions) || *value != Page::Transfer)
                    .map(move |value| {
                        ui::command()
                            .justify_start()
                            .min_h_12()
                            .selected(move || page.get() == value)
                            .selected_style(|s| s.bg(move || theme().accent))
                            .on_press(move || {
                                batch(|| {
                                    page.set(value);
                                    on_select();
                                });
                            })
                            .child(text(move || value.title(i18n)))
                    }),
                ),
            ),
        )
}

fn header(
    title: impl Fn() -> &'static str + 'static,
    on_back: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    view()
        .h_16()
        .flex_row()
        .items_center()
        .gap_3()
        .px_4()
        .border_b_1()
        .border_color(move || theme().border)
        .child(
            ui::icon_button()
                .on_press(on_back)
                .accessibility_label(move || Some(i18n.text(Key::Back).into()))
                .child(ui::icon(ui::Icon::ArrowLeft)),
        )
        .child(text(title).text_lg().font_weight(FontWeight::SEMIBOLD))
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    page: Signal<Page>,
    on_back: impl Fn() + 'static,
    on_signed_out: impl Fn() + 'static,
    session_owner: Owner,
) -> impl IntoElement {
    let i18n = i18n::current();
    let on_signed_out = Arc::new(on_signed_out);
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .child(header(move || page.get().title(i18n), on_back))
        .child(dynamic(move || {
            let current = page.get();
            let account = account.clone();
            let on_signed_out = on_signed_out.clone();
            untracked(|| match current {
                Page::Profile => {
                    profile::screen(account, move || on_signed_out(), session_owner).into_element()
                }
                Page::Notifications => notifications::screen().into_element(),
                Page::Security => crate::security::settings(
                    use_context::<crate::security::Handle>().expect("Home provides screen privacy"),
                )
                .into_element(),
                Page::Storage => storage::screen(account).into_element(),
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
