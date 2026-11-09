//! View-owned bounded history, edge loading and message jumps.
mod query;

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use futures::StreamExt as _;
use mixin_desktop_api::{ConversationListData, MessageActions, MessageListView};
use nsh_core::{element::ImageSrc, prelude::*};

use super::message_actions::Actions;
use super::message_appearance::floating_disc;
use crate::presentation::message_window::splice_phase;
use crate::{
    i18n::{I18n, Key},
    runtime,
    ui::{self, Typography},
};
use query::{Load, Window};

/// Coalesces a sync burst into one window read; short enough to read as instant.
const REFRESH_INTERVAL: Duration = Duration::from_millis(60);
/// The most rows a jump scrolls through; past them a reader cannot follow the
/// motion, so the target is placed at once and its rows ease in instead.
const SCROLL_JUMP_ROWS: usize = 40;
const REVEAL_MS: f32 = 220.0;
const REVEAL_OFFSET: f32 = 24.0;
/// Where a jump target lands. NSH mirrors view_position for an inverted list:
/// 0.8 puts the target near Flutter's 0.2 focus anchor measured from the top.
pub(crate) const FOCUS: f32 = 0.8;

/// How a jump brings its target row into view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Motion {
    /// Scrolls there through the rows in between.
    Scroll,
    /// Shows the target at once; `reveal` eases the rows in when they replace
    /// rows the reader was looking at.
    Place { reveal: bool },
}

impl Motion {
    /// `distance` is how many rows separate the target from the visible row,
    /// `None` when the target is outside the rows the list holds.
    pub(crate) fn for_jump(distance: Option<usize>, visible: bool) -> Self {
        match distance {
            Some(distance) if distance <= SCROLL_JUMP_ROWS => Self::Scroll,
            _ => Self::Place { reveal: visible },
        }
    }
}

/// Hides the list while a jump places it, then shows it: at once, or easing in
/// from the side its rows came from when they replace rows the reader saw.
#[derive(Clone, Copy)]
pub(crate) struct Reveal {
    opacity: nsh_core::element::AnimatedValue,
    offset: nsh_core::element::AnimatedValue,
}

impl Reveal {
    pub(crate) fn new() -> Self {
        Self {
            opacity: animated_value(1.0),
            offset: animated_value(0.0),
        }
    }

    fn hide(self) {
        self.opacity.set(0.0);
    }

    /// `older`: the rows came from above, as older messages sit in a chat.
    fn show(self, from: Option<bool>) {
        let Some(older) = from else {
            self.offset.set(0.0);
            self.opacity.set(1.0);
            return;
        };
        let ms = crate::motion::duration(REVEAL_MS);
        self.offset.set(if reduce_motion() {
            0.0
        } else if older {
            -REVEAL_OFFSET
        } else {
            REVEAL_OFFSET
        });
        let _ = self
            .opacity
            .timing(1.0, ms, nsh_core::animation::Easing::EaseOut);
        let _ = self
            .offset
            .timing(0.0, ms, nsh_core::animation::Easing::EaseOut);
    }

    pub(crate) fn surface(self, view: View) -> View {
        view.bind_opacity(&self.opacity)
            .bind_transform(nsh_core::TransformKind::TranslateY, &self.offset)
    }
}

/// Shows row `index` of rows set in this turn without scrolling through the
/// rows before it. A scroll command sent with the rows would apply to the
/// rows they replace, so the list stays hidden for the commit that lays them
/// out at their default position, is placed after it, and shows once placed.
/// `position` is where in the viewport the row lands (`FOCUS` for a jump),
/// `from` how it shows (`Reveal::show`), `current` whether the jump still
/// stands when the rows are laid out.
pub(crate) fn place(
    history: ElementRef,
    reveal: Reveal,
    index: usize,
    position: f32,
    from: Option<bool>,
    current: impl Fn() -> bool + 'static,
) {
    reveal.hide();
    after_answer(move || {
        if current() {
            history.scroll_to_index(index as u32, percentage(position), Points::ZERO, false);
        }
        // Shown even when superseded: a later jump may not hide it again.
        after_answer(move || reveal.show(from));
    });
}

