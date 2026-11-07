use nsh_base::{SelectEntry, SelectItem, select, theme};
use nsh_core::prelude::*;

use super::{Handle, passcode, tr};
use crate::{
    i18n::{self, Key},
    ui,
};

pub(crate) fn settings(handle: Handle) -> impl IntoElement {
    let i18n = i18n::current();
    let setup = signal(false);
    let presence = crate::motion::Presence::modal(move || setup.get());
    effect(move || {
        if handle.enabled() {
            setup.set(false);
        }
    });
    scroll().flex_1().min_h_0().child(view().w_full().max_w(px(600.0)).p_6().gap_4()
        .child(text(move || tr(i18n, "Screen privacy", "锁屏保护")).text_lg().font_weight(FontWeight::SEMIBOLD))
        .child(text(move || tr(i18n, "A screen passcode hides account content while Mixin keeps syncing in the background.", "锁屏密码可隐藏账号内容。锁屏后，Mixin 仍会在后台同步消息。")).text_sm().text_color(move || theme().muted_foreground))
        .child(when(move || handle.state.with(|state| state.as_ref().is_some_and(|state| !state.available)), move || text(move || tr(i18n, "Screen privacy is not available on this platform yet.", "此平台暂不支持锁屏保护。")).text_sm()))
        .child(when(move || handle.state.with(|state| state.as_ref().is_some_and(|state| state.available)), move || {
            view().gap_4()
                .child(crate::ui::setting_toggle(move || tr(i18n, "Screen passcode", "锁屏密码"), move || handle.enabled(), move || handle.busy.get(), move |enabled| {
                    if enabled { setup.set(true); } else { let client = handle.client(); handle.command(async move { client.disable_passcode().await }); }
                }))
                .child(when(move || handle.enabled(), move || {
                    view().gap_4()
                        .child(text(move || tr(i18n, "Automatic lock", "自动锁屏")).text_sm())
                        .child(select().items(move || [0_u32, 1, 5, 60, 300].into_iter().map(|minutes| SelectItem::new(minutes.to_string(), duration(i18n, minutes))).collect::<Vec<_>>())
                            .value(move || handle.state.with(|state| state.as_ref().map(|state| SharedString::from(state.auto_lock_minutes.to_string()))))
                            .disabled(handle.busy).accessibility_label(move || Some(tr(i18n, "Automatic lock", "自动锁屏").into()))
                            .popup_style(ui::popup).item_class("rounded-md focus:bg-accent selected:bg-accent")
                            .render_item(|item: &SelectEntry| view().px_3().py_2().child(text(item.title.clone()).text_sm()))
                            .trigger(ui::command().justify_start().child(text(move || duration(i18n, handle.state.with(|state| state.as_ref().map_or(0, |state| state.auto_lock_minutes))))).child(ui::icon(ui::Icon::ChevronDown)))
                            .on_change(move |next: &Option<SharedString>| {
                                if let Some(minutes) = next.as_ref().and_then(|next| next.parse::<u32>().ok()) {
                                    let client = handle.client(); handle.command(async move { client.set_auto_lock_minutes(minutes).await });
                                }
                            }))
                        .child(text(move || tr(i18n, "Lock after Mixin loses focus. Returning before the timeout cancels the lock.", "Mixin 失去焦点后开始计时。在到期前返回应用，可取消自动锁屏。")).text_sm().text_color(move || theme().muted_foreground))
                        .child(crate::ui::setting_toggle(move || tr(i18n, "Biometric unlock", "生物识别解锁"),
                            move || handle.state.with(|state| state.as_ref().is_some_and(|state| state.biometric_enabled)),
                            move || handle.busy.get(),
                            move |enabled| { let client = handle.client(); handle.command(async move { client.set_biometric_enabled(enabled).await }); }))
                        .child(ui::action(move || tr(i18n, "Lock now", "立即锁屏"), move || handle.lock()).disabled(handle.busy))
                }))
        }))
        .child(when(handle.failed, move || text(move || i18n.text(Key::CommandFailed)).text_sm().text_color(move || theme().destructive)))
        .child(when(move || presence.mounted(), move || setup_dialog(handle, setup, presence))))
}

fn duration(i18n: i18n::I18n, minutes: u32) -> String {
    if minutes == 0 {
        tr(i18n, "Disabled", "关闭").into()
    } else if minutes < 60 {
        i18n.format(
            "{minutes} min",
            "{minutes} 分钟",
            &[("minutes", &minutes.to_string())],
        )
    } else {
        i18n.format(
            "{hours} h",
            "{hours} 小时",
            &[("hours", &(minutes / 60).to_string())],
        )
    }
}

fn setup_dialog(
    handle: Handle,
    setup: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let first = signal(None::<String>);
    let draft = signal(String::new());
    let mismatch = signal(false);
    let submit = move || {
        let value = draft.get_untracked();
        if !presence.interactive() || value.len() != 6 || handle.busy.get_untracked() {
            return;
        }
        match first.get_untracked() {
            None => {
                first.set(Some(value));
                draft.set(String::new());
            }
            Some(previous) if previous == value => {
                first.set(None);
                draft.set(String::new());
                let client = handle.client();
                handle.command(async move { client.set_passcode(value).await });
            }
            Some(_) => {
                first.set(None);
                draft.set(String::new());
                mismatch.set(true);
            }
        }
    };
    let card = view()
        .w_80()
        .max_w(relative(1.0))
        .p_6()
        .gap_4()
        .rounded_lg()
        .bg(move || theme().popover)
        .child(
            text(move || {
                if first.with(Option::is_some) {
                    tr(i18n, "Confirm your passcode", "确认锁屏密码")
                } else {
                    tr(i18n, "Set a six-digit passcode", "设置六位锁屏密码")
                }
            })
            .text_lg()
            .font_weight(FontWeight::SEMIBOLD),
        )
        .child(passcode::input(
            draft,
            move || handle.busy.get(),
            move || {
                mismatch.set(false);
                handle.failed.set(false);
            },
            move |_| submit(),
        ))
        .child(when(mismatch, move || {
            text(move || {
                tr(
                    i18n,
                    "The passcodes didn’t match. Try again.",
                    "两次输入的密码不一致，请重试。",
                )
            })
            .text_sm()
            .text_color(move || theme().destructive)
        }))
        .child(when(handle.failed, move || {
            text(move || i18n.text(Key::CommandFailed))
                .text_sm()
                .text_color(move || theme().destructive)
        }))
        .child(
            view().flex_row().justify_end().gap_3().child(
                ui::action(move || i18n.text(Key::Cancel), move || setup.set(false))
                    .disabled(handle.busy),
            ),
        );
    nsh_base::dialog()
        .animation(nsh_core::ModalAnimation::None)
        .close_on_escape(false)
        .close_on_backdrop_press(false)
        .on_cancel(move || {
            if presence.interactive() && !handle.busy.get_untracked() {
                setup.set(false);
            }
        })
        .flex_1()
        .child(
            view()
                .flex_1()
                .p_8()
                .items_center()
                .justify_center()
                .child(
                    presence.backdrop(
                        view()
                            .absolute()
                            .inset_0()
                            .bg(move || theme().foreground.opacity(0.2)),
                    ),
                )
                .child(presence.surface(card)),
        )
}
