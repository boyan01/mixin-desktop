use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData, MessageListView};
use nsh_core::{element::desktop::MenuItem, prelude::*};
use std::sync::Arc;

use super::message_appearance::floating_disc;
use crate::{
    i18n::{I18n, Key},
    runtime,
    ui::{self, Typography},
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
    ready: Signal<bool>,
    latest_visible: Memo<bool>,
    latest_observed: Signal<Option<String>>,
    loading_failed: Signal<bool>,
    marking: Signal<bool>,
    retry: Signal<u64>,
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    target: Signal<Option<String>>,
    failed: Signal<bool>,
    /// Whether the person can see these rows now.
    attentive: Memo<bool>,
}
impl Unread {
    pub fn mount(
        account: Arc<AccountClient>,
        conversation: &ConversationListData,
        target: Signal<Option<String>>,
        failed: Signal<bool>,
        messages: Signal<Vec<MessageListView>>,
        attentive: Memo<bool>,
    ) -> Self {
        let unseen = conversation.unseen_count;
        let last_read = conversation.last_read_message_id.clone();
        let latest_observed = signal(None::<String>);
        let state = Self {
            owner: Owner::current(),
            last_viewable: stored((ViewableItems::default(), Vec::new())),
            first: signal(None),
            mentions: signal(Vec::new()),
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
            conversation: stored(conversation.conversation_id.clone()),
            target,
            failed,
            attentive,
        };
        effect(move || {
            state.retry.get();
            let client = state.account.with(|a| a.clone());
            let id = state.conversation.with(|id| id.clone());
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(client.conversation_changes()));
                loop {
                    let reader = client.clone();
                    let conversation = id.clone();
                    let first_needed = state.first.with_untracked(Option::is_none)
                        && (!state.ready.get_untracked() || !state.latest_visible.get_untracked());
                    match runtime::run(async move {
                        let mentions = reader
                            .message()
                            .unread_mention_message_ids(conversation.clone())
                            .await?;
                        let first = if first_needed {
                            reader
                                .message()
                                .first_unread_message_id(conversation)
                                .await?
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
                    loop {
                        let Some(event) = changes.next().await else {
                            return;
                        };
                        if event.reload_all || event.conversation_ids.contains(&id) {
                            break;
                        }
                    }
                }
            });
        });
        // As in Flutter, the divider follows the last read row, so a chat opened
        // at its unread rows shows it with them instead of after its own query.
        if let Some(last_read) = last_read.filter(|_| unseen > 0) {
            effect(move || {
                if state.ready.get() || state.first.with(Option::is_some) {
                    return;
                }
                let newer = messages.with(|rows| {
                    let at = rows.iter().position(|row| row.message_id == last_read)?;
                    Some(rows[at.checked_sub(1)?].message_id.clone())
                });
                if newer.is_some() {
                    state.first.set(newer);
                }
            });
        }
        effect(move || {
            if attentive.get() {
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

    /// Marks the conversation read once its rows are in front of the person,
    /// as Flutter does on opening it, wherever they are scrolled to; again only
    /// when the newest row or the rows awaiting a read change.
    pub fn mark_read_while_open(
        self,
        messages: Signal<Vec<MessageListView>>,
        loaded: impl Fn() -> bool + 'static,
    ) {
        let own_id = self.account.with(|account| account.account_id());
        let reading = computed(move || {
            let open = self.attentive.get() && self.ready.get() && loaded();
            open.then(|| {
                messages.with(|rows| {
                    let waiting = rows
                        .iter()
                        .filter(|row| {
                            row.sender_id != own_id
                                && matches!(row.status.as_str(), "SENT" | "DELIVERED")
                        })
                        .count();
                    (rows.first().map(|row| row.message_id.clone()), waiting)
                })
            })
        });
        effect(move || {
            if reading.with(Option::is_none) {
                return;
            }
            let client = self.account.with(|account| account.clone());
            let id = self.conversation.with(|id| id.clone());
            spawn(async move {
                if let Err(error) =
                    runtime::run(async move { client.message().mark_conversation_read(id).await })
                        .await
                {
                    log::warn!("Could not mark the open conversation read: {error}");
                }
            });
        });
    }

    pub fn viewable(self, event: &ViewableItems, messages: Signal<Vec<MessageListView>>) {
        self.last_viewable
            .replace((*event, visible_ids(event, messages)));
        self.handle_viewable(event, messages);
    }

    /// A range callback only reports changed indices. After recycling replaces
    /// a row at the same index, measure the committed row and viewport instead
    /// of replaying an old visibility range.
    pub fn observe_row(
        self,
        message: impl Fn() -> Option<String> + Copy + 'static,
        first: impl Fn() -> bool + Copy + 'static,
        row: ElementRef,
        viewport: ElementRef,
    ) {
        let version = stored(0_u64);
        effect(move || {
            let ticket = version.with(|version| {
                *version = version.wrapping_add(1);
                *version
            });
            let id = message();
            let latest = first();
            if !self.attentive.get() {
                if latest {
                    self.latest_observed.set(None);
                }
                return;
            }
            let Some(id) = id.filter(|id| latest || self.has_mention(id)) else {
                return;
            };
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
                        || !self.attentive.get_untracked()
                    {
                        return;
                    }
                    let visible = intersects(row, viewport);
                    if latest && untracked(first) {
                        self.latest_observed.set(visible.then(|| id.clone()));
                    }
                    if visible && self.mentions.with_untracked(|ids| ids.contains(&id)) {
                        self.mark_visible(id);
                    }
                });
            });
        });
    }

