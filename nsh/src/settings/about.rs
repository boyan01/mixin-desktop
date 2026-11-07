use super::{body, tr};
use crate::{
    i18n::{self, Key},
    ui,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use nsh_base::theme;
use nsh_core::prelude::*;
pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let failed = signal(false);
    scroll().flex_1().min_h_0().child(
        body()
            .child(
                image(format!(
                    "data:image/png;base64,{}",
                    STANDARD.encode(include_bytes!("../../assets/logo.png"))
                ))
                .size_16(),
            )
            .child(
                text("Mixin Messenger")
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text(env!("CARGO_PKG_VERSION"))
                    .text_sm()
                    .text_color(move || theme().muted_foreground),
            )
            .children(
                [
                    ("Follow us on X", "关注 X", "https://x.com/MixinMessenger"),
                    (
                        "Follow us on Facebook",
                        "关注 Facebook",
                        "https://fb.com/MixinMessenger",
                    ),
                    ("Help center", "帮助中心", "https://support.mixin.one"),
                    (
                        "Terms of service",
                        "服务条款",
                        "https://mixin.one/pages/terms",
                    ),
                    (
                        "Privacy policy",
                        "隐私政策",
                        "https://mixin.one/pages/privacy",
                    ),
                ]
                .into_iter()
                .map(move |(en, zh, url)| {
                    ui::command()
                        .justify_start()
                        .child(text(move || tr(i18n, en, zh)))
                        .on_press(move || {
                            spawn(async move {
                                failed.set(!open_url(url).await);
                            });
                        })
                }),
            )
            .child(when(
                || !cfg!(target_os = "macos"),
                move || {
                    ui::command()
                    .justify_start()
                    .child(text(move || tr(i18n, "Check for updates", "检查更新")))
                    .on_press(move || {
                        spawn(async move {
                            let destination = if cfg!(target_os = "windows") {
                                "https://apps.microsoft.com/store/detail/mixin-desktop/9NQ6HF99B8NJ"
                            } else {
                                "https://mixin.one/messenger"
                            };
                            failed.set(!open_url(destination).await);
                        });
                    })
                },
            ))
            .child(
                ui::command()
                    .justify_start()
                    .child(text(move || tr(i18n, "Open log directory", "打开日志目录")))
                    .on_press(move || {
                        spawn(async move {
                            match mixin_desktop_api::log_directory() {
                                Ok(path) => failed.set(open_path(path).await.is_err()),
                                Err(_) => failed.set(true),
                            }
                        });
                    }),
            )
            .child(when(failed, move || {
                text(move || i18n.text(Key::CommandFailed))
                    .text_sm()
                    .text_color(move || theme().destructive)
            })),
    )
}