/// How a read's rows reach the list, across its splice phases.
#[derive(Clone, Copy)]
struct Replacement {
    ticket: u64,
    tail: bool,
    /// The position revision to scroll to the latest row under.
    scroll_latest: Option<u64>,
    /// The reader asked for the latest rows: move there as a jump does.
    animate: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Timeline {
    conversation: Stored<String>,
    actions: Actions,
    messages: Signal<Vec<MessageListView>>,
    history: ElementRef,
    target: Signal<Option<String>>,
    return_target: Signal<Option<String>>,
    request: Signal<Option<(u64, Load)>>,
    revision: Signal<u64>,
    position_revision: Signal<u64>,
    last_load: Stored<Load>,
    dirty: Signal<bool>,
    deferred_edge: Stored<Option<bool>>,
    busy: Signal<bool>,
    pub loading: Signal<bool>,
    pub failed: Signal<bool>,
    missing_target: Signal<bool>,
    pub highlight: Signal<Option<String>>,
    pub highlight_opacity: nsh_core::element::AnimatedValue,
    pub has_newer: Signal<bool>,
    pub has_older: Signal<bool>,
    visible_anchor: Signal<Option<String>>,
    following_tail: Signal<bool>,
    show_latest: Signal<bool>,
    pub reveal: Reveal,
}

impl Timeline {
    /// `target` notifies on every write: writing it, even with the same value,
    /// is how a surface asks for a jump (`Some`) or the latest rows (`None`).
    pub fn mount(
        conversation: ConversationListData,
        target: Signal<Option<String>>,
        history: ElementRef,
        messages: Signal<Vec<MessageListView>>,
        actions: Actions,
    ) -> Self {
        let state = Self {
            conversation: stored(conversation.conversation_id),
            actions,
            messages,
            history,
            target,
            return_target: use_context::<crate::navigation::ChatNavigation>()
                .expect("Home provides chat navigation")
                .return_target,
            request: signal_with(None, SignalEquals::Never),
            revision: signal(0),
            position_revision: signal(0),
            last_load: stored(Load::Latest),
            dirty: signal(false),
            deferred_edge: stored(None),
            busy: signal(false),
            loading: signal(true),
            failed: signal(false),
            missing_target: signal(false),
            highlight: signal_with(None, SignalEquals::Never),
            highlight_opacity: animated_value(0.0),
            has_newer: signal(false),
            has_older: signal(false),
            visible_anchor: signal(None),
            following_tail: signal(true),
            show_latest: signal(false),
            reveal: Reveal::new(),
        };
        effect(move || {
            let Some((ticket, request)) = state.request.get() else {
                return;
            };
            let previous = Window {
                rows: messages.get_untracked(),
                older: state.has_older.get_untracked(),
                newer: state.has_newer.get_untracked(),
                missing: false,
                scroll: None,
                tail: false,
                deleted: Default::default(),
            };
            let client = actions.account.with(|account| account.clone());
            let id = state.conversation.with(|id| id.clone());
            let highlight = matches!(request, Load::Jump(_));
            let explicit = matches!(
                request,
                Load::Latest | Load::Jump(_) | Load::InitialUnread(_)
            );
            spawn(async move {
                let result = runtime::run(async move {
                    let window = query::load(client.clone(), id.clone(), request, previous).await?;
                    let ids = window
                        .rows
                        .iter()
                        .map(|row| row.message_id.clone())
                        .collect();
                    let permissions = client.message().message_actions(id, ids).await?;
                    Ok((window, permissions))
                })
                .await;
                if state.revision.get_untracked() != ticket {
                    return;
                }
                match result {
                    Ok((window, permissions)) => {
                        if state.apply(window, permissions, highlight, explicit, ticket) {
                            return;
                        }
                    }
                    Err(_) => state.failed.set(true),
                }
                state.complete();
            });
        });
        effect(move || {
            if state.highlight.get().is_some() {
                state.highlight_opacity.set(1.0);
                let finished = state.highlight_opacity.timing(
                    0.0,
                    crate::motion::duration(1200.0),
                    nsh_core::animation::Easing::EaseOut,
                );
                spawn(async move {
                    if finished.await {
                        state.highlight.set(None);
                    }
                });
            }
        });
        let first = stored(true);
        effect(move || {
            let requested = target.get();
            let initial = first.with(|first| std::mem::replace(first, false));
            let load = if let Some(id) = requested {
                Load::Jump(id)
            } else if initial && conversation.unseen_count > 0 {
                conversation
                    .last_read_message_id
                    .clone()
                    .map(Load::InitialUnread)
                    .unwrap_or(Load::Latest)
            } else {
                Load::Latest
            };
            state.schedule(load, true);
        });
        spawn(async move {
            let client = actions.account.with(|account| account.clone());
            let conversation = state.conversation.with(|id| id.clone());
            let mut changes = Box::pin(runtime::throttled(
                client.message_changes(Some(conversation)),
                REFRESH_INTERVAL,
            ));
            while changes.next().await.is_some() {
                state.refresh();
            }
        });
        effect(move || {
            if !state.loading.get()
                && !state.has_newer.get()
                && !state.show_latest.get()
                && !state.missing_target.get()
            {
                state.return_target.set(None);
            }
        });
        state
    }

