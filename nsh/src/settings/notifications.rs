use nsh_base::theme;
use nsh_core::prelude::*;

use super::{body, section, toggle};
use crate::{
    conversation::parts,
    i18n::{self, I18n, Key},
    notifications::Handle,
    platform::notifications::{Availability, Permission},
    ui,
};

pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let handle = use_context::<Handle>().expect("App provides native notifications");
    let available = move || handle.availability == Availability::Available;
    scroll().flex_1().min_h_0().child(
        body()
            .child(
                section(move || i18n.tr("Notifications", "通知"))
                    .child(
                        text(move || permission_summary(i18n, handle))
                            .text_sm()
                            .text_color(move || theme().muted_foreground),
                    )
                    .child(when(
                        move || {
                            available()
                                && handle.permission.get() == Some(Permission::NotDetermined)
                        },
                        move || {
                            ui::action(
                                move || i18n.tr("Allow notifications", "允许通知"),
                                move || handle.request_permission(),
                            )
                            .disabled(handle.busy)
                        },
                    ))
                    .child(when(available, move || {
                        ui::action(
                            move || i18n.tr("Refresh permission", "刷新权限状态"),
                            move || handle.refresh_permission(),
                        )
                        .disabled(handle.busy)
                    })),
            )
            .child(when(available, move || preview_setting(handle)))
            .child(when(handle.failed, move || {
                ui::error_text(move || {
                    i18n.tr(
                        "Couldn’t use notifications. Check system permissions and try again.",
                        "无法使用系统通知，请检查系统权限后重试。",
                    )
                })
            }))
            .child(when(available, system_settings)),
    )
}

fn permission_summary(i18n: I18n, handle: Handle) -> &'static str {
    if handle.availability == Availability::MissingAppBundle {
        return i18n.tr(
            "Notifications are available when you launch the packaged Mixin app.",
            "请通过打包后的 Mixin 应用使用系统通知。",
        );
    }
    if handle.availability != Availability::Available {
        return i18n.tr(
            "Notifications are not available on this platform yet.",
            "此平台暂不支持系统通知。",
        );
    }
    match handle.permission.get() {
        #[cfg(target_os = "linux")]
        Some(Permission::NotRequired) => i18n.tr(
            "No app permission prompt is required on this system.",
            "此系统无需单独授权应用通知。",
        ),
        Some(Permission::Authorized | Permission::Provisional) => {
            i18n.tr("Notifications are enabled", "已允许通知")
        }
        Some(Permission::Denied) => i18n.tr(
            "Notifications are disabled in system settings",
            "系统设置中已关闭通知",
        ),
        Some(Permission::NotDetermined) => i18n.tr(
            "Allow notifications to receive new message alerts.",
            "允许通知后，可接收新消息提醒。",
        ),
        Some(Permission::Unknown) => i18n.tr(
            "The system returned an unknown notification permission.",
            "系统返回了未知的通知权限状态。",
        ),
        None => i18n.tr("Checking notification permission…", "正在检查通知权限…"),
    }
}

fn preview_setting(handle: Handle) -> impl IntoElement {
    let i18n = i18n::current();
    view()
        .gap_3()
        .child(toggle(
            move || i18n.tr("Message preview", "消息预览"),
            move || handle.preview.get(),
            move || !handle.preview_ready.get() || handle.preview_busy.get(),
            move |value| handle.set_preview(value),
        ))
        .child(
            text(move || {
                i18n.tr(
                    "Show message content in notifications. Turn this off to keep message content private.",
                    "在通知中显示消息内容。关闭后，通知仅显示通用消息提示。",
                )
            })
            .text_sm()
            .text_color(move || theme().muted_foreground),
        )
        .child(when(handle.preview_failed, move || {
            parts::failure(
                move || i18n.text(Key::CommandFailed),
                move || handle.refresh_preview(),
            )
        }))
}

fn system_settings() -> impl IntoElement {
    let i18n = i18n::current();
    let failed = signal(false);
    view()
        .gap_2()
        .child(when(
            move || cfg!(any(target_os = "macos", target_os = "windows")),
            move || {
                ui::action(
                    move || i18n.tr("Open notification settings", "打开系统通知设置"),
                    move || {
                        spawn(async move {
                            failed.set(
                                !open_url(if cfg!(target_os = "windows") {
                                    "ms-settings:notifications"
                                } else {
                                    "x-apple.systempreferences:com.apple.preference.notifications"
                                })
                                .await,
                            );
                        });
                    },
                )
            },
        ))
        .child(when(failed, move || {
            ui::error_text(move || i18n.text(Key::CommandFailed))
        }))
}
