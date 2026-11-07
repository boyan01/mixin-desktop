use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, MessageListView};
use nsh_base::{context_menu, menu_item, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use super::message_actions::label;
use crate::{
    i18n::{I18n, Key},
    runtime, ui,
};

fn visible_ids(event: &ViewableItems, messages: Signal<Vec<MessageListView>>) -> Vec<String> {
    event.viewable.map_or_else(Vec::new, |(first, last)| {
        messages.with_untracked(|rows| {
            rows.iter()
                .skip(first)
                .take(last.saturating_sub(first) + 1)
                .map(|row| row.message_id.clone())
                .collect()
        })
    })
}

fn intersects(row: Measurement, viewport: Measurement) -> bool {
    let left = row.page.x.0.max(viewport.page.x.0);
    let top = row.page.y.0.max(viewport.page.y.0);
    let right = (row.page.x.0 + row.frame.size.width.0)
        .min(viewport.page.x.0 + viewport.frame.size.width.0);
    let bottom = (row.page.y.0 + row.frame.size.height.0)
        .min(viewport.page.y.0 + viewport.frame.size.height.0);
    right > left && bottom > top
}

#[derive(Clone, Copy)]
pub(super) struct Unread {
    owner: Owner,
    last_viewable: Stored<(ViewableItems, Vec<String>)>,
    first: Signal<Option<String>>,
    mentions: Signal<Vec<String>>,
    pending: Signal<Option<String>>,
    pub ready: Signal<bool>,
    pub latest_visible: Memo<bool>,
    latest_observed: Signal<Option<String>>,
    loading_failed: Signal<bool>,
    marking: Signal<bool>,
    retry: Signal<u64>,
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    target: Signal<Option<String>>,
    reload: Signal<u64>,
    failed: Signal<bool>,
}
impl Unread {
    pub fn mount(
        account: Arc<AccountClient>,
        conversation: String,
        unseen: i64,
        target: Signal<Option<String>>,
        reload: Signal<u64>,
        failed: Signal<bool>,
        messages: Signal<Vec<MessageListView>>,
    ) -> Self {
        let latest_observed = signal(None::<String>);
        let state = Self {
            owner: Owner::current(),
            last_viewable: stored((ViewableItems::default(), Vec::new())),
            first: signal(None),
            mentions: signal(Vec::new()),
            pending: signal(None),
            ready: signal(unseen == 0),
            latest_visible: computed(move || {
                latest_observed.with(|observed| {
                    observed.as_ref().is_some_and(|id| {
                        messages.with(|rows| rows.first().is_some_and(|row| &row.message_id == id))
                    })
                })
            }),
            latest_observed,
            loading_failed: signal(false),
            marking: signal(false),
            retry: signal(0),
            account: stored(account),
            conversation: stored(conversation),
            target,
            reload,
            failed,
        };
        effect(move || {
            state.retry.get();
            let client = state.account.with(|a| a.clone());
            let id = state.conversation.with(|id| id.clone());
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(client.message_changes()));
                loop {
                    let client = client.clone();
                    let id = id.clone();
                    let first_needed = state.first.with_untracked(Option::is_none)
                        && (!state.ready.get_untracked() || !state.latest_visible.get_untracked());
                    match runtime::run(async move {
                        let mentions = client
                            .message()
                            .unread_mention_message_ids(id.clone())
                            .await?;
                        let first = if first_needed {
                            client.message().first_unread_message_id(id).await?
                        } else {
                            None
                        };
                        Ok((mentions, first))
                    })
                    .await
                    {
                        Ok((mentions, first)) => batch(|| {
                            state.mentions.set(mentions);
                            if state.first.with_untracked(Option::is_none) && first.is_some() {
                                state.first.set(first);
                            }
                            state.ready.set(true);
                            state.loading_failed.set(false);
                        }),
                        Err(_) => state.loading_failed.set(true),
                    }
                    if changes.next().await.is_none() {
                        break;
                    }
                }
            });
        });
        effect(move || {
            if lifecycle() == nsh_core::platform::Lifecycle::Active {
                let (last, ids) = state.last_viewable.with(|last| last.clone());
                if ids == visible_ids(&last, messages) {
                    state.handle_viewable(&last, messages);
                } else {
                    state.latest_observed.set(None);
                }
            }
        });
        state
    }
    pub fn viewable(self, event: &ViewableItems, messages: Signal<Vec<MessageListView>>) {
        self.last_viewable
            .replace((*event, visible_ids(event, messages)));
        self.handle_viewable(event, messages);
    }

    pub fn observe_row(
        self,
        message: impl Fn() -> Option<String> + Copy + 'static,
        first: impl Fn() -> bool + Copy + 'static,
        row: ElementRef,
        viewport: ElementRef,
        allowed: impl Fn() -> bool + Copy + 'static,
    ) {
        let version = stored(0_u64);
        effect(move || {
            let ticket = version.with(|version| {
                *version = version.wrapping_add(1);
                *version
            });
            let id = message();
            let latest = first();
            let pending = self.pending.get();
            if !allowed() || lifecycle() != nsh_core::platform::Lifecycle::Active {
                if latest {
                    self.latest_observed.set(None);
                }
                return;
            }
            let Some(id) = id.filter(|id| latest || pending.as_ref() == Some(id)) else {
                return;
            };
            // A range callback only reports changed indices. After recycling
            // replaces row 0 at the same indices, measure the real committed
            // row and viewport rather than replaying an old visibility range.
            after_answer(move || {
                spawn(async move {
                    let row_measure = row.measure();
                    let viewport_measure = viewport.measure();
                    let (Some(row), Some(viewport)) = (row_measure.await, viewport_measure.await)
                    else {
                        return;
                    };
                    if untracked(message).as_ref() != Some(&id)
                        || version.with(|version| *version) != ticket
                        || !untracked(allowed)
                        || lifecycle() != nsh_core::platform::Lifecycle::Active
                    {
                        return;
                    }
                    let visible = intersects(row, viewport);
                    if latest && untracked(first) {
                        self.latest_observed.set(visible.then(|| id.clone()));
                    }
                    if visible && self.pending.get_untracked().as_ref() == Some(&id) {
                        self.mark_visible(id);
                    }
                });
            });
        });
    }

    fn handle_viewable(self, event: &ViewableItems, messages: Signal<Vec<MessageListView>>) {
        self.latest_observed
            .set(if event.viewable.is_some_and(|(first, _)| first == 0) {
                messages.with_untracked(|rows| rows.first().map(|row| row.message_id.clone()))
            } else {
                None
            });
        if lifecycle() != nsh_core::platform::Lifecycle::Active {
            return;
        }
        let Some(target) = self.pending.get_untracked() else {
            return;
        };
        if !event.viewable.is_some_and(|(first, last)| {
            messages.with_untracked(|rows| {
                rows.iter()
                    .enumerate()
                    .any(|(index, row)| index >= first && index <= last && row.message_id == target)
            })
        }) {
            return;
        }
        self.mark_visible(target);
    }

    fn mark_visible(self, target: String) {
        if self.marking.get_untracked() {
            return;
        }
        let client = self.account.with(|a| a.clone());
        let id = self.conversation.with(|id| id.clone());
        self.marking.set(true);
        self.owner.run(|| {
            spawn(async move {
                let read = target.clone();
                let result =
                    runtime::commit(
                        async move { client.message().mark_mention_read(id, read).await },
                    )
                    .await;
                if result.is_ok() {
                    self.mentions.update(|ids| ids.retain(|id| id != &target));
                    if self.pending.get_untracked().as_ref() == Some(&target) {
                        self.pending.set(None);
                    }
                } else {
                    self.failed.set(true);
                }
                self.marking.set(false);
            })
        });
    }
    pub fn controls(self, i18n: I18n) -> impl IntoElement {
        view()
            .flex_row()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap_2()
            .child(when(self.loading_failed, move || {
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        text(move || {
                            label(i18n, "Couldn’t load unread messages.", "无法加载未读消息。")
                        })
                        .text_xs()
                        .text_color(move || theme().destructive),
                    )
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || self.retry.update(|v| *v += 1),
                    ))
            }))
    }

    pub fn jump_button(self, i18n: I18n) -> impl IntoElement {
        when(
            move || !self.mentions.with(Vec::is_empty),
            move || {
                let button = ui::command()
                    .w_10()
                    .h(px(52.0))
                    .px_0()
                    .py_0()
                    .relative()
                    .rounded_full()
                    .disabled(self.marking)
                    .accessibility_label(move || {
                        Some(
                            format!(
                                "{} ({})",
                                label(i18n, "Next unread mention", "下一条未读提及"),
                                self.mentions.with(Vec::len)
                            )
                            .into(),
                        )
                    })
                    .on_press(move || {
                        let Some(id) = self.mentions.with(|ids| ids.first().cloned()) else {
                            return;
                        };
                        batch(|| {
                            self.pending.set(Some(id.clone()));
                            self.target.set(Some(id));
                            self.reload.update(|value| *value += 1);
                        });
                    })
                    .child(
                        view()
                            .absolute()
                            .bottom_0()
                            .size_10()
                            .rounded_full()
                            .shadow_md()
                            .items_center()
                            .justify_center()
                            .bg(move || theme().color("bubble-in").unwrap_or(theme().card))
                            .child(
                                text("@")
                                    .text_size(px(17.0))
                                    .text_color(move || theme().foreground),
                            ),
                    )
                    .child(
                        view()
                            .absolute()
                            .top_0()
                            .left_0()
                            .w_10()
                            .h_5()
                            .items_center()
                            .child(
                                view()
                                    .min_w_5()
                                    .max_w_10()
                                    .h_5()
                                    .px_1()
                                    .rounded_full()
                                    .items_center()
                                    .justify_center()
                                    .bg(move || theme().primary)
                                    .child(
                                        text(move || {
                                            let count = self.mentions.with(Vec::len);
                                            if count > 99 {
                                                "99+".into()
                                            } else {
                                                count.to_string()
                                            }
                                        })
                                        .text_size(px(14.0))
                                        .text_color(move || theme().primary_foreground),
                                    ),
                            ),
                    );
                context_menu()
                    .disabled(self.marking)
                    .popup_style(ui::popup)
                    .trigger(button)
                    .child(
                        menu_item()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .hover(|style| style.bg(move || theme().accent))
                            .focus(|style| style.bg(move || theme().accent))
                            .disabled(self.marking)
                            .on_select(move || self.clear_mentions())
                            .child(text(move || label(i18n, "Clear", "清除")).text_sm()),
                    )
            },
        )
    }

    fn clear_mentions(self) {
        if self.marking.get_untracked() {
            return;
        }
        self.marking.set(true);
        let client = self.account.with(|client| client.clone());
        let id = self.conversation.with(|id| id.clone());
        let committed =
            runtime::commit(async move { client.message().clear_unread_mentions(id).await });
        self.owner.run(|| {
            spawn(async move {
                if committed.await.is_ok() {
                    self.pending.set(None);
                    // Query again: incoming mentions may race this command.
                    self.retry.update(|value| *value += 1);
                } else {
                    self.failed.set(true);
                }
                self.marking.set(false);
            })
        });
    }

    pub fn divider(
        self,
        i18n: I18n,
        messages: Signal<Vec<MessageListView>>,
        index: impl Fn() -> usize + Copy + 'static,
    ) -> impl IntoElement {
        when(
            move || {
                self.first.with(|id| {
                    id.as_ref().is_some_and(|id| {
                        messages
                            .with(|rows| rows.get(index()).is_some_and(|row| &row.message_id == id))
                    })
                })
            },
            move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .py_2()
                    .child(
                        view()
                            .flex_1()
                            .h(px(1.0))
                            .bg(move || theme().primary.opacity(0.35)),
                    )
                    .child(
                        text(move || label(i18n, "Unread messages", "未读消息"))
                            .text_xs()
                            .text_color(move || theme().primary),
                    )
                    .child(
                        view()
                            .flex_1()
                            .h(px(1.0))
                            .bg(move || theme().primary.opacity(0.35)),
                    )
            },
        )
    }
}
