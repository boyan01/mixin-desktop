//! The one hover, pressed, selected, disabled and focus treatment.
//!
//! Fills are translucent, so the same step reads on every surface, and a selected
//! row inside a card stacks on the card's tone.

use nsh_base::{ButtonElement, button};
use nsh_core::{element::StyleRefinement, prelude::*};

use super::tokens::{
    focus_ring, focus_ring_width, opacity_disabled, state_hover, state_pressed, state_selected,
};
use crate::motion;

/// A button with the app's interaction states and nothing else. State styles
/// added later win, so a variant restyles `hover` before `active`.
pub(super) fn pressable() -> ButtonElement {
    feedback_transition(button())
        // Hover outranks selected; a selected row keeps its fill under the pointer.
        .hover(|s| s.bg(state_hover).selected(|s| s.bg(state_selected)))
        .active(|s| s.bg(state_pressed))
        .focus(ring)
        .selected_style(|s| s.bg(state_selected))
        .disabled_style(|s| s.opacity(opacity_disabled()))
}

/// The color transition of interactive feedback.
pub(super) fn feedback_transition<T: Styled>(element: T) -> T {
    element
        .transition_colors()
        .duration(|| motion::duration(motion::FEEDBACK_MS))
        .ease_out()
}

/// The keyboard focus ring, outside the element.
pub(crate) fn ring(style: StyleRefinement) -> StyleRefinement {
    style.outline(focus_ring_width()).outline_color(focus_ring)
}

/// The focus ring of an element whose parent clips, drawn inside it.
pub(super) fn inset_ring(style: StyleRefinement) -> StyleRefinement {
    let width = focus_ring_width();
    ring(style).outline_offset(px(-width.0))
}

/// The small press response of a compact control.
pub(super) fn press_scale(style: StyleRefinement) -> StyleRefinement {
    style.scale(|| if reduce_motion() { 1.0_f32 } else { 0.985_f32 })
}
