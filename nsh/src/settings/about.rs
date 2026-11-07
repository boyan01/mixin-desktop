use crate::{
    conversation::parts,
    i18n::{self, Key},
    ui::{self, Typography as _},
};
use nsh_core::prelude::*;
pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let failed = signal(false);
    scroll().flex_1().min_h_0().child(
        parts::page_body()
            .child(
                view()
                    .items_center()
                    .gap_1()
                    .py_6()
                    .child(ui::logo().size_20().mb_3())
                    .child(
                        text(move || i18n.tr("Mixin Messenger Desktop", "Mixin Messenger 桌面"))
                            .text_role(ui::TextRole::Headline),
                    )
                    .child(
                        text(env!("CARGO_PKG_VERSION"))
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::fg_secondary),
                    ),
            )
            .child(
                ui::card().children(
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
                        ui::settings_row(None, move || i18n.tr(en, zh))
                            .child(ui::chevron())
                            .on_press(move || {
                                spawn(async move {
                                    failed.set(!open_url(url).await);
                                });
                            })
                    }),
                ),
            )
            .child(
                ui::card()
                    .child(when(
                        || !cfg!(target_os = "macos"),
                        move || {
                            ui::settings_row(None, move || i18n.tr("Check for updates", "检查更新"))
                                .child(ui::chevron())
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
                        ui::settings_row(None, move || {
                            i18n.tr("Open log directory", "打开日志目录")
                        })
                        .child(ui::chevron())
                        .on_press(move || {
                            spawn(async move {
                                match mixin_desktop_api::log_directory() {
                                    Ok(path) => failed.set(open_path(path).await.is_err()),
                                    Err(_) => failed.set(true),
                                }
                            });
                        }),
                    ),
            )
            .child(when(failed, move || {
                ui::error_text(move || i18n.text(Key::CommandFailed))
            })),
    )
}
