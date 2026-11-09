//! Input controls: fields, the search pill, select, switch, the choice controls,
//! sliders and tabs.

use nsh_base::{
    ButtonElement, CheckboxElement, Input, SelectElement, SelectEntry, SelectEvent, SliderElement,
    TabElement, input, select as base_select,
};
use nsh_core::prelude::*;

use super::{
    button::command,
    icon::{Icon, icon},
    overlay::popup,
    state::{feedback_transition, inset_ring, ring},
    tokens::{
        TextRole, Typography, accent, button_height, fg_on_accent, fg_primary, fg_secondary,
        field_height, focus_ring, icon_sm, input_border, input_fill, opacity_disabled, radius_md,
        radius_sm, space_md, space_sm, space_xs, state_hover, state_pressed,
    },
};
use crate::motion;

fn text_input() -> Input {
    feedback_transition(input())
        .text_role(TextRole::Body)
        .text_color(fg_primary)
        .placeholder_color(|| color(fg_secondary()))
        .caret_color(|| color(accent()))
}

pub(crate) fn field() -> Input {
    // A soft filled plate, as in the search pill; the border only appears on focus.
    feedback_transition(text_input())
        .min_h(field_height())
        .px(space_md())
        .py(space_sm())
        .border_1()
        .border_color(move || input_border().opacity(0.0))
        .rounded(radius_md())
        .bg(input_fill)
        .hover(|s| s.border_color(input_border))
        .focus(|s| s.border_color(focus_ring))
}

/// A `field` of several lines. Size it at the call site.
pub(crate) fn textarea() -> Input {
    field().multiline(true)
}

/// The rounded frame of a search field: its icon, with `search_input` and any
/// clear command added after it.
pub(crate) fn search_pill() -> View {
    feedback_transition(view())
        .flex_1()
        .min_w_0()
        .flex_row()
        .items_center()
        .gap(space_sm())
        .h(button_height())
        .px(space_md())
        .rounded_full()
        .bg(input_fill)
        .focus_within(ring)
        .child(icon(Icon::Search).size(icon_sm()).text_color(fg_secondary))
}

/// The frameless input of a `search_pill`, which draws the fill.
pub(crate) fn search_input() -> Input {
    text_input()
        .flex_1()
        .min_w_0()
        .bg(move || input_fill().opacity(0.0))
}

/// Callers supply the items, value, change handler and trigger.
pub(crate) fn select() -> SelectElement {
    base_select()
        .on_open_change(|event: &SelectEvent| {
            if matches!(event, SelectEvent::OpenChange(true, _)) {
                dismiss_keyboard();
            }
        })
        .popup_style(popup)
        .item_class(
            "rounded-md focus:bg-state-hover selected:bg-state-selected selected:font-medium disabled:opacity-40",
        )
        .render_item(|item: &SelectEntry| {
            view()
                .px(space_md())
                .py(space_sm())
                .child(text(item.title.clone()).text_role(TextRole::Body))
        })
}

pub(crate) fn select_trigger(label: impl IntoReactive<SharedString>) -> ButtonElement {
    command()
        .rounded(radius_sm())
        .justify_start()
        .min_h(field_height())
        .border_1()
        .border_color(input_border)
        .child(
            text(label)
                .text_role(TextRole::Body)
                .flex_1()
                .min_w_0()
                .number_of_lines(1),
        )
        .child(icon(Icon::ChevronDown))
}

pub(crate) fn switch(
    label: impl Fn() -> &'static str + 'static,
    checked: impl Fn() -> bool + 'static,
    disabled: impl Fn() -> bool + 'static,
    change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    nsh_base::switch()
        .checked(checked)
        .disabled(disabled)
        .accessibility_label(move || Some(label().into()))
        .on_change(move |checked: &bool| change(*checked))
        .app_class("flex-row items-center rounded-full focus:outline-2 focus:outline-ring disabled:opacity-40")
        .track_class("w-10 h-6 rounded-full bg-muted-foreground/40 checked:bg-primary p-0.5")
        .track_style(|s| {
            s.transition_colors()
                .duration(|| motion::duration(motion::FEEDBACK_MS))
                .ease_out()
        })
        .thumb_class("w-5 h-5 rounded-full bg-primary-foreground checked:translate-x-4")
        .thumb_style(|s| {
            s.transition_transform()
                .duration(|| {
                    if reduce_motion() {
                        0.0
                    } else {
                        motion::FEEDBACK_MS
                    }
                })
                .ease_out()
        })
}

