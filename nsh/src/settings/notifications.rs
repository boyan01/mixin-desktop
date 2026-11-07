use nsh_base::theme;
use nsh_core::prelude::*;

use super::{body, section, toggle, tr};
use crate::{
    i18n::{self, Key},
    notifications::Handle,
    platform::notifications::{Availability, Permission},
    ui,
};

pub(super) fn screen() -> impl IntoElement {
    let i18n = i18n::current();
    let handle = use_context::<Handle>().expect("App provides native notifications");
    scroll().flex_1().min_h_0().child(body()
        .child(section(move || tr(i18n, "Notifications", "通知"))
            .child(text(move || {
                if handle.availability == Availability::MissingAppBundle {
                    return tr(i18n, "Notifications are available when you launch the packaged Mixin app.", "请通过打包后的 Mixin 应用使用系统通知。");
                }
                if handle.availability != Availability::Available {
                    return tr(i18n, "Notifications are not available on this platform yet.", "此平台暂不支持系统通知。");
                }
                match handle.permission.get() {
                    #[cfg(target_os = "linux")]
                    Some(Permission::NotRequired) => tr(i18n, "No app permission prompt is required on this system.", "此系统无需单独授权应用通知。"),
                    Some(Permission::Authorized | Permission::Provisional) => tr(i18n, "Notifications are enabled", "已允许通知"),
                    Some(Permission::Denied) => tr(i18n, "Notifications are disabled in system settings", "系统设置中已关闭通知"),
                    Some(Permission::NotDetermined) => tr(i18n, "Allow notifications to receive new message alerts.", "允许通知后，可接收新消息提醒。"),
                    Some(Permission::Unknown) => tr(i18n, "The system returned an unknown notification permission.", "系统返回了未知的通知权限状态。"),
                    None => tr(i18n, "Checking notification permission…", "正在检查通知权限…"),
                }
            }).text_sm().text_color(move || theme().muted_foreground))
            .child(when(move || handle.availability == Availability::Available && handle.permission.get() == Some(Permission::NotDetermined), move || {
                ui::action(move || tr(i18n, "Allow notifications", "允许通知"), move || handle.request_permission()).disabled(handle.busy)
            }))
            .child(when(move || handle.availability == Availability::Available, move || {
                ui::action(move || tr(i18n, "Refresh permission", "刷新权限状态"), move || handle.refresh_permission()).disabled(handle.busy)
            })))
        .child(when(move || handle.availability == Availability::Available, move || {
            view().gap_3()
                .child(toggle(move || tr(i18n, "Message preview", "消息预览"), move || handle.preview.get(),
                    move || !handle.preview_ready.get() || handle.preview_busy.get(), move |value| handle.set_preview(value)))
                .child(text(move || tr(i18n, "Show message content in notifications. Turn this off to keep message content private.", "在通知中显示消息内容。关闭后，通知仅显示通用消息提示。")).text_sm().text_color(move || theme().muted_foreground))
                .child(when(handle.preview_failed, move || view().gap_2()
                    .child(text(move || i18n.text(Key::CommandFailed)).text_sm().text_color(move || theme().destructive))
                    .child(ui::action(move || i18n.text(Key::Retry), move || handle.refresh_preview()))))
        }))
        .child(when(handle.failed, move || text(move || tr(i18n, "Couldn’t use notifications. Check system permissions and try again.", "无法使用系统通知，请检查系统权限后重试。")).text_sm().text_color(move || theme().destructive)))
        .child(when(move || handle.availability == Availability::Available, system_settings)))
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
                    move || tr(i18n, "Open notification settings", "打开系统通知设置"),
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
            text(move || i18n.text(Key::CommandFailed))
                .text_sm()
                .text_color(move || theme().destructive)
        }))
}
