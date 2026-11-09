//! Conversation-scoped search and its bounded, cursor-paged presentation window.

use std::{collections::HashSet, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, ClientResult, ConversationParticipantItem, MessageListView,
};
use nsh_core::prelude::*;

use crate::presentation::message_window::splice_phase;
use crate::{
    i18n::{self, Key, tr},
    navigation::ChatNavigation,
    presentation, runtime,
    ui::parts,
    ui::{self, Typography as _},
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
    account: Stored<Option<Arc<AccountClient>>>,
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
    fn idle(self) -> bool {
        self.query.with_untracked(|query| query.trim().is_empty())
            && self.sender.with_untracked(Option::is_none)
    }

    fn schedule(self, load: Load) {
        if matches!(load, Load::Refresh) && self.idle() {
            return;
        }
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

#[derive(Clone)]
struct Search {
    account: Arc<AccountClient>,
    id: String,
    query: String,
    sender: Option<String>,
    category: Option<&'static str>,
}

impl Search {
    async fn page(&self, index: usize, anchor: Option<String>) -> ClientResult<Page> {
        let categories = self
            .category
            .map(|kind| {
                ["PLAIN", "SIGNAL", "ENCRYPTED"]
                    .map(|prefix| format!("{prefix}_{kind}"))
                    .to_vec()
            })
            .unwrap_or_default();
        let mut rows = self
            .account
            .message()
            .search_messages(
                self.id.clone(),
                self.query.clone(),
                self.sender.clone(),
                categories,
                anchor.clone(),
                PAGE_SIZE + 1,
            )
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

    async fn refresh(&self, pages: Vec<Page>) -> ClientResult<Vec<Page>> {
        let mut refreshed = Vec::new();
        let mut anchor = pages.first().and_then(|page| page.anchor.clone());
        for page in pages {
            let page = self.page(page.index, anchor).await?;
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
            return Ok(vec![self.page(0, None).await?]);
        }
        Ok(refreshed)
    }

    async fn older(&self, mut pages: Vec<Page>) -> ClientResult<Vec<Page>> {
        let index = pages.last().map_or(0, |page| page.index + 1);
        let anchor = pages
            .last()
            .and_then(|page| page.rows.last().map(|row| row.message_id.clone()));
        let page = self.page(index, anchor).await?;
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
        Ok(pages)
    }

    async fn newer(
        &self,
        retained: Vec<Page>,
        cursors: Vec<Option<String>>,
    ) -> ClientResult<Vec<Page>> {
        let index = retained
            .first()
            .map_or(0, |page| page.index.saturating_sub(1));
        let page = self
            .page(index, cursors.get(index).cloned().flatten())
            .await?;
        if page.index > 0 && page.rows.is_empty() {
            return Ok(vec![self.page(0, None).await?]);
        }
        let count = (retained.len() + 1).min(WINDOW_PAGES);
        let mut pages = vec![page];
        // Rebuild adjacent boundaries after inserts/deletions so the
        // restored page cannot leave a hole before retained results.
        while pages.len() < count && pages.last().is_some_and(|page| page.more) {
            let anchor = pages
                .last()
                .and_then(|page| page.rows.last())
                .map(|row| row.message_id.clone());
            pages.push(self.page(index + pages.len(), anchor).await?);
        }
        Ok(pages)
    }

    async fn load(
        self,
        load: Load,
        pages: Vec<Page>,
        cursors: Vec<Option<String>>,
    ) -> ClientResult<Vec<Page>> {
        match load {
            Load::Reset => Ok(vec![self.page(0, None).await?]),
            Load::Older => self.older(pages).await,
            Load::Newer => self.newer(pages, cursors).await,
            Load::Refresh => self.refresh(pages).await,
        }
    }
}

/// The sender picker, which reloads through `retry` while it is open.
#[derive(Clone, Copy)]
struct Senders {
    open: Signal<bool>,
    query: Signal<String>,
    selected: Signal<Option<ConversationParticipantItem>>,
    rows: Signal<Vec<ConversationParticipantItem>>,
    busy: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
}

impl Senders {
    fn reload(self) {
        self.retry.update(|revision| *revision += 1);
    }

    fn close(self) {
        batch(|| {
            self.open.set(false);
            self.query.set(String::new());
        });
    }
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    id: String,
    close: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
) -> impl IntoElement {
    let senders = Senders {
        open: signal(false),
        query: signal(String::new()),
        selected: signal_with(None, SignalEquals::Never),
        rows: signal_with(Vec::new(), SignalEquals::Never),
        busy: signal(false),
        failed: signal(false),
        retry: signal(0),
    };
    let input = element_ref();
    let results = Results {
        account: stored(Some(account.clone())),
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
        let search = Search {
            account: results
                .account
                .with(|account| account.clone().expect("live search has an account")),
            id: results.conversation.with(|id| id.clone()),
            query: results.query.get_untracked().trim().to_owned(),
            sender: results.sender.get_untracked(),
            category: results.category.get_untracked(),
        };
        let pages = results.pages.with(|pages| pages.clone());
        let cursors = results.cursors.with(|cursors| cursors.clone());
        spawn(async move {
            if search.query.is_empty() && search.sender.is_none() {
                results.complete();
                return;
            }
            if matches!(load, Load::Reset) {
                timer(parts::SEARCH_DELAY).await;
            }
            let loaded = runtime::run(search.load(load, pages, cursors)).await;
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
    load_senders(account.clone(), id.clone(), senders);
    let changes_account = account.clone();
    let message_conversation = id.clone();
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(changes_account.conversation_changes()));
        while let Some(change) = changes.next().await {
            if senders.open.get_untracked()
                && (change.reload_all || change.conversation_ids.contains(&id))
            {
                senders.reload();
            }
        }
    });
    spawn(async move {
        parts::throttled(
            Box::pin(runtime::observe(
                account.message_changes(Some(message_conversation)),
            )),
            move || {
                results.schedule(Load::Refresh);
                if senders.open.get_untracked() {
                    senders.reload();
                }
            },
        )
        .await;
    });
    let back = move || {
        if senders.open.get_untracked() {
            dismiss_keyboard();
            senders.close();
        } else {
            close();
        }
    };
    content(results, senders, input, back, target)
}

fn load_senders(account: Arc<AccountClient>, id: String, senders: Senders) {
    effect(move || {
        let is_open = senders.open.get();
        let keyword = senders.query.get();
        senders.retry.get();
        if !is_open {
            return;
        }
        let account = account.clone();
        let id = id.clone();
        batch(|| {
            senders.busy.set(true);
            senders.failed.set(false);
        });
        spawn(async move {
            timer(parts::SEARCH_DELAY).await;
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
            batch(|| {
                match answer {
                    Ok(rows) => senders.rows.set(rows),
                    Err(_) => senders.failed.set(true),
                }
                senders.busy.set(false);
            });
        });
    });
}

fn filters(
    results: Results,
    senders: Senders,
    input: ElementRef,
    back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let picker = senders.open;
    view()
        .flex_none()
        .px(ui::space_lg())
        .py(ui::space_md())
        .gap(ui::space_sm())
        .child(when(
            move || !picker.get() && senders.selected.with(Option::is_some),
            move || sender_chip(results.sender, senders.selected, input),
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(ui::search_pill().child(dynamic(move || {
                    let is_picker = picker.get();
                    search_field(results, senders, input, is_picker, back)
                })))
                .child(when(
                    move || !picker.get(),
                    move || {
                        ui::round_button()
                            .size_8()
                            .accessibility_label(move || {
                                Some(tr("Choose a sender", "选择发送者").into())
                            })
                            .on_press(move || {
                                dismiss_keyboard();
                                batch(|| {
                                    senders.query.set(String::new());
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
                    .child(category_button(results.category, "TEXT", move || {
                        tr("Text", "文字")
                    }))
                    .child(category_button(results.category, "POST", move || {
                        tr("Post", "长文")
                    }))
            },
        ))
}

fn sender_chip(
    sender: Signal<Option<String>>,
    selected: Signal<Option<ConversationParticipantItem>>,
    input: ElementRef,
) -> impl IntoElement {
    ui::button(ui::ButtonTone::Secondary, ui::ButtonSize::Regular)
        .justify_start()
        .w_full()
        .accessibility_label(move || Some(tr("Clear sender", "清除发送者").into()))
        .on_press(move || {
            selected.set(None);
            sender.set(None);
            input.focus();
        })
        .child(
            text(move || {
                format!(
                    "{}: {}",
                    tr("From", "发送者"),
                    selected.with(|sender| sender
                        .as_ref()
                        .map(|sender| sender.full_name.clone())
                        .unwrap_or_default())
                )
            })
            .text_role(ui::TextRole::Body)
            .number_of_lines(1)
            .flex_1()
            .min_w_0(),
        )
        .child(ui::icon(ui::Icon::Close))
}

fn search_field(
    results: Results,
    senders: Senders,
    input: ElementRef,
    is_picker: bool,
    back: impl Fn() + Copy + 'static,
) -> Input {
    let i18n = i18n::current();
    let label = move || {
        if is_picker {
            tr("Search senders", "搜索发送者")
        } else {
            i18n.text(Key::SearchMessages)
        }
    };
    ui::search_input()
        .key(if is_picker {
            "sender-search"
        } else {
            "message-search"
        })
        .element_ref(&input)
        .auto_focus(true)
        .placeholder(move || label().to_owned())
        .label(label)
        .value(move || {
            if is_picker {
                senders.query.get()
            } else {
                results.query.get()
            }
        })
        .on_change(move |value: &str| {
            if is_picker {
                senders.query.set(value.to_owned());
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
            if is_picker && senders.query.with_untracked(String::is_empty) {
                back();
            } else if !is_picker
                && results.query.with_untracked(String::is_empty)
                && results.sender.with_untracked(Option::is_some)
            {
                batch(|| {
                    senders.selected.set(None);
                    results.sender.set(None);
                });
            }
        })
}

fn sender_list(results: Results, senders: Senders) -> impl IntoElement {
    let rows = senders.rows;
    view()
        .flex_1()
        .min_h_0()
        .child(when(
            move || senders.busy.get() && rows.with(Vec::is_empty),
            || {
                parts::status(move || tr("Loading senders…", "正在加载发送者…"))
                    .px_4()
                    .py_2()
            },
        ))
        .child(when(senders.failed, move || {
            sender_error(move || senders.reload()).px_4().py_2()
        }))
        .child(when(
            move || !senders.busy.get() && !senders.failed.get() && rows.with(Vec::is_empty),
            || ui::placeholder(move || tr("No senders found", "未找到发送者")),
        ))
        .child(when(
            move || !rows.with(Vec::is_empty),
            move || {
                virtual_list(
                    move || rows.with(Vec::len),
                    move |at| {
                        let sender = move || rows.with(|rows| rows.get(at.index()).cloned());
                        parts::entity_row(
                            move || sender().map(|sender| sender.user_id).unwrap_or_default(),
                            move || sender().map(|sender| sender.full_name).unwrap_or_default(),
                            move || sender().map(|sender| sender.avatar_url).unwrap_or_default(),
                            String::new,
                            || false,
                        )
                        .on_press(move || {
                            if let Some(sender) = untracked(sender) {
                                dismiss_keyboard();
                                batch(|| {
                                    results.sender.set(Some(sender.user_id.clone()));
                                    senders.selected.set(Some(sender));
                                    senders.close();
                                });
                            }
                        })
                    },
                )
                .keys(move || {
                    rows.with(|rows| {
                        rows.iter()
                            .map(|row| row.user_id.clone())
                            .collect::<Vec<_>>()
                    })
                })
                .estimated_item_size(parts::row_height())
                .flex_1()
                .min_h_0()
            },
        ))
}

fn result_list(results: Results, target: Signal<Option<String>>) -> impl IntoElement {
    let i18n = i18n::current();
    let navigation = use_context::<ChatNavigation>().expect("Home provides chat navigation");
    let rows = results.rows;
    view()
        .flex_1()
        .min_h_0()
        .child(when(
            move || results.busy.get() && rows.with(Vec::is_empty),
            move || {
                parts::status(move || i18n.text(Key::LoadingMessages))
                    .px_4()
                    .py_2()
            },
        ))
        .child(when(results.failed, move || {
            results_error(move || results.schedule(results.last.with(|load| *load)))
                .px_4()
                .py_2()
        }))
        .child(when(
            move || !results.busy.get() && !results.failed.get() && rows.with(Vec::is_empty),
            move || {
                ui::placeholder(move || {
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
            move || !rows.with(Vec::is_empty),
            move || {
                virtual_list(
                    move || rows.with(Vec::len),
                    move |at| {
                        let field = move |read: fn(&MessageListView) -> String| {
                            rows.with(|rows| rows.get(at.index()).map(read).unwrap_or_default())
                        };
                        parts::compact_row()
                            .py(ui::space_sm())
                            .on_press(move || {
                                let id = rows.with_untracked(|rows| {
                                    rows.get(at.index()).map(|row| row.message_id.clone())
                                });
                                if let Some(id) = id {
                                    batch(|| {
                                        navigation.highlight_search(
                                            results.conversation.with(|value| value.clone()),
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
                                        text(move || field(|row| row.sender_name.clone()))
                                            .text_role(ui::TextRole::Body)
                                            .number_of_lines(1),
                                    )
                                    .child(
                                        text(move || {
                                            rows.with(|rows| {
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
                                        .text_role(ui::TextRole::Caption)
                                        .number_of_lines(2)
                                        .text_color(ui::fg_secondary),
                                    ),
                            )
                    },
                )
                .keys(move || {
                    rows.with(|rows| {
                        rows.iter()
                            .map(|row| row.message_id.clone())
                            .collect::<Vec<_>>()
                    })
                })
                .estimated_item_size(parts::row_height())
                .flex_1()
                .min_h_0()
                .on_viewable_items_change(move |event: &ViewableItems| {
                    if let Some((first, _)) = event.viewable {
                        results.visible.replace(rows.with_untracked(|rows| {
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
}

fn category_button(
    category: Signal<Option<&'static str>>,
    kind: &'static str,
    label: impl Fn() -> &'static str + 'static,
) -> impl IntoElement {
    ui::button(ui::ButtonTone::Secondary, ui::ButtonSize::Small)
        .rounded(ui::radius_full())
        .selected(move || category.get() == Some(kind))
        .on_press(move || {
            category.update(|category| {
                *category = if *category == Some(kind) {
                    None
                } else {
                    Some(kind)
                }
            })
        })
        .child(text(label).text_role(ui::TextRole::Body))
}

pub(crate) fn sender_error(retry: impl Fn() + 'static) -> View {
    parts::failure(move || tr("Couldn’t load senders", "无法加载发送者"), retry)
}

pub(crate) fn results_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    parts::failure(move || i18n.text(Key::MessagesFailed), retry)
}

fn content(
    results: Results,
    senders: Senders,
    input: ElementRef,
    back: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
) -> View {
    let i18n = i18n::current();
    view()
        .flex_1()
        .min_h_0()
        .child(parts::page_header(
            move || {
                if senders.open.get() {
                    tr("Choose a sender", "选择发送者")
                } else {
                    i18n.text(Key::SearchMessages)
                }
            },
            Some(
                parts::back_button(back)
                    .style_when(move || !senders.open.get(), |style| style.hidden()),
            ),
            Some(
                parts::close_button(back)
                    .style_when(move || senders.open.get(), |style| style.hidden()),
            ),
        ))
        .child(filters(results, senders, input, back))
        .child(
            when(senders.open, move || sender_list(results, senders))
                .otherwise(move || result_list(results, target)),
        )
}

#[cfg(debug_assertions)]
pub(crate) fn preview(
    rows: Vec<MessageListView>,
    members: Vec<ConversationParticipantItem>,
    state: Signal<&'static str>,
) -> View {
    let failed = signal(false);
    let sender_failed = signal(false);
    let senders = Senders {
        open: signal(false),
        query: signal(String::new()),
        selected: signal_with(None, SignalEquals::Never),
        rows: signal_with(members, SignalEquals::Never),
        busy: signal(false),
        failed: sender_failed,
        retry: signal(0),
    };
    let results = Results {
        account: stored(None),
        conversation: stored("preview-group".into()),
        query: signal("design".into()),
        sender: signal(None),
        category: signal(None),
        rows: signal_with(rows, SignalEquals::Never),
        pages: stored(Vec::new()),
        cursors: stored(vec![None]),
        request: signal_with(None, SignalEquals::Never),
        revision: signal(0),
        busy: signal(false),
        failed,
        older: signal(false),
        newer: signal(false),
        visible: stored(None),
        dirty: stored(false),
        deferred: stored(None),
        last: stored(Load::Reset),
    };
    effect(move || {
        failed.set(state.get() == "Error");
        sender_failed.set(state.get() == "Sender error");
        senders
            .open
            .set(matches!(state.get(), "Senders" | "Sender error"));
    });
    content(results, senders, element_ref(), || {}, signal(None))
}