/// The label argument of a `checkbox` or `radio` that shows none: give it an
/// accessible name, or add its content after the mark.
pub(crate) const NO_LABEL: Option<fn() -> &'static str> = None;

/// What a `checkbox` and a `radio` share: a target around a mark. Size it further
/// at the call site. A macro, because base controls have these state styles as
/// inherent methods, with no trait to be generic over.
macro_rules! choice_frame {
    ($element:expr) => {
        feedback_transition($element)
            .flex_row()
            .items_center()
            .gap(space_sm())
            .px(space_xs())
            .py(space_xs())
            .rounded(radius_md())
            .hover(|s| s.bg(state_hover))
            .active(|s| s.bg(state_pressed))
            .focus(ring)
            .disabled_style(|s| s.opacity(opacity_disabled()))
    };
}

fn choice_label(label: impl Fn() -> &'static str + 'static) -> Text {
    text(label).text_role(TextRole::Body)
}

/// An independent choice.
pub(crate) fn checkbox(
    label: Option<impl Fn() -> &'static str + 'static>,
    checked: impl Fn() -> bool + Copy + 'static,
    disabled: impl Fn() -> bool + 'static,
    on_change: impl Fn(bool) + 'static,
) -> CheckboxElement {
    let frame = choice_frame!(nsh_base::checkbox())
        .checked(checked)
        .disabled(disabled)
        .on_change(move |checked: &bool| on_change(*checked))
        .indicator_class(
            "size-5 rounded-sm border border-input checked:bg-primary checked:border-primary items-center justify-center",
        )
        .indicator(
            icon(Icon::Check)
                .size(icon_sm())
                .text_color(fg_on_accent)
                .style_when(move || !checked(), |s| s.invisible()),
        );
    match label {
        Some(label) => frame.child(choice_label(label)),
        None => frame,
    }
}

/// A value on a scale. Callers supply the range, value and change handler.
pub(crate) fn slider() -> SliderElement {
    nsh_base::slider()
        .track_class("h-8 justify-center disabled:opacity-40")
        .indicator_class("h-1 rounded-full bg-muted-foreground/30")
        .range_class("rounded-full bg-primary")
        .thumb_class("size-4 rounded-full bg-primary focus:outline-2 focus:outline-ring")
}

/// A `slider` inside a message or a strip, such as a playback position.
pub(crate) fn slider_compact() -> SliderElement {
    slider()
        .track_class("h-6 justify-center disabled:opacity-40")
        .thumb_class("size-3 rounded-full bg-transparent focus:outline-2 focus:outline-ring")
}

/// Playback controls on the dark photo/video viewer surface in either appearance.
pub(crate) fn slider_media() -> SliderElement {
    slider_compact()
        .indicator_class("h-0.5 rounded-full bg-media-overlay-foreground/25")
        .range_class("rounded-full bg-media-overlay-foreground")
        .thumb_class(
            "size-3 rounded-full bg-media-overlay-foreground focus:outline-2 focus:outline-ring",
        )
}

/// A `slider` over a scale the caller draws, of which it shows only the thumb.
pub(crate) fn slider_over_scale() -> SliderElement {
    slider()
        .indicator_class("h-2 rounded-full")
        .range_class("bg-transparent")
        .thumb_class(
            "w-2 h-7 rounded-full bg-background border border-foreground/40 focus:outline-2 focus:outline-ring",
        )
}

/// A `slider` over a waveform, as tall as it, which draws nothing: the waveform shows
/// the progress by filling its bars, and the slider only takes the seeking.
pub(crate) fn slider_over_wave() -> SliderElement {
    slider()
        .track_class("h-3 justify-center disabled:opacity-40")
        .indicator_class("h-3 bg-transparent")
        .range_class("bg-transparent")
        .thumb_class("size-3 rounded-full bg-transparent focus:outline-2 focus:outline-ring")
}

/// One tab of a `tabs` row, sharing its width with the others. The accent label
/// marks the selected one; the caller changes it on press.
pub(crate) fn tab(
    label: impl Fn() -> &'static str + 'static,
    selected: impl Fn() -> bool + Copy + 'static,
) -> TabElement {
    feedback_transition(nsh_base::tab())
        .flex_1()
        .items_center()
        .justify_center()
        .hover(|s| s.bg(state_hover))
        .focus(inset_ring)
        .selected(selected)
        .child(
            text(label)
                .text_role(TextRole::Body)
                .text_color(move || if selected() { accent() } else { fg_secondary() }),
        )
}
