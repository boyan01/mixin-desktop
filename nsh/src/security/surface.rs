use nsh_base::{dialog, theme};
use nsh_core::prelude::*;

use super::{Handle, passcode, tr};
use crate::{
    i18n::{self, Key},
    ui,
};

pub(crate) fn surface(handle: Handle) -> impl IntoElement {
    dynamic(move || {
        let state = handle.state.get();
        let pending = handle.lock_pending.get();
        untracked(|| match state {
            Some(_) if pending => unavailable(handle).into_element(),
            Some(state) if state.locked => lock_screen(handle).into_element(),
            Some(_) => view().hidden().into_element(),
            None => unavailable(handle).into_element(),
        })
    })
}

fn unavailable(handle: Handle) -> impl IntoElement {
    let i18n = i18n::current();
    view()
        .flex_1()
        .p_8()
        .gap_4()
        .items_center()
        .justify_center()
        .bg(move || theme().background)
        .child(ui::icon(ui::Icon::Lock).size_12())
        .child(
            text(move || {
                if handle.load_failed.get() {
                    tr(
                        i18n,
                        "Couldn’t read screen privacy settings. Account content stays hidden.",
                        "无法读取锁屏设置，账号内容将保持隐藏。",
                    )
                } else {
                    if handle.lock_pending.get() {
                        tr(i18n, "Locking Mixin…", "正在锁定 Mixin…")
                    } else {
                        tr(i18n, "Checking screen privacy…", "正在检查锁屏设置…")
                    }
                }
            })
            .text_sm()
            .text_center(),
        )
        .child(when(handle.load_failed, move || {
            ui::action(move || i18n.text(Key::Retry), move || handle.retry())
        }))
}

fn lock_screen(handle: Handle) -> impl IntoElement {
    let i18n = i18n::current();
    let passcode = signal(String::new());
    dialog()
        .animation(nsh_core::ModalAnimation::None)
        .close_on_escape(false)
        .close_on_backdrop_press(false)
        .flex_1()
        .p_8()
        .items_center()
        .justify_center()
        .bg(move || theme().background)
        .child(
            view()
                .w_80()
                .max_w(relative(1.0))
                .gap_6()
                .items_center()
                .child(
                    ui::icon(ui::Icon::Lock)
                        .size_16()
                        .text_color(move || theme().muted_foreground),
                )
                .child(
                    text(move || tr(i18n, "Unlock with passcode", "输入锁屏密码解锁"))
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(passcode::input(
                    passcode,
                    move || handle.busy.get() || handle.cooldown_seconds.get() > 0,
                    move || handle.failed.set(false),
                    move |value| {
                        passcode.set(String::new());
                        handle.unlock(value);
                    },
                ))
                .child(when(handle.busy, move || {
                    text(move || tr(i18n, "Unlocking…", "正在解锁…"))
                        .text_sm()
                        .text_color(move || theme().muted_foreground)
                }))
                .child(when(handle.failed, move || {
                    text(move || {
                        tr(
                            i18n,
                            "Couldn’t unlock. Check your passcode and try again.",
                            "无法解锁，请检查锁屏密码后重试。",
                        )
                    })
                    .text_sm()
                    .text_center()
                    .text_color(move || theme().destructive)
                }))
                .child(when(
                    move || handle.cooldown_seconds.get() > 0,
                    move || {
                        text(move || {
                            let seconds = handle.cooldown_seconds.get();
                            i18n.format(
                                "Try again in {seconds}s",
                                "请在 {seconds} 秒后重试",
                                &[("seconds", &seconds.to_string())],
                            )
                        })
                        .text_sm()
                        .text_color(move || theme().muted_foreground)
                    },
                ))
                .child(when(
                    move || {
                        handle.state.with(|state| {
                            state.as_ref().is_some_and(|state| {
                                state.biometric_enabled && state.biometrics_available
                            })
                        })
                    },
                    move || {
                        ui::action(
                            move || tr(i18n, "Use biometrics", "使用生物识别"),
                            move || handle.authenticate(),
                        )
                        .disabled(handle.busy)
                    },
                )),
        )
}
