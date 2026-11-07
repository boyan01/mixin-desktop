//! Bounded pinned-message browsing; mutations remain account-owned commands.

use std::{collections::HashSet, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, MessageListView};
use nsh_base::alert_dialog;
use nsh_core::{element::ViewableItems, prelude::*};

use crate::{
    chat::MediaPreview,
    conversation::parts,
    i18n::{self, Key},
    presentation, runtime,
    ui::{self, Typography as _},
};

const PAGE_SIZE: usize = 60;
const WINDOW_SIZE: usize = PAGE_SIZE * 3;

#[derive(Clone, Copy)]
struct Pinned {
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    rows: Signal<Vec<MessageListView>>,
    offset: Signal<usize>,
    count: Signal<usize>,
    total: Signal<usize>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    mutating: Signal<bool>,
    can_unpin: Signal<bool>,
    request: Signal<u64>,
    dirty: Stored<bool>,
    deferred: Stored<Option<bool>>,
    visible: Stored<Option<String>>,
    visible_range: Signal<Option<(usize, usize)>>,
    preview: MediaPreview,
    owner: Owner,
}

impl Pinned {
    fn load(self, refresh: bool, close: impl Fn() + Copy + 'static) {
        if self.loading.get_untracked() {
            self.dirty.replace(true);
            return;
        }
        let ticket = self.request.get_untracked().wrapping_add(1);
        self.request.set(ticket);
        self.loading.set(true);
        self.failed.set(false);
        let account = self.account.with(|account| account.clone());
        let conversation = self.conversation.with(|id| id.clone());
        let previous_offset = self.offset.get_untracked();
        let count = self.count.get_untracked();
        let anchor = refresh
            .then(|| {
                self.rows
                    .with(|rows| rows.first().map(|row| row.message_id.clone()))
            })
            .flatten();
        let selected = self.preview.selection_ids();
        // Retry calls this from a branch that clearing `failed` disposes at once.
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::run(async move {
                    let messages = account.message();
                    let total =
                        usize::try_from(messages.pinned_message_count(conversation.clone()).await?)
                            .unwrap_or_default();
                    let offset = if let Some(anchor) = anchor {
                        messages
                            .pinned_message_offset(conversation.clone(), anchor)
                            .await?
                            .unwrap_or(previous_offset)
                    } else {
                        previous_offset
                    }
                    .min(total.saturating_sub(1));
                    let rows = messages
                        .pinned_messages_page(conversation.clone(), offset as i64, count as i64)
                        .await?;
                    let mut ids: Vec<String> = rows
                        .iter()
                        .map(|row| row.message_id.clone())
                        .chain(selected)
                        .collect();
                    let mut seen = HashSet::new();
                    ids.retain(|id| seen.insert(id.clone()));
                    let mut policies = Vec::new();
                    for ids in ids.chunks(200) {
                        policies.extend(
                            messages
                                .message_actions(conversation.clone(), ids.to_vec())
                                .await?,
                        );
                    }
                    let can_unpin = messages.can_unpin_messages(conversation.clone()).await?;
                    let can_send = account.conversation().can_send(conversation).await?;
                    Ok((rows, offset, total, policies, can_unpin, can_send))
                })
                .await;
                if self.request.get_untracked() != ticket {
                    return;
                }
                match result {
                    Ok((rows, offset, total, policies, can_unpin, can_send)) => {
                        self.offset.set(offset);
                        self.total.set(total);
                        self.can_unpin.set(can_unpin);
                        self.preview.pinned_permissions(policies, can_send);
                        if total == 0 {
                            close();
                            return;
                        }
                        self.apply(rows, ticket, close);
                    }
                    Err(error) => {
                        log::warn!("Could not load pinned messages: {error}");
                        self.can_unpin.set(false);
                        self.preview.pinned_permissions(Vec::new(), false);
                        self.failed.set(true);
                        self.loading.set(false);
                        self.deferred.replace(None);
                    }
                }
            })
        });
    }

    fn apply(self, rows: Vec<MessageListView>, ticket: u64, close: impl Fn() + Copy + 'static) {
        let previous = self.rows.get_untracked();
        let visible = self.visible.with(|id| id.clone());
        if let Some(phase) =
            presentation::message_window::splice_phase(&previous, &rows, visible.as_deref())
        {
            self.rows.set(phase);
            after_answer(move || {
                if self.owner.is_live() && self.request.get_untracked() == ticket {
                    self.apply(rows, ticket, close);
                }
            });
            return;
        }
        self.rows.set(rows);
        self.loading.set(false);
        if self.dirty.with(std::mem::take) {
            self.load(true, close);
        } else if let Some(older) = self.deferred.with(|edge| edge.take()) {
            self.page(older, close);
        }
    }

    fn page(self, older: bool, close: impl Fn() + Copy + 'static) {
        if self.loading.get_untracked() {
            self.deferred.replace(Some(older));
            return;
        }
        if self.failed.get_untracked() {
            return;
        }
        let offset = self.offset.get_untracked();
        let count = self.count.get_untracked();
        if older && offset + self.rows.with(Vec::len) < self.total.get_untracked() {
            if count < WINDOW_SIZE {
                self.count.set((count + PAGE_SIZE).min(WINDOW_SIZE));
            } else {
                self.offset.set(offset.saturating_add(PAGE_SIZE));
            }
        } else if !older && offset > 0 {
            self.offset.set(offset.saturating_sub(PAGE_SIZE));
        } else {
            return;
        }
        self.load(false, close);
    }

    fn unpin_all(self, close: impl Fn() + Copy + 'static) {
        if self.mutating.get_untracked() || !self.can_unpin.get_untracked() {
            return;
        }
        self.mutating.set(true);
        let i18n = i18n::current();
        spawn(async move {
            let result = alert_dialog()
                .title(i18n.tr(
                    "Are you sure you want to unpin all messages?",
                    "确定取消置顶所有消息么？",
                ))
                .cancel("cancel", i18n.text(Key::Cancel))
                .action("unpin", i18n.tr("Unpin All Messages", "取消所有置顶消息"))
                .show()
                .await;
            if matches!(result, Ok(answer) if answer.action.as_deref() == Some("unpin")) {
                let account = self.account.with(|account| account.clone());
                let conversation = self.conversation.with(|id| id.clone());
                let command = runtime::commit(async move {
                    account.message().unpin_all_messages(conversation).await
                });
                let result = command.await;
                self.failed.set(result.is_err());
                if result.is_ok() {
                    self.preview.prune_selection(&self.preview.selection_ids());
                    self.load(true, close);
                }
            }
            self.mutating.set(false);
        });
    }
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    conversation: String,
    close: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
) -> impl IntoElement {
    crate::ui::owned(conversation.clone(), move || {
        reader(account.clone(), conversation.clone(), close, target)
    })
}

