use nsh_base::{SelectItem, theme};
use nsh_core::prelude::*;

use super::{Handle, passcode};
use crate::{
    conversation::parts::{self, Prompt, confirm},
    i18n::{self, Key},
    settings::{body, section},
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
    let available = move |expected: bool| {
        handle.state.with(|state| {
            state
                .as_ref()
                .is_some_and(|state| state.available == expected)
        })
    };
    scroll().flex_1().min_h_0().child(
        body()
            .child(
                section(move || i18n.tr("Screen privacy", "锁屏保护")).child(
                    text(move || {
                        i18n.tr(
                            "A screen passcode hides account content while Mixin keeps syncing in the background.",
                            "锁屏密码可隐藏账号内容。锁屏后，Mixin 仍会在后台同步消息。",
                        )
                    })
                    .text_sm()
                    .text_color(move || theme().muted_foreground),
                ),
            )
            .child(when(
                move || available(false),
                move || {
                    text(move || {
                        i18n.tr(
                            "Screen privacy is not available on this platform yet.",
                            "此平台暂不支持锁屏保护。",
                        )
                    })
                    .text_sm()
                },
            ))
            .child(when(
                move || available(true),
                move || {
                    view()
                        .gap_4()
                        .child(ui::setting_toggle(
                            move || i18n.tr("Screen passcode", "锁屏密码"),
                            move || handle.enabled(),
                            move || handle.busy.get(),
                            move |enabled| {
                                if enabled {
                                    setup.set(true);
                                } else {
                                    disable_passcode(handle);
                                }
                            },
                        ))
                        .child(when(move || handle.enabled(), move || lock_options(handle)))
                },
            ))
            .child(when(handle.failed, move || {
                ui::error_text(move || i18n.text(Key::CommandFailed))
            }))
            .child(when(
                move || presence.mounted(),
                move || setup_dialog(handle, setup, presence),
            )),
    )
}

fn disable_passcode(handle: Handle) {
    let i18n = i18n::current();
    let client = handle.client();
    spawn(async move {
        let prompt = Prompt::new(
            i18n.tr("Turn off screen passcode?", "关闭锁屏密码？"),
            i18n.tr(
                "Account content will no longer be hidden when Mixin locks.",
                "关闭后，将不再隐藏账号内容。",
            ),
            i18n.tr("Turn off", "关闭"),
        );
        if confirm(prompt).await {
            handle.command(async move { client.disable_passcode().await });
        }
    });
}

fn lock_options(handle: Handle) -> impl IntoElement {
    let i18n = i18n::current();
    let minutes = move || {
        handle
            .state
            .with(|state| state.as_ref().map_or(0, |state| state.auto_lock_minutes))
    };
    view()
        .gap_4()
        .child(
            section(move || i18n.tr("Automatic lock", "自动锁屏"))
                .child(
                    parts::option_select()
                        .items(move || {
                            [0_u32, 1, 5, 60, 300]
                                .into_iter()
                                .map(|minutes| {
                                    SelectItem::new(minutes.to_string(), duration(i18n, minutes))
                                })
                                .collect::<Vec<_>>()
                        })
                        .value(move || Some(SharedString::from(minutes().to_string())))
                        .disabled(handle.busy)
                        .accessibility_label(move || {
                            Some(i18n.tr("Automatic lock", "自动锁屏").into())
                        })
                        .trigger(parts::select_trigger(move || duration(i18n, minutes())))
                        .on_change(move |next: &Option<SharedString>| {
                            if let Some(minutes) =
                                next.as_ref().and_then(|next| next.parse::<u32>().ok())
                            {
                                let client = handle.client();
                                handle.command(async move {
                                    client.set_auto_lock_minutes(minutes).await
                                });
                            }
                        }),
                )
                .child(
                    text(move || {
                        i18n.tr(
                            "Lock after Mixin loses focus. Returning before the timeout cancels the lock.",
                            "Mixin 失去焦点后开始计时。在到期前返回应用，可取消自动锁屏。",
                        )
                    })
                    .text_sm()
                    .text_color(move || theme().muted_foreground),
                ),
        )
        .child(ui::setting_toggle(
            move || i18n.tr("Biometric unlock", "生物识别解锁"),
            move || {
                handle
                    .state
                    .with(|state| state.as_ref().is_some_and(|state| state.biometric_enabled))
            },
            move || handle.busy.get(),
            move |enabled| {
                let client = handle.client();
                handle.command(async move { client.set_biometric_enabled(enabled).await });
            },
        ))
        .child(
            ui::action(move || i18n.tr("Lock now", "立即锁屏"), move || handle.lock())
                .disabled(handle.busy),
        )
}

fn duration(i18n: i18n::I18n, minutes: u32) -> String {
    if minutes == 0 {
        i18n.tr("Disabled", "关闭").into()
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
    let cancel = move || {
        if !handle.busy.get_untracked() {
            setup.set(false);
        }
    };
    ui::modal(
        presence,
        cancel,
        ui::modal_card()
            .child(ui::modal_title(move || {
                if first.with(Option::is_some) {
                    i18n.tr("Confirm your passcode", "确认锁屏密码")
                } else {
                    i18n.tr("Set a six-digit passcode", "设置六位锁屏密码")
                }
            }))
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
                ui::error_text(move || {
                    i18n.tr(
                        "The passcodes didn’t match. Try again.",
                        "两次输入的密码不一致，请重试。",
                    )
                })
            }))
            .child(when(handle.failed, move || {
                ui::error_text(move || i18n.text(Key::CommandFailed))
            }))
            .child(
                parts::dialog_actions().child(
                    ui::action(move || i18n.text(Key::Cancel), cancel).disabled(handle.busy),
                ),
            ),
    )
}