    fn schedule(self, load: Load, replace: bool) {
        if self.busy.get_untracked() && !replace {
            self.dirty.set(true);
            return;
        }
        self.last_load.replace(load.clone());
        if matches!(&load, Load::Latest) {
            self.highlight.set(None);
        }
        if replace {
            self.deferred_edge.replace(None);
            self.position_revision
                .update(|revision| *revision = revision.wrapping_add(1));
            // A jump within the rows may not scroll at all, so only the next
            // scroll report can say where the reader is.
            if !self.in_window(&load) {
                let tail = matches!(&load, Load::Latest);
                self.following_tail.set(tail);
                self.show_latest.set(!tail);
            }
        }
        self.revision
            .update(|revision| *revision = revision.wrapping_add(1));
        batch(|| {
            self.busy.set(true);
            self.loading
                .set(self.messages.with_untracked(Vec::is_empty));
            self.failed.set(false);
            self.missing_target.set(false);
            self.request
                .set(Some((self.revision.get_untracked(), load)));
        });
    }

    fn in_window(self, load: &Load) -> bool {
        let (Load::Jump(id) | Load::InitialUnread(id)) = load else {
            return false;
        };
        self.messages
            .with_untracked(|rows| rows.iter().any(|row| &row.message_id == id))
    }

    fn refresh(self) {
        self.schedule(
            Load::Refresh {
                anchor: self.visible_anchor.get_untracked(),
                follow_tail: !self.has_newer.get_untracked() && self.following_tail.get_untracked(),
            },
            false,
        );
    }

    fn apply(
        self,
        window: Window,
        permissions: Vec<MessageActions>,
        highlight: bool,
        explicit: bool,
        ticket: u64,
    ) -> bool {
        let tail = window.tail && (explicit || self.following_tail.get_untracked());
        if !explicit
            && !tail
            && let Some(anchor) = self.visible_anchor.get_untracked()
            && self
                .messages
                .with_untracked(|rows| rows.iter().any(|row| row.message_id == anchor))
            && !window.rows.iter().any(|row| row.message_id == anchor)
            && !window.deleted.contains(&anchor)
        {
            // A user can move to the opposite end while a page is loading.
            // Do not trim their newly visible row using the old request's view.
            self.dirty.set(true);
            return false;
        }
        let position_revision = self.position_revision.get_untracked();
        let scroll = window.scroll;
        batch(|| {
            self.actions.policies.update(|cache: &mut HashMap<_, _>| {
                cache.retain(|id, _| self.actions.selected.with_untracked(|ids| ids.contains(id)));
                cache.extend(
                    permissions
                        .into_iter()
                        .map(|policy| (policy.message_id.clone(), policy)),
                );
            });
            self.has_older.set(window.older);
            self.has_newer.set(window.newer);
            self.missing_target.set(window.missing);
        });
        if let Some(id) = scroll {
            let (motion, older) = self.messages.with_untracked(|rows| {
                let visible = self
                    .visible_anchor
                    .with_untracked(|anchor| {
                        rows.iter()
                            .position(|row| Some(&row.message_id) == anchor.as_ref())
                    })
                    .or((!rows.is_empty()).then_some(0));
                let target = rows.iter().position(|row| row.message_id == id);
                let distance = target
                    .zip(visible)
                    .map(|(target, visible)| target.abs_diff(visible));
                // The list stays hidden until its first rows: nothing to reveal.
                let motion = Motion::for_jump(distance, !self.loading.get_untracked());
                // The list is newest first; across windows, compare times.
                let older = match (target, visible) {
                    (Some(target), Some(visible)) => target > visible,
                    _ => {
                        let at = |rows: &[MessageListView]| {
                            rows.iter()
                                .find(|row| row.message_id == id)
                                .map(|row| row.created_at_micros)
                        };
                        let shown = visible.and_then(|visible| rows.get(visible));
                        match (at(&window.rows), shown) {
                            (Some(target), Some(shown)) => target <= shown.created_at_micros,
                            _ => true,
                        }
                    }
                };
                (motion, older)
            });
            let index = window.rows.iter().position(|row| row.message_id == id);
            self.messages.set(window.rows);
            self.visible_anchor.set(Some(id.clone()));
            if highlight {
                self.highlight.set(Some(id.clone()));
            }
            match (motion, index) {
                (Motion::Place { reveal }, Some(index)) => {
                    let position = self.position_revision;
                    place(
                        self.history,
                        self.reveal,
                        index,
                        FOCUS,
                        reveal.then_some(older),
                        move || position.get_untracked() == position_revision,
                    );
                }
                _ => self.scroll_to(id, position_revision),
            }
            return false;
        }
        // A status update must not pull a reader who is a few points up.
        let moved = explicit
            || self.messages.with_untracked(|rows| {
                rows.first().map(|row| &row.message_id)
                    != window.rows.first().map(|row| &row.message_id)
            });
        self.apply_rows(
            window.rows,
            window.deleted,
            Replacement {
                ticket,
                tail,
                scroll_latest: (tail && moved).then_some(position_revision),
                animate: explicit && !self.loading.get_untracked(),
            },
        )
    }

