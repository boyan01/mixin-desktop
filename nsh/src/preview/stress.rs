//! Long histories through production rows: a 10,000-message chat read through
//! the timeline's bounded window, and 10,000 conversations. Actions jump and
//! scroll as the app does, for captures and `nsh test --fps` readings.

use nsh_core::prelude::*;

use super::{Actions, fixtures};
use crate::{chat, ui};

/// The rows a timeline holds around its target (`CAPACITY` in the timeline).
const WINDOW: usize = 199;
const HALF: usize = WINDOW / 2;
/// A target within the window, close enough to scroll to.
const NEAR: usize = 30;

pub(super) const MESSAGE_ACTIONS: &[&str] = &["JumpNear", "JumpFar", "Scroll"];

pub(super) fn messages(actions: Actions) -> View {
    super::messages::provide_services();
    let history = stored(fixtures::messages::stress_rows());
    let window = move |start: usize| history.with(|rows| rows[start..start + WINDOW].to_vec());
    let list = chat::DetachedMessages::new(signal(window(0)));
    // Where the window starts in the history; the reader sees its first row.
    let start = stored(0_usize);
    let reset = move || {
        list.highlight.set(None);
        list.highlight_opacity.set(0.0);
        start.replace(0);
        list.messages.set(window(0));
        list.history
            .scroll_to_index(0, percentage(0.0), Points::ZERO, false);
    };
    // As the timeline jumps to row `target` of the history.
    let jump = move |target: usize| {
        let visible = start.with(|start| *start);
        let distance = (visible..visible + WINDOW)
            .contains(&target)
            .then(|| target - visible);
        match chat::Motion::for_jump(distance, true) {
            chat::Motion::Scroll => list.history.scroll_to_index(
                (target - visible) as u32,
                percentage(chat::FOCUS),
                Points::ZERO,
                true,
            ),
            chat::Motion::Place { reveal } => {
                let from = target.saturating_sub(HALF);
                start.replace(from);
                list.messages.set(window(from));
                chat::place_jump(
                    list.history,
                    list.reveal,
                    target - from,
                    chat::FOCUS,
                    reveal.then_some(target > visible),
                    || true,
                );
            }
        }
        list.highlight
            .set(history.with(|rows| Some(rows[target].message_id.clone())));
        spawn(chat::blink(list.highlight_opacity));
    };
    // A quote a screen or two up: the reader follows the scroll.
    actions.on("JumpNear", reset, move || jump(NEAR));
    // The newest message quotes one 9,000 rows back, far outside the window.
    actions.on("JumpFar", reset, move || {
        jump(fixtures::messages::STRESS_FAR_QUOTE)
    });
    actions.on("Scroll", reset, move || {
        // Through every row of the window, long texts and quotes included.
        list.history
            .scroll_to_index((WINDOW - 1) as u32, percentage(0.0), Points::ZERO, true);
    });
    super::messages::timeline_view(chat::detached_message_list(
        fixtures::USER_ID,
        list,
        true,
        fixtures::messages::mention_names(),
    ))
}

pub(super) const CONVERSATION_ACTIONS: &[&str] = &["Scroll", "JumpFar"];

pub(super) fn conversations(actions: Actions) -> View {
    super::messages::provide_services();
    let rows = signal(fixtures::conversations::stress_rows());
    let list = element_ref();
    let top = move || list.scroll_to_index(0, percentage(0.0), Points::ZERO, false);
    actions.on("Scroll", top, move || {
        list.scroll_to_index(
            (fixtures::conversations::STRESS_ROWS - 1) as u32,
            percentage(1.0),
            Points::ZERO,
            true,
        );
    });
    actions.on("JumpFar", top, move || {
        list.scroll_to_index(9_000, percentage(0.0), Points::ZERO, false);
    });
    view().flex_1().flex_row().bg(ui::surface_pane).child(
        view()
            .w(px(320.0))
            .flex_shrink_0()
            .border_r_1()
            .border_color(ui::hairline)
            .child(crate::conversation_list::detached(
                fixtures::USER_ID,
                rows,
                signal(None),
                list,
            )),
    )
}