fn reader(
    account: Arc<AccountClient>,
    conversation: String,
    close: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let media_viewport = crate::chat::message_media::viewport();
    let state = Pinned {
        account: stored(account.clone()),
        conversation: stored(conversation.clone()),
        rows: signal_with(Vec::new(), SignalEquals::Never),
        offset: signal(0),
        count: signal(PAGE_SIZE),
        total: signal(0),
        loading: signal(false),
        failed: signal(false),
        mutating: signal(false),
        can_unpin: signal(false),
        request: signal(0),
        dirty: stored(false),
        deferred: stored(None),
        visible: stored(None),
        visible_range: signal(None),
        preview: MediaPreview::pinned(account.clone()),
        owner: Owner::current(),
    };
    state.preview.mount_mentions(state.rows);
    let locate = move |id| {
        target.set(Some(id));
        close();
    };
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
        state.load(false, close);
        while let Some(change) = changes.next().await {
            if change.reload_all || change.conversation_ids.contains(&conversation) {
                state.load(true, close);
            }
        }
    });
    view()
        .flex_1()
        .min_h_0()
        .child(parts::page_header(
            move || format!("{} ({})", i18n.text(Key::Pinned), state.total.get()),
            parts::back_button(close),
        ))
        .child(when(
            move || state.loading.get() && state.rows.with(Vec::is_empty),
            move || {
                parts::status(move || i18n.text(Key::LoadingMessages))
                    .px_4()
                    .py_2()
            },
        ))
        .child(when(
            move || state.failed.get() || state.preview.failed.get(),
            move || {
                parts::failure(
                    move || i18n.text(Key::CommandFailed),
                    move || {
                        state.preview.failed.set(false);
                        state.load(true, close);
                    },
                )
                .px_4()
                .py_2()
            },
        ))
        .child(
            virtual_list(
                move || state.rows.with(Vec::len),
                move |at| {
                    crate::sticker::visibility(move || {
                        state
                            .visible_range
                            .get()
                            .is_some_and(|(first, last)| (first..=last).contains(&at.index()))
                            && state.preview.animation_visible()
                            && !state.mutating.get()
                    });
                    let row = move || state.rows.with(|rows| rows.get(at.index()).cloned());
                    state.preview.pinned_row(
                        row,
                        row_body(state, move || at.index(), row, locate),
                        locate,
                    )
                },
            )
            .on_layout(media_viewport)
            .keys(move || {
                state.rows.with(|rows| {
                    rows.iter()
                        .map(|row| row.message_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .on_viewable_items_change(move |items: &ViewableItems| {
                state.visible_range.set(items.viewable);
                state.visible.replace(items.viewable.and_then(|(first, _)| {
                    state
                        .rows
                        .with(|rows| rows.get(first).map(|row| row.message_id.clone()))
                }));
            })
            .on_load_more(move || state.page(true, close))
            .on_load_more_at_start(move || state.page(false, close))
            .inverted()
            .estimated_item_size(ui::list_row_height())
            .flex_1()
            .min_h_0(),
        )
        .child(state.preview.pinned_selection(state.conversation))
        .child(
            ui::card_row()
                .justify_center()
                .border_t_1()
                .border_color(ui::border)
                .disabled(move || {
                    state.mutating.get() || state.total.get() == 0 || !state.can_unpin.get()
                })
                .on_press(move || state.unpin_all(close))
                .child(
                    text(move || i18n.tr("Unpin All Messages", "取消所有置顶消息"))
                        .text_color(ui::accent),
                ),
        )
        .child(state.preview.surface())
}

fn row_body(
    state: Pinned,
    index: impl Fn() -> usize + Copy + 'static,
    row: impl Fn() -> Option<MessageListView> + Copy + 'static,
    locate: impl Fn(String) + Copy + 'static,
) -> View {
    let i18n = i18n::current();
    let mine = move || {
        row().is_some_and(|row| state.account.with(|account| account.account_id()) == row.sender_id)
    };
    let inline_media =
        move || row().is_some_and(|row| crate::chat::message_media::inline_metadata(&row));
    view()
        .flex_1()
        .min_w_0()
        .px_3()
        .py_2()
        .gap_2()
        .child(when(
            move || {
                state.rows.with(|rows| {
                    let day =
                        |row: &MessageListView| presentation::day(i18n, row.created_at_micros);
                    let current = rows.get(index()).map(day);
                    current.is_some() && current != rows.get(index() + 1).map(day)
                })
            },
            move || {
                text(move || {
                    row()
                        .map(|row| presentation::day(i18n, row.created_at_micros))
                        .unwrap_or_default()
                })
                .text_role(ui::TextRole::Caption)
                .text_color(ui::fg_secondary)
            },
        ))
        .child(
            text(move || row().map(|row| row.sender_name).unwrap_or_default())
                .text_role(ui::TextRole::Body)
                .text_emphasis()
                .style_when(mine, |style| style.hidden()),
        )
        .child(
            view()
                .flex_row()
                .style_when(mine, |style| style.flex_row_reverse())
                .items_center()
                .gap_2()
                .child(dynamic(move || {
                    let row = row();
                    untracked(move || match row {
                        Some(row) => bubble(state, row, mine).into_element(),
                        None => view().hidden().into_element(),
                    })
                }))
                .child(
                    ui::icon_button()
                        .size_8()
                        .px_1()
                        .on_press(move || {
                            if let Some(row) = row() {
                                locate(row.message_id);
                            }
                        })
                        .accessibility_label(move || {
                            Some(i18n.tr("Locate in chat", "在聊天中定位").into())
                        })
                        .child(ui::icon(ui::Icon::ArrowLeft)),
                ),
        )
        .child(
            text(move || {
                row()
                    .map(|row| presentation::time(i18n, row.created_at_micros / 1000))
                    .unwrap_or_default()
            })
            .text_role(ui::TextRole::Caption)
            .style_when(inline_media, |style| style.hidden())
            .text_color(ui::fg_secondary),
        )
}

fn bubble(
    state: Pinned,
    row: MessageListView,
    mine: impl Fn() -> bool + 'static,
) -> impl IntoElement {
    let media = crate::chat::message_media::inline_metadata(&row);
    view()
        .min_w_0()
        .flex_1()
        .p_2()
        .style_when(move || media, |style| style.p_0().flex_none())
        .rounded(ui::radius_md())
        .bg(move || {
            if mine() {
                ui::bubble_out()
            } else {
                ui::bubble_in()
            }
        })
        .child(state.preview.content(row))
}