    fn apply_rows(
        self,
        rows: Vec<MessageListView>,
        deleted: HashSet<String>,
        replacement: Replacement,
    ) -> bool {
        let previous = self.messages.get_untracked();
        let anchor = if replacement.tail {
            previous.first().map(|row| row.message_id.clone())
        } else {
            self.visible_anchor.get_untracked()
        };
        if !replacement.tail
            && let Some(anchor) = anchor.as_deref()
            && previous.iter().any(|row| row.message_id == anchor)
            && !rows.iter().any(|row| row.message_id == anchor)
            && !deleted.contains(anchor)
        {
            self.dirty.set(true);
            return false;
        }
        if let Some(phase) = splice_phase(&previous, &rows, anchor.as_deref()) {
            self.messages.set(phase);
            // NSH finds a single splice by common prefix/suffix. Updating
            // both ends at once would discard their shared middle, including
            // its measured anchor. Each answer keeps that row at a common end.
            after_answer(move || {
                if self.revision.get_untracked() == replacement.ticket
                    && !self.apply_rows(rows, deleted, replacement)
                {
                    self.complete();
                }
            });
            return true;
        }
        // How far the reader is from the newest row, if it is in these rows.
        let distance = replacement.animate.then(|| {
            self.visible_anchor.with_untracked(|anchor| {
                rows.iter()
                    .position(|row| Some(&row.message_id) == anchor.as_ref())
            })
        });
        self.messages.set(rows);
        if let Some(revision) = replacement.scroll_latest {
            let current = move || self.position_revision.get_untracked() == revision;
            match distance.map(|distance| Motion::for_jump(distance, true)) {
                // Newer rows replace the ones in view: they come up from below.
                Some(Motion::Place { .. }) => {
                    place(self.history, self.reveal, 0, 0.0, Some(false), current);
                }
                motion => after_answer(move || {
                    if current() {
                        let animated = motion.is_some() && !reduce_motion();
                        self.history
                            .scroll_to_index(0, percentage(0.0), Points::ZERO, animated);
                    }
                }),
            }
        }
        false
    }

    fn complete(self) {
        batch(|| {
            self.loading.set(false);
            self.busy.set(false);
        });
        // A native edge callback fires once per approach. A read in flight
        // must not lose it, nor an independent account update.
        if let Some(older) = self.deferred_edge.with(|edge| edge.take()) {
            if older {
                self.older();
            } else {
                self.newer();
            }
            if self.busy.get_untracked() {
                return;
            }
        }
        if self.dirty.get_untracked() {
            self.dirty.set(false);
            self.refresh();
        }
    }

    fn scroll_to(self, id: String, ticket: u64) {
        after_answer(move || {
            if self.position_revision.get_untracked() != ticket {
                return;
            }
            if let Some(index) = self
                .messages
                .with_untracked(|rows| rows.iter().position(|row| row.message_id == id))
            {
                // The anchor `place` uses.
                self.history.scroll_to_index(
                    index as u32,
                    percentage(FOCUS),
                    Points::ZERO,
                    !reduce_motion(),
                );
            }
        });
    }

    pub fn viewable(self, event: &ViewableItems) {
        let anchor = event.viewable.and_then(|(first, _)| {
            self.messages
                .with_untracked(|rows| rows.get(first).map(|row| row.message_id.clone()))
        });
        if anchor.is_some() {
            self.visible_anchor.set(anchor);
        }
    }

