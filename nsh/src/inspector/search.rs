//! Conversation-scoped search and its bounded, cursor-paged presentation window.

use std::{collections::HashSet, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ClientResult, ConversationParticipantItem, MessageListView,
};
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::presentation::message_window::splice_phase;
use crate::{
    i18n::{self, Key, tr},
    presentation, runtime, ui,
};

const PAGE_SIZE: u32 = 64;
const WINDOW_PAGES: usize = 3;

#[derive(Clone, Copy)]
enum Load {
    Reset,
    Older,
    Newer,
    Refresh,
}

#[derive(Clone)]
struct Page {
    index: usize,
    anchor: Option<String>,
    rows: Vec<MessageListView>,
    more: bool,
}

#[derive(Clone, Copy)]
struct Results {
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    query: Signal<String>,
    sender: Signal<Option<String>>,
    category: Signal<Option<&'static str>>,
    rows: Signal<Vec<MessageListView>>,
    pages: Stored<Vec<Page>>,
    cursors: Stored<Vec<Option<String>>>,
    request: Signal<Option<(u64, Load)>>,
    revision: Signal<u64>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    older: Signal<bool>,
    newer: Signal<bool>,
    visible: Stored<Option<String>>,
    dirty: Stored<bool>,
    deferred: Stored<Option<Load>>,
    last: Stored<Load>,
}

impl Results {
    fn schedule(self, load: Load) {
        if self.busy.get_untracked() && !matches!(load, Load::Reset) {
            if matches!(load, Load::Refresh) {
                self.dirty.replace(true);
            } else {
                self.deferred.replace(Some(load));
            }
            return;
        }
        if matches!(load, Load::Reset) {
            self.pages.replace(Vec::new());
            self.cursors.replace(vec![None]);
            self.visible.replace(None);
            self.dirty.replace(false);
            self.deferred.replace(None);
            self.rows.set(Vec::new());
            self.newer.set(false);
            self.older.set(false);
        }
        let ticket = self.revision.get_untracked().wrapping_add(1);
        self.last.replace(load);
        batch(|| {
            self.revision.set(ticket);
            self.failed.set(false);
            self.busy.set(true);
            self.request.set(Some((ticket, load)));
        });
    }

    fn complete(self) {
        self.busy.set(false);
        if let Some(load) = self.deferred.with(|load| load.take()) {
            let possible = match load {
                Load::Older => self.older.get_untracked(),
                Load::Newer => self.newer.get_untracked(),
                _ => true,
            };
            if possible {
                self.schedule(load);
                return;
            }
        }
        if self.dirty.with(std::mem::take) {
            self.schedule(Load::Refresh);
        }
    }

    fn apply(self, pages: Vec<Page>, ticket: u64) {
        let mut seen = HashSet::new();
        let rows: Vec<_> = pages
            .iter()
            .flat_map(|page| page.rows.iter())
            .filter(|row| seen.insert(row.message_id.clone()))
            .cloned()
            .collect();
        let previous = self.rows.get_untracked();
        let visible = self.visible.with(|id| id.clone());
        let recovered = pages.len() == 1
            && pages.first().is_some_and(|page| page.index == 0)
            && self
                .pages
                .with(|pages| pages.first().is_some_and(|page| page.index > 0));
        // An opposite-edge scroll during the request must not evict the reader's row.
        if !matches!(self.last.with(|load| *load), Load::Refresh)
            && !recovered
            && let Some(id) = visible.as_deref()
            && previous.iter().any(|row| row.message_id == id)
            && !rows.iter().any(|row| row.message_id == id)
        {
            self.complete();
            return;
        }
        self.apply_rows(rows, pages, ticket);
    }