    fn has_mention(self, id: &str) -> bool {
        self.mentions
            .with(|ids| ids.iter().any(|mention| mention == id))
    }

    fn handle_viewable(self, event: &ViewableItems, messages: Signal<Vec<MessageListView>>) {
        self.latest_observed
            .set(if event.viewable.is_some_and(|(first, _)| first == 0) {
                messages.with_untracked(|rows| rows.first().map(|row| row.message_id.clone()))
            } else {
                None
            });
        if !self.attentive.get_untracked() {
            return;
        }
        let mention = self.mentions.with_untracked(|mentions| {
            visible_ids(event, messages)
                .into_iter()
                .find(|id| mentions.contains(id))
        });
        if let Some(mention) = mention {
            self.mark_visible(mention);
        }
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
                    .child(ui::error_text(move || {
                        i18n.tr("Couldn’t load unread messages.", "无法加载未读消息。")
                    }))
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
                // One press target, as in Flutter: the count stands over the disc.
                let button = ui::round_button()
                    .w_10()
                    .pt_3()
                    .relative()
                    .disabled(self.marking)
                    .accessibility_label(move || {
                        Some(
                            format!(
                                "{} ({})",
                                i18n.tr("Next unread mention", "下一条未读提及"),
                                self.mentions.with(Vec::len)
                            )
                            .into(),
                        )
                    })
                    .on_press(move || {
                        let Some(id) = self.mentions.with(|ids| ids.first().cloned()) else {
                            return;
                        };
                        self.target.set(Some(id));
                    })
                    .child(
                        floating_disc()
                            .size_10()
                            .shadow_md()
                            .items_center()
                            .justify_center()
                            .child(
                                text("@")
                                    .text_role(ui::TextRole::Label)
                                    .text_color(ui::fg_primary),
                            ),
                    )
                    .child(
                        view()
                            .absolute()
                            .top_0()
                            .left_0()
                            .w_10()
                            .items_center()
                            .child(ui::count_pill(
                                move || {
                                    let count = self.mentions.with(Vec::len);
                                    if count > 99 {
                                        "99+".into()
                                    } else {
                                        count.to_string()
                                    }
                                },
                                || true,
                            )),
                    );
                ui::native_context_menu(button, move || {
                    if self.marking.get_untracked() {
                        return Vec::new();
                    }
                    vec![
                        MenuItem::new(i18n.tr("Clear", "清除"))
                            .on_select(move || self.clear_mentions()),
                    ]
                })
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
        id: impl Fn() -> Option<String> + 'static,
    ) -> impl IntoElement {
        when(
            move || self.first.with(|first| first.is_some() && *first == id()),
            move || {
                let appearance = use_context::<crate::appearance::Appearance>()
                    .expect("Home provides chat appearance");
                view()
                    .items_center()
                    .my(ui::space_xs() + ui::space_2xs())
                    .py_1()
                    .bg(ui::surface_pane)
                    .child(
                        text(move || i18n.tr("Unread messages", "未读消息"))
                            .text_size(move || appearance.secondary_size())
                            .text_color(ui::message_secondary),
                    )
            },
        )
    }
}