    pub fn scrolled(self, event: &ScrollEvent) {
        let distance =
            (event.content_size.height.0 - event.size.height.0 - event.offset.y.0).max(0.0);
        self.following_tail.set(distance <= 50.0);
        self.show_latest.set(distance > 96.0);
    }

    pub fn older(self) {
        if !self.has_older.get_untracked() {
            return;
        }
        if self.busy.get_untracked() {
            self.deferred_edge.replace(Some(true));
            return;
        }
        self.schedule(Load::Older, false);
    }
    pub fn newer(self) {
        if !self.has_newer.get_untracked() {
            return;
        }
        if self.busy.get_untracked() {
            self.deferred_edge.replace(Some(false));
            return;
        }
        self.schedule(Load::Newer, false);
    }
    fn latest(self) {
        self.highlight.set(None);
        let source = self.return_target.get_untracked();
        self.return_target.set(None);
        self.target.set(source);
    }

    /// Failures and missing targets stay in the layout: they need an answer.
    pub fn controls(self, i18n: I18n) -> impl IntoElement {
        view()
            .flex_row()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap_2()
            .child(when(self.missing_target, move || {
                text(move || i18n.tr("This message is no longer available.", "此消息已不可用。"))
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::fg_secondary)
            }))
            .child(when(self.failed, move || {
                load_error(move || {
                    let load = self.last_load.with(|load| load.clone());
                    if matches!(load, Load::Refresh { .. }) {
                        self.refresh();
                    } else {
                        self.schedule(load, true);
                    }
                })
            }))
    }

    /// Floats over the rows: a page load must not resize the list it extends,
    /// and a background refresh has nothing to report.
    pub fn paging_indicator(self, i18n: I18n) -> impl IntoElement {
        view()
            .absolute()
            .top_2()
            .left_0()
            .right_0()
            .items_center()
            .pointer_events(PointerEvents::None)
            .child(when(
                move || {
                    self.busy.get()
                        && !self.loading.get()
                        && self
                            .last_load
                            .with(|load| matches!(load, Load::Older | Load::Newer | Load::Jump(_)))
                },
                move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .rounded(ui::radius_full())
                        .shadow_md()
                        .bg(ui::bubble_in)
                        .child(ui::busy_indicator(
                            || true,
                            move || i18n.text(Key::LoadingMessages).into(),
                        ))
                        .child(
                            text(move || i18n.text(Key::LoadingMessages))
                                .text_role(ui::TextRole::Caption)
                                .text_color(ui::fg_secondary),
                        )
                },
            ))
    }

    pub fn jump_button(self, i18n: I18n) -> impl IntoElement {
        // The button slides in and out instead of popping, but never blocks input while leaving.
        let presence = crate::motion::Presence::bottom(move || {
            !self.loading.get()
                && (self.has_newer.get() || self.show_latest.get() || self.missing_target.get())
        });
        when(
            move || presence.mounted(),
            move || {
                let arrow = include_str!("../../assets/jump_current_arrow.svg")
                    .replace("fill=\"white\"", "fill=\"currentColor\"");
                presence.surface(floating_disc().shadow_md()).child(
                    ui::round_button()
                        .size_10()
                        .accessibility_label(move || {
                            Some(if self.return_target.with(Option::is_some) {
                                i18n.tr("Return to message", "返回消息").into()
                            } else {
                                i18n.text(Key::Latest).into()
                            })
                        })
                        .on_press(move || self.latest())
                        .child(
                            svg(ImageSrc::svg_markup(&arrow))
                                .size(ui::icon_sm())
                                .text_color(ui::fg_primary),
                        ),
                )
            },
        )
    }
}

pub(crate) fn load_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .flex_row()
        .gap_2()
        .child(ui::error_text(move || i18n.text(Key::MessagesFailed)))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}

#[cfg(test)]
mod tests {
    use super::{Motion, SCROLL_JUMP_ROWS};

    #[test]
    fn jumps_scroll_only_through_a_few_rows_and_place_farther_targets() {
        for (distance, visible, motion) in [
            (Some(0), true, Motion::Scroll),
            (Some(SCROLL_JUMP_ROWS), true, Motion::Scroll),
            (
                Some(SCROLL_JUMP_ROWS + 1),
                true,
                Motion::Place { reveal: true },
            ),
            // Outside the window: thousands of rows away.
            (None, true, Motion::Place { reveal: true }),
            // Opening a chat at its target: nothing on screen to replace.
            (None, false, Motion::Place { reveal: false }),
        ] {
            assert_eq!(Motion::for_jump(distance, visible), motion, "{distance:?}");
        }
    }
}