    fn apply_rows(self, rows: Vec<MessageListView>, pages: Vec<Page>, ticket: u64) {
        let previous = self.rows.get_untracked();
        let visible = self.visible.with(|id| id.clone());
        if let Some(phase) = splice_phase(&previous, &rows, visible.as_deref()) {
            self.rows.set(phase);
            // NSH 0.4 recognizes one prefix/suffix splice per native answer.
            // Keep the measured visible row while changing the two window edges.
            after_answer(move || {
                if self.revision.get_untracked() == ticket {
                    self.apply_rows(rows, pages, ticket);
                }
            });
            return;
        }
        self.cursors.with(|cursors| {
            for page in &pages {
                if cursors.len() <= page.index {
                    cursors.resize(page.index + 1, None);
                }
                cursors[page.index] = page.anchor.clone();
            }
        });
        batch(|| {
            self.newer
                .set(pages.first().is_some_and(|page| page.index > 0));
            self.older.set(pages.last().is_some_and(|page| page.more));
            self.pages.replace(pages);
            self.rows.set(rows);
        });
        self.complete();
    }
}

async fn load_page(
    account: Arc<AccountClient>,
    id: String,
    query: String,
    sender: Option<String>,
    category: Option<&'static str>,
    index: usize,
    anchor: Option<String>,
) -> ClientResult<Page> {
    let categories = category
        .map(|kind| {
            ["PLAIN", "SIGNAL", "ENCRYPTED"]
                .map(|prefix| format!("{prefix}_{kind}"))
                .to_vec()
        })
        .unwrap_or_default();
    let mut rows = account
        .message()
        .search_messages(id, query, sender, categories, anchor.clone(), PAGE_SIZE + 1)
        .await?;
    let more = rows.len() > PAGE_SIZE as usize;
    rows.truncate(PAGE_SIZE as usize);
    Ok(Page {
        index,
        anchor,
        rows,
        more,
    })
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    id: String,
    close: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let navigation =
        use_context::<crate::navigation::ChatNavigation>().expect("Home provides chat navigation");
    let picker = signal(false);
    let sender_query = signal(String::new());
    let selected = signal_with(None::<ConversationParticipantItem>, SignalEquals::Never);
    let candidates = signal_with(
        Vec::<ConversationParticipantItem>::new(),
        SignalEquals::Never,
    );
    let candidates_busy = signal(false);
    let candidates_failed = signal(false);
    let candidates_retry = signal(0_u64);
    let candidate_revision = signal(0_u64);
    let input = element_ref();
    let results = Results {
        account: stored(account.clone()),
        conversation: stored(id.clone()),
        query: signal(String::new()),
        sender: signal(None),
        category: signal(None),
        rows: signal_with(Vec::new(), SignalEquals::Never),
        pages: stored(Vec::new()),
        cursors: stored(vec![None]),
        request: signal_with(None, SignalEquals::Never),
        revision: signal(0),
        busy: signal(false),
        failed: signal(false),
        older: signal(false),
        newer: signal(false),
        visible: stored(None),
        dirty: stored(false),
        deferred: stored(None),
        last: stored(Load::Reset),
    };
    effect(move || {
        results.query.get();
        results.sender.get();
        results.category.get();
        results.schedule(Load::Reset);
    });
    effect(move || {
        let Some((ticket, load)) = results.request.get() else {
            return;
        };
        let query = results.query.get_untracked().trim().to_owned();
        let sender = results.sender.get_untracked();
        let category = results.category.get_untracked();
        let account = results.account.with(|account| account.clone());
        let id = results.conversation.with(|id| id.clone());
        let mut pages = results.pages.with(|pages| pages.clone());
        let cursors = results.cursors.with(|cursors| cursors.clone());
        spawn(async move {
            if query.is_empty() && sender.is_none() {
                results.complete();
                return;
            }
            if matches!(load, Load::Reset) {
                timer(std::time::Duration::from_millis(150)).await;
            }
            let loaded = runtime::run(async move {
                let index = match load {
                    Load::Reset => 0,
                    Load::Older => pages.last().map_or(0, |page| page.index + 1),
                    Load::Newer => pages.first().map_or(0, |page| page.index.saturating_sub(1)),
                    Load::Refresh => {
                        let mut refreshed = Vec::new();
                        let mut anchor = pages.first().and_then(|page| page.anchor.clone());
                        for page in pages {
                            let page = load_page(
                                account.clone(),
                                id.clone(),
                                query.clone(),
                                sender.clone(),
                                category,
                                page.index,
                                anchor,
                            )
                            .await?;
                            anchor = page.rows.last().map(|row| row.message_id.clone());
                            let more = page.more;
                            refreshed.push(page);
                            if !more {
                                break;
                            }
                        }
                        if refreshed
                            .first()
                            .is_some_and(|page| page.index > 0 && page.rows.is_empty())
                        {
                            // A deleted cursor can no longer locate the retained window.
                            // Recover from a valid boundary instead of a false empty search.
                            return Ok(vec![
                                load_page(account, id, query, sender, category, 0, None).await?,
                            ]);
                        }
                        return Ok(refreshed);
                    }
                };
                let anchor = if matches!(load, Load::Older) {
                    pages
                        .last()
                        .and_then(|page| page.rows.last().map(|row| row.message_id.clone()))
                } else {
                    cursors.get(index).cloned().flatten()
                };
                let page = load_page(
                    account.clone(),
                    id.clone(),
                    query.clone(),
                    sender.clone(),
                    category,
                    index,
                    anchor,
                )
                .await?;
                match load {
                    Load::Reset => pages = vec![page],
                    Load::Older => {
                        if page.rows.is_empty() {
                            if let Some(last) = pages.last_mut() {
                                last.more = false;
                            }
                            return Ok(pages);
                        }
                        pages.push(page);
                        if pages.len() > WINDOW_PAGES {
                            pages.remove(0);
                        }
                    }
                    Load::Newer => {
                        if page.index > 0 && page.rows.is_empty() {
                            return Ok(vec![
                                load_page(account, id, query, sender, category, 0, None).await?,
                            ]);
                        }
                        let count = (pages.len() + 1).min(WINDOW_PAGES);
                        pages = vec![page];
                        // Rebuild adjacent boundaries after inserts/deletions so the
                        // restored page cannot leave a hole before retained results.
                        while pages.len() < count && pages.last().is_some_and(|page| page.more) {
                            let anchor = pages
                                .last()
                                .and_then(|page| page.rows.last())
                                .map(|row| row.message_id.clone());
                            let page = load_page(
                                account.clone(),
                                id.clone(),
                                query.clone(),
                                sender.clone(),
                                category,
                                index + pages.len(),
                                anchor,
                            )
                            .await?;
                            pages.push(page);
                        }
                    }
                    Load::Refresh => unreachable!(),
                }
                Ok(pages)
            })
            .await;
            if results.revision.get_untracked() != ticket {
                return;
            }
            match loaded {
                Ok(pages) => results.apply(pages, ticket),
                Err(_) => {
                    results.failed.set(true);
                    results.complete();
                }
            }
        });
    });
    let picker_account = account.clone();
    let picker_id = id.clone();
    effect(move || {
        let is_open = picker.get();
        let keyword = sender_query.get();
        candidates_retry.get();
        let ticket = candidate_revision.get_untracked().wrapping_add(1);
        candidate_revision.set(ticket);
        if !is_open {
            return;
        }
        let account = picker_account.clone();
        let id = picker_id.clone();
        batch(|| {
            candidates.set(Vec::new());
            candidates_busy.set(true);
            candidates_failed.set(false);
        });
        spawn(async move {
            timer(std::time::Duration::from_millis(100)).await;
            let answer = runtime::run(async move {
                let access = account.conversation();
                let conversation = access
                    .conversation_items_by_ids(vec![id.clone()])
                    .await?
                    .into_iter()
                    .next();
                if conversation.is_some_and(|row| row.is_bot || row.is_bot_group) {
                    access.search_bot_group_users(id, keyword).await
                } else if keyword.is_empty() {
                    access.conversation_participants(id).await
                } else {
                    access.search_group_users(id, keyword).await
                }
            })
            .await;
            if candidate_revision.get_untracked() != ticket {
                return;
            }
            batch(|| {
                match answer {
                    Ok(rows) => candidates.set(rows),
                    Err(_) => candidates_failed.set(true),
                }
                candidates_busy.set(false);
            });
        });
    });
    let changes_account = account.clone();
    let changes_id = id.clone();
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(changes_account.conversation_changes()));
        while let Some(change) = changes.next().await {
            if picker.get_untracked()
                && (change.reload_all || change.conversation_ids.contains(&changes_id))
            {
                candidates_retry.update(|revision| *revision += 1);
            }
        }
    });
    spawn(async move {
        let mut messages = Box::pin(runtime::observe(account.message_changes()));
        while messages.next().await.is_some() {
            results.schedule(Load::Refresh);
            if picker.get_untracked() {
                candidates_retry.update(|revision| *revision += 1);
            }
        }
    });
    let back = move || {
        if picker.get_untracked() {
            dismiss_keyboard();
            batch(|| {
                picker.set(false);
                sender_query.set(String::new());
            });
        } else {
            close();
        }
    };
    view()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .h_16()
                .px_4()
                .gap_3()
                .flex_row()
                .items_center()
                .child(
                    ui::icon_button()
                        .size_8()
                        .px_1()
                        .on_press(back)
                        .accessibility_label(move || Some(i18n.text(Key::Back).into()))
                        .child(ui::icon(ui::Icon::ArrowLeft)),
                )
                .child(
                    text(move || {
                        if picker.get() {
                            tr("Choose a sender", "选择发送者")
                        } else {
                            i18n.text(Key::SearchMessages)
                        }
                    })
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD),
                ),
        )
        .child(
            view()
                .px_4()
                .pb_3()
                .gap_2()
                .child(when(
                    move || !picker.get() && selected.with(Option::is_some),
                    move || {
                        ui::command()
                            .justify_start()
                            .w_full()
                            .bg(move || theme().muted)
                            .accessibility_label(move || {
                                Some(tr("Clear sender", "清除发送者").into())
                            })
                            .on_press(move || {
                                selected.set(None);
                                results.sender.set(None);
                                input.focus();
                            })
                            .child(
                                text(move || {
                                    format!(
                                        "{}: {} ×",
                                        tr("From", "发送者"),
                                        selected.with(|sender| sender
                                            .as_ref()
                                            .map(|sender| sender.full_name.clone())
                                            .unwrap_or_default())
                                    )
                                })
                                .text_sm()
                                .number_of_lines(1),
                            )
                    },
                ))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(dynamic(move || {
                            let is_picker = picker.get();
                            ui::field()
                                .key(if is_picker {
                                    "sender-search"
                                } else {
                                    "message-search"
                                })
                                .flex_1()
                                .min_w_0()
                                .element_ref(&input)
                                .auto_focus(true)
                                .placeholder(move || {
                                    if is_picker {
                                        tr("Search senders", "搜索发送者").to_owned()
                                    } else {
                                        i18n.text(Key::SearchMessages).to_owned()
                                    }
                                })
                                .label(move || {
                                    if is_picker {
                                        tr("Search senders", "搜索发送者")
                                    } else {
                                        i18n.text(Key::SearchMessages)
                                    }
                                })
                                .value(move || {
                                    if is_picker {
                                        sender_query.get()
                                    } else {
                                        results.query.get()
                                    }
                                })
                                .on_change(move |value: &str| {
                                    if is_picker {
                                        sender_query.set(value.to_owned());
                                    } else {
                                        batch(|| {
                                            results.query.set(value.to_owned());
                                            if value.is_empty() {
                                                results.category.set(None);
                                            }
                                        });
                                    }
                                })
                                .on_key_press(move |key: &str| {
                                    if key != "Backspace" {
                                        return;
                                    }
                                    if is_picker && sender_query.with_untracked(String::is_empty) {
                                        back();
                                    } else if !is_picker
                                        && results.query.with_untracked(String::is_empty)
                                        && results.sender.with_untracked(Option::is_some)
                                    {
                                        batch(|| {
                                            selected.set(None);
                                            results.sender.set(None);
                                        });
                                    }
                                })
                        }))
                        .child(when(
                            move || !picker.get(),
                            move || {
                                ui::icon_button()
                                    .size_10()
                                    .px_2()
                                    .accessibility_label(move || {
                                        Some(tr("Choose a sender", "选择发送者").into())
                                    })
                                    .on_press(move || {
                                        dismiss_keyboard();
                                        batch(|| {
                                            sender_query.set(String::new());
                                            picker.set(true);
                                        });
                                    })
                                    .child(ui::icon(ui::Icon::Contacts))
                            },
                        )),
                )
                .child(when(
                    move || !picker.get() && !results.query.with(|query| query.trim().is_empty()),
                    move || {
                        view()
                            .flex_row()
                            .gap_2()
                            .child(category_button(results, "TEXT", move || tr("Text", "文字")))
                            .child(category_button(results, "POST", move || tr("Post", "长文")))
                    },
                )),
        )
        .child(
            when(picker, move || {
                view()
                    .flex_1()
                    .min_h_0()
                    .child(feedback(
                        candidates_busy,
                        candidates_failed,
                        true,
                        move || candidates_retry.update(|n| *n += 1),
                    ))
                    .child(when(
                        move || {
                            !candidates_busy.get()
                                && !candidates_failed.get()
                                && candidates.with(Vec::is_empty)
                        },
                        move || empty(move || tr("No senders found", "未找到发送者")),
                    ))
                    .child(when(
                        move || !candidates.with(Vec::is_empty),
                        move || {
                            virtual_list(
                                move || candidates.with(Vec::len),
                                move |at| {
                                    ui::command()
                                        .justify_start()
                                        .px_4()
                                        .py_3()
                                        .min_h_16()
                                        .on_press(move || {
                                            if let Some(sender) =
                                                candidates.with_untracked(|rows| {
                                                    rows.get(at.index()).cloned()
                                                })
                                            {
                                                dismiss_keyboard();
                                                batch(|| {
                                                    results
                                                        .sender
                                                        .set(Some(sender.user_id.clone()));
                                                    selected.set(Some(sender));
                                                    picker.set(false);
                                                    sender_query.set(String::new());
                                                });
                                            }
                                        })
                                        .child(ui::avatar_view(
                                            move || {
                                                candidates.with(|rows| {
                                                    rows.get(at.index())
                                                        .map(|sender| sender.full_name.clone())
                                                        .unwrap_or_default()
                                                        .into()
                                                })
                                            },
                                            move || {
                                                candidates.with(|rows| {
                                                    rows.get(at.index())
                                                        .map(|sender| sender.avatar_url.clone())
                                                        .unwrap_or_default()
                                                })
                                            },
                                        ))
                                        .child(
                                            text(move || {
                                                candidates.with(|rows| {
                                                    rows.get(at.index())
                                                        .map(|sender| sender.full_name.clone())
                                                        .unwrap_or_default()
                                                })
                                            })
                                            .text_sm()
                                            .number_of_lines(1)
                                            .flex_1()
                                            .min_w_0(),
                                        )
                                },
                            )
                            .keys(move || {
                                candidates.with(|rows| {
                                    rows.iter()
                                        .map(|row| row.user_id.clone())
                                        .collect::<Vec<_>>()
                                })
                            })
                            .estimated_item_size(px(64.0))
                            .flex_1()
                            .min_h_0()
                        },
                    ))
            })
            .otherwise(move || {
                view()
                    .flex_1()
                    .min_h_0()
                    .child(feedback(results.busy, results.failed, false, move || {
                        results.schedule(results.last.with(|load| *load))
                    }))
                    .child(when(
                        move || {
                            !results.busy.get()
                                && !results.failed.get()
                                && results.rows.with(Vec::is_empty)
                        },
                        move || {
                            empty(move || {
                                if results.query.with(|query| query.trim().is_empty())
                                    && results.sender.with(Option::is_none)
                                {
                                    tr("Enter a keyword to search", "输入关键词搜索")
                                } else {
                                    tr("No messages found", "未找到消息")
                                }
                            })
                        },
                    ))
                    .child(when(
                        move || !results.rows.with(Vec::is_empty),
                        move || {
                            virtual_list(
                                move || results.rows.with(Vec::len),
                                move |at| {
                                    ui::command()
                                        .justify_start()
                                        .px_4()
                                        .py_3()
                                        .min_h_16()
                                        .on_press(move || {
                                            if let Some(id) = results.rows.with_untracked(|rows| {
                                                rows.get(at.index())
                                                    .map(|row| row.message_id.clone())
                                            }) {
                                                batch(|| {
                                                    navigation.highlight_search(
                                                        results
                                                            .conversation
                                                            .with(|value| value.clone()),
                                                        results.query.get_untracked(),
                                                    );
                                                    target.set(Some(id));
                                                    navigation.panel.set(None);
                                                });
                                            }
                                        })
                                        .child(
                                            view()
                                                .gap_1()
                                                .flex_1()
                                                .min_w_0()
                                                .child(
                                                    text(move || {
                                                        results.rows.with(|rows| {
                                                            rows.get(at.index())
                                                                .map(|row| row.sender_name.clone())
                                                                .unwrap_or_default()
                                                        })
                                                    })
                                                    .text_sm()
                                                    .number_of_lines(1),
                                                )
                                                .child(
                                                    text(move || {
                                                        results.rows.with(|rows| {
                                                            rows.get(at.index())
                                                                .map(|row| {
                                                                    presentation::preview(
                                                                        i18n,
                                                                        &row.category,
                                                                        &row.content,
                                                                    )
                                                                })
                                                                .unwrap_or_default()
                                                        })
                                                    })
                                                    .text_xs()
                                                    .number_of_lines(2)
                                                    .text_color(move || theme().muted_foreground),
                                                ),
                                        )
                                },
                            )
                            .keys(move || {
                                results.rows.with(|rows| {
                                    rows.iter()
                                        .map(|row| row.message_id.clone())
                                        .collect::<Vec<_>>()
                                })
                            })
                            .estimated_item_size(px(64.0))
                            .flex_1()
                            .min_h_0()
                            .on_viewable_items_change(move |event: &ViewableItems| {
                                if let Some((first, _)) = event.viewable {
                                    results.visible.replace(results.rows.with_untracked(|rows| {
                                        rows.get(first).map(|row| row.message_id.clone())
                                    }));
                                }
                            })
                            .on_load_more(move || {
                                if results.older.get_untracked() {
                                    results.schedule(Load::Older);
                                }
                            })
                            .on_load_more_at_start(move || {
                                if results.newer.get_untracked() {
                                    results.schedule(Load::Newer);
                                }
                            })
                        },
                    ))
            }),
        )
}

