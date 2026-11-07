use nsh_base::{OtpInput, OtpInputApi, OtpInputCompleteEvent, otp_input};
use nsh_core::prelude::*;

use crate::i18n;

pub(super) fn input(
    value: Signal<String>,
    disabled: impl Fn() -> bool + 'static,
    on_change: impl Fn() + 'static,
    on_complete: impl Fn(String) + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let input = component_ref::<OtpInput>();
    let focused = signal(false);
    view()
        .on_layout(move |_: &Bounds<Points>| {
            if !focused.get_untracked() && let Some(api) = input.api::<OtpInputApi>() {
                api.focus(); focused.set(true);
            }
        })
        .child(otp_input().component_ref(&input).length(6).masked(true).disabled(disabled)
        .value(move || Some(SharedString::from(value.get())))
        .accessibility_label(move || Some(i18n.tr("Six-digit screen passcode", "六位锁屏密码").into()))
        .group_class("flex-row items-center gap-6 min-h-11")
        .cell_class("size-3 rounded-full border border-foreground text-foreground data-filled:bg-foreground focus:outline-2 focus:outline-ring disabled:opacity-40")
        .caret_class("hidden")
        .on_change(move |next: &SharedString| { value.set(next.to_string()); on_change(); })
        .on_complete(move |event: &OtpInputCompleteEvent| on_complete(event.value.to_string())))
}
