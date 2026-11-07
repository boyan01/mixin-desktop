//! View-owned bounded history, edge loading and message jumps.
mod query;

use std::collections::{HashMap, HashSet};

use futures::StreamExt as _;
use mixin_desktop_api::{ConversationListData, MessageActions, MessageListView};
use nsh_base::theme;
use nsh_core::{element::ImageSrc, prelude::*};

use super::message_actions::{Actions, label};
use crate::presentation::message_window::splice_phase;
use crate::{
    i18n::{I18n, Key},
    runtime, ui,
};
use query::{Load, Window};

#[derive(Clone, Copy)]
pub(super) struct Timeline {
    conversation: Stored<String>,
    actions: Actions,
    header: Signal<ConversationListData>,
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
    pub busy: Signal<bool>,
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
}

impl Timeline {
    pub fn mount(
        conversation: ConversationListData,
        target: Signal<Option<String>>,
        retry: Signal<u64>,
        history: ElementRef,
        messages: Signal<Vec<MessageListView>>,
        header: Signal<ConversationListData>,
        actions: Actions,
    ) -> Self {
        let state = Self {
            conversation: stored(conversation.conversation_id),
            actions,
            header,
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
                    let permissions = client.message().message_actions(id.clone(), ids).await?;
                    let metadata = client
                        .conversation()
                        .conversation_items_by_ids(vec![id])
                        .await?
                        .into_iter()
                        .next();
                    Ok((window, permissions, metadata))
                })
                .await;
                if state.revision.get_untracked() != ticket {
                    return;
                }
                match result {
                    Ok((window, permissions, metadata)) => {
                        if state.apply(window, permissions, metadata, highlight, explicit, ticket) {
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
            retry.get();
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
            let mut changes = Box::pin(runtime::observe(client.message_changes()));
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
            let tail = matches!(&load, Load::Latest);
            self.following_tail.set(tail);
            self.show_latest.set(!tail);
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
        metadata: Option<ConversationListData>,
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
            if let Some(metadata) = metadata {
                self.header.set(metadata);
            }
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
        if !explicit && !tail {
            return self.apply_rows(window.rows, window.deleted, ticket);
        }
        self.messages.set(window.rows);
        if let Some(id) = scroll {
            self.visible_anchor.set(Some(id.clone()));
            if highlight {
                self.highlight.set(Some(id.clone()));
            }
            self.scroll_to(id, position_revision);
        } else if tail {
            after_answer(move || {
                if self.position_revision.get_untracked() == position_revision {
                    self.history
                        .scroll_to_index(0, percentage(0.0), px(0.0), false);
                }
            });
        }
        false
    }

    fn apply_rows(self, rows: Vec<MessageListView>, deleted: HashSet<String>, ticket: u64) -> bool {
        let previous = self.messages.get_untracked();
        let anchor = self.visible_anchor.get_untracked();
        if let Some(anchor) = anchor.as_deref()
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
                if self.revision.get_untracked() == ticket
                    && !self.apply_rows(rows, deleted, ticket)
                {
                    self.complete();
                }
            });
            return true;
        }
        self.messages.set(rows);
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
                // NSH mirrors view_position for an inverted list: 0.8 puts the
                // target near Flutter's 0.2 focus anchor measured from the top.
                self.history.scroll_to_index(
                    index as u32,
                    percentage(0.8),
                    px(0.0),
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
        if let Some(id) = source {
            if self.target.get_untracked().as_ref() == Some(&id) {
                self.schedule(Load::Jump(id), true);
            } else {
                self.target.set(Some(id));
            }
            return;
        }
        if self.target.get_untracked().is_some() {
            self.target.set(None);
        } else {
            self.schedule(Load::Latest, true);
        }
    }

    pub fn controls(self, i18n: I18n) -> impl IntoElement {
        view()
            .flex_row()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap_2()
            .child(when(
                move || self.busy.get() && !self.loading.get(),
                move || {
                    text(move || {
                        self.last_load.with(|load| {
                            if matches!(load, Load::Older) {
                                format!("{}…", i18n.text(Key::Earlier))
                            } else {
                                i18n.text(Key::LoadingMessages).to_owned()
                            }
                        })
                    })
                    .text_xs()
                    .text_color(move || theme().muted_foreground)
                },
            ))
            .child(when(self.missing_target, move || {
                text(move || {
                    label(
                        i18n,
                        "This message is no longer available.",
                        "此消息已不可用。",
                    )
                })
                .text_xs()
                .text_color(move || theme().muted_foreground)
            }))
            .child(when(self.failed, move || {
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        text(move || i18n.text(Key::MessagesFailed))
                            .text_xs()
                            .text_color(move || theme().destructive),
                    )
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || {
                            let load = self.last_load.with(|load| load.clone());
                            if matches!(load, Load::Refresh { .. }) {
                                self.refresh();
                            } else {
                                self.schedule(load, true);
                            }
                        },
                    ))
            }))
    }
    pub fn jump_button(self, i18n: I18n) -> impl IntoElement {
        when(
            move || {
                !self.loading.get()
                    && (self.has_newer.get() || self.show_latest.get() || self.missing_target.get())
            },
            move || {
                let arrow = include_str!("../../assets/jump_current_arrow.svg")
                    .replace("fill=\"white\"", "fill=\"currentColor\"");
                ui::command()
                    .size_10()
                    .px_0()
                    .py_0()
                    .rounded_full()
                    .shadow_md()
                    .bg(move || theme().color("bubble-in").unwrap_or(theme().card))
                    .accessibility_label(move || {
                        Some(if self.return_target.with(Option::is_some) {
                            label(i18n, "Return to message", "返回消息").into()
                        } else {
                            i18n.text(Key::Latest).into()
                        })
                    })
                    .on_press(move || self.latest())
                    .child(
                        svg(ImageSrc::svg_markup(&arrow))
                            .w(px(14.0))
                            .h(px(9.0))
                            .text_color(move || theme().foreground),
                    )
            },
        )
    }
}
