//! Quiet content: counts, headings, dividers, and error, busy and empty states.

use nsh_core::{element::ImageSrc, prelude::*};

use super::{
    icon::logo,
    tokens::{
        TextRole, Typography, accent, badge_height, badge_min_width, badge_quiet, fg_on_accent,
        fg_primary, fg_secondary, hairline, hairline_width, icon_sm, space_md, space_sm, space_xl,
        space_xs, state_selected,
    },
};

/// The count pill of list rows and the navigation rail. Quiet when not emphasized.
pub(crate) fn count_pill(
    label: impl Fn() -> String + 'static,
    emphasized: impl Fn() -> bool + Copy + 'static,
) -> View {
    view()
        .min_w(badge_min_width())
        .h(badge_height())
        .px(space_xs())
        .flex_shrink_0()
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || {
            if emphasized() {
                accent()
            } else {
                badge_quiet()
            }
        })
        .child(
            text(label)
                .text_role(TextRole::Caption)
                .tabular_nums()
                .text_color(move || {
                    if emphasized() {
                        fg_on_accent()
                    } else {
                        fg_primary()
                    }
                }),
        )
}

/// The heading of a group, lined up with the labels of its rows.
pub(crate) fn section_label(title: impl Fn() -> &'static str + 'static) -> Text {
    text(title)
        .px(space_md())
        .text_role(TextRole::Body)
        .text_color(fg_secondary)
        .role(AccessibilityRole::Header)
}

/// A headed group of cards.
pub(crate) fn section(title: impl Fn() -> &'static str + 'static) -> View {
    view().gap(space_md()).child(section_label(title))
}

/// A boundary between groups in one pane.
pub(crate) fn divider() -> View {
    view()
        .h(hairline_width())
        .mx(space_sm())
        .my(space_sm())
        .bg(hairline)
}

/// A recoverable failure, stated quietly next to what it affects.
pub(crate) fn error_text(message: impl Fn() -> &'static str + 'static) -> Text {
    text(message)
        .text_role(TextRole::Body)
        .text_color(fg_secondary)
}

/// A quiet message that fills an empty pane.
pub(crate) fn placeholder(message: impl Fn() -> &'static str + 'static) -> View {
    view()
        .flex_1()
        .min_h_0()
        .p(space_xl())
        .items_center()
        .justify_center()
        .child(
            text(message)
                .text_role(TextRole::Body)
                .text_center()
                .text_color(fg_secondary),
        )
}

/// Mount only while actual work is pending. Never manufacture a percentage.
pub(crate) fn busy_indicator(
    busy: impl Fn() -> bool + 'static,
    label: impl Fn() -> SharedString + 'static,
) -> impl IntoElement {
    let label = stored(label);
    when(busy, move || {
        svg(ImageSrc::svg_markup("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'><circle cx='12' cy='12' r='9' opacity='.2'/><path d='M12 3a9 9 0 0 1 9 9'/></svg>"))
            .size(icon_sm())
            .flex_shrink_0()
            .role(AccessibilityRole::Progressbar)
            .label(move || label.with(|label| label()))
            .style_with(|s| if reduce_motion() { s.animate_none() } else { s.animate_spin() })
    })
}

/// A state with nothing to show yet: the mark, what is empty and what to do next.
pub(crate) fn empty(
    title: impl IntoReactive<SharedString>,
    body: impl IntoReactive<SharedString>,
) -> View {
    view()
        .flex_1()
        .min_h_0()
        .p_8()
        .gap(space_md())
        .items_center()
        .justify_center()
        .child(
            view()
                .size_16()
                .rounded_2xl()
                .bg(state_selected)
                .items_center()
                .justify_center()
                .text_color(fg_secondary)
                .child(logo().size_12().opacity(0.65)),
        )
        .child(text(title).text_role(TextRole::Headline).text_center())
        .child(
            text(body)
                .max_w_80()
                .text_role(TextRole::Body)
                .text_center()
                .text_color(fg_secondary),
        )
}