fn category_button(
    results: Results,
    kind: &'static str,
    label: impl Fn() -> &'static str + 'static,
) -> impl IntoElement {
    ui::command()
        .min_h_8()
        .py_1()
        .rounded_full()
        .selected(move || results.category.get() == Some(kind))
        .bg(move || {
            if results.category.get() == Some(kind) {
                theme().primary
            } else {
                theme().muted
            }
        })
        .text_color(move || {
            if results.category.get() == Some(kind) {
                theme().primary_foreground
            } else {
                theme().foreground
            }
        })
        .on_press(move || {
            results.category.update(|category| {
                *category = if *category == Some(kind) {
                    None
                } else {
                    Some(kind)
                }
            })
        })
        .child(text(label).text_sm())
}

fn empty(label: impl Fn() -> &'static str + 'static) -> impl IntoElement {
    view()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p_4()
        .items_center()
        .justify_center()
        .child(
            text(label)
                .text_sm()
                .text_color(move || theme().muted_foreground),
        )
}

fn feedback(
    busy: Signal<bool>,
    failed: Signal<bool>,
    senders: bool,
    retry: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    view()
        .px_4()
        .gap_2()
        .child(when(busy, move || {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .py_2()
                .child(ui::busy_indicator(
                    move || busy.get(),
                    move || {
                        if senders {
                            tr("Loading senders…", "正在加载发送者…")
                        } else {
                            i18n.text(Key::LoadingMessages)
                        }
                        .into()
                    },
                ))
                .child(
                    text(move || {
                        if senders {
                            tr("Loading senders…", "正在加载发送者…")
                        } else {
                            i18n.text(Key::LoadingMessages)
                        }
                    })
                    .text_sm()
                    .text_color(move || theme().muted_foreground),
                )
        }))
        .child(when(failed, move || {
            view()
                .gap_2()
                .py_2()
                .child(
                    text(move || {
                        if senders {
                            tr("Couldn’t load senders", "无法加载发送者")
                        } else {
                            i18n.text(Key::MessagesFailed)
                        }
                    })
                    .text_sm()
                    .text_color(move || theme().destructive),
                )
                .child(ui::action(move || i18n.text(Key::Retry), retry))
        }))
}
