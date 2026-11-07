use crate::i18n::{self, I18n, Key};
use mixin_desktop_api::{DesktopClient, SettingsClient};
use nsh_base::theme;
use nsh_core::prelude::*;

pub(in crate::settings) fn tr(i18n: I18n, en: &'static str, zh: &'static str) -> &'static str {
    i18n.tr(en, zh)
}

pub(in crate::settings) async fn client() -> mixin_desktop_api::ClientResult<SettingsClient> {
    Ok(DesktopClient::open().await?.settings())
}

pub(in crate::settings) fn section(title: impl Fn() -> &'static str + 'static) -> View {
    view()
        .gap_3()
        .child(text(title).text_sm().font_weight(FontWeight::MEDIUM))
}

pub(in crate::settings) fn body() -> View {
    view().w_full().max_w(px(600.0)).p_6().gap_6()
}

#[derive(Clone, Copy)]
pub(in crate::settings) struct Feedback {
    pub(in crate::settings) busy: Signal<bool>,
    pub(in crate::settings) failed: Signal<bool>,
}

impl Feedback {
    pub(in crate::settings) fn new() -> Self {
        Self {
            busy: signal(false),
            failed: signal(false),
        }
    }
    pub(in crate::settings) fn start(self) {
        batch(|| {
            self.busy.set(true);
            self.failed.set(false);
        });
    }
    pub(in crate::settings) fn finish(self, succeeded: bool) {
        batch(|| {
            self.busy.set(false);
            self.failed.set(!succeeded);
        });
    }
    pub(in crate::settings) fn view(
        self,
        busy_label: impl Fn() -> &'static str + Copy + 'static,
    ) -> impl IntoElement {
        let i18n = i18n::current();
        view()
            .gap_2()
            .child(when(self.busy, move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .text_color(move || theme().muted_foreground)
                    .child(crate::ui::busy_indicator(
                        move || self.busy.get(),
                        move || busy_label().into(),
                    ))
                    .child(text(busy_label).text_sm())
            }))
            .child(when(self.failed, move || {
                text(move || i18n.text(Key::CommandFailed))
                    .text_sm()
                    .text_color(move || theme().destructive)
            }))
    }
}

pub(in crate::settings) use crate::ui::setting_toggle as toggle;
