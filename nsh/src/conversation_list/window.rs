//! A bounded account conversation window; the database owns filtering and ordering.
use std::{sync::Arc, time::Duration};

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::{element::ViewableItems, prelude::*};

use crate::{
    navigation::ChatNavigation, presentation::message_window::splice_by, runtime,
    sidebar::Navigation,
};

const PAGE_SIZE: i64 = 100;
const WINDOW_SIZE: i64 = PAGE_SIZE * 3;
const REFRESH_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, PartialEq)]
struct Scope {
    category: &'static str,
    circle: Option<String>,
    unread: bool,
}

#[derive(Clone)]
enum Load {
    Reset,
    Refresh,
    Next,
    Previous,
    Locate(String),
}

struct Loaded {
    rows: Vec<ConversationListData>,
    offset: i64,
    count: i64,
    more: bool,
    locate: Option<String>,
    paging: bool,
    reset: bool,
    // The reader is at the list's top: what arrives above must show, not push it down.
    top: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Window {
    account: Stored<Arc<AccountClient>>,
    // The list's scope and its reset revision; kept while search covers the list.
    scope: Stored<Option<(u64, Scope)>>,
    searching: Stored<bool>,
    navigation: ChatNavigation,
    pub rows: Signal<Vec<ConversationListData>>,
    pub loading: Signal<bool>,
    pub failed: Signal<bool>,
    pub list: ElementRef,
    offset: Signal<i64>,
    count: Signal<i64>,
    more: Signal<bool>,
    request: Signal<u64>,
    step_request: Signal<u64>,
    visible: Stored<Option<String>>,
    viewable: Stored<Option<(usize, usize)>>,
    dirty: Stored<bool>,
    deferred: Stored<Option<Load>>,
    running: Stored<Option<Spawned<()>>>,
    owner: Owner,
}

impl Window {
    pub(crate) fn mount(
        account: Arc<AccountClient>,
        nav: Navigation,
        query: Signal<String>,
        composing: Signal<bool>,
        unread: Signal<bool>,
    ) -> Self {
        let navigation = use_context::<ChatNavigation>().expect("Home provides chat navigation");
        let state = Self {
            account: stored(account.clone()),
            scope: stored(None),
            searching: stored(false),
            navigation,
            rows: signal(Vec::new()),
            loading: signal(false),
            failed: signal(false),
            list: element_ref(),
            offset: signal(0),
            count: signal(PAGE_SIZE),
            more: signal(false),
            request: signal(0),
            step_request: signal(0),
            visible: stored(None),
            viewable: stored(None),
            dirty: stored(false),
            deferred: stored(None),
            running: stored(None),
            owner: Owner::current(),
        };
        effect(move || {
            let revision = nav.list_reset.get();
            let scope = Scope {
                category: nav.category.get(),
                circle: nav.circle.get(),
                unread: unread.get(),
            };
            let searching = composing.get() || !query.with(|query| query.trim().is_empty());
            state.cancel();
            let unchanged = state.scope.with(|current| {
                current
                    .as_ref()
                    .is_some_and(|c| c.0 == revision && c.1 == scope)
            });
            state.scope.replace(Some((revision, scope)));
            state.searching.replace(searching);
            if searching {
                state.failed.set(false);
            } else if !unchanged {
                state.load(Load::Reset);
            } else {
                // Leaving search keeps the reader's place, and shows what it opened.
                match navigation.selected_id.get_untracked() {
                    Some(id) if !state.contains(&id) => state.load(Load::Locate(id)),
                    _ => state.load(Load::Refresh),
                }
            }
        });
        effect(move || {
            if let Some(id) = navigation.selected_id.get() {
                if state.contains(&id) {
                    state.scroll_to(id);
                } else {
                    state.load(Load::Locate(id));
                }
            }
        });
        state.owner.run(move || {
            spawn(async move {
                let mut changes = Box::pin(runtime::throttled(
                    account.conversation_changes(),
                    REFRESH_INTERVAL,
                ));
                while changes.next().await.is_some() {
                    state.load(Load::Refresh);
                }
            });
        });
        state
    }

    fn is_selected(self, id: &str) -> bool {
        self.navigation
            .selected_id
            .with_untracked(|selected| selected.as_deref() == Some(id))
    }

    fn contains(self, id: &str) -> bool {
        self.rows
            .with_untracked(|rows| rows.iter().any(|row| row.conversation_id == id))
    }

    fn active_scope(self) -> Option<Scope> {
        if self.searching.get() {
            return None;
        }
        self.scope.with(|scope| scope.as_ref().map(|s| s.1.clone()))
    }

    fn cancel(self) {
        self.request
            .update(|request| *request = request.wrapping_add(1));
        if let Some(task) = self.running.replace(None) {
            task.cancel();
        }
        self.loading.set(false);
        self.dirty.replace(false);
        self.deferred.replace(None);
    }

    pub(crate) fn retry(self) {
        self.load(Load::Refresh);
    }
    pub(crate) fn next(self) {
        if self.more.get_untracked() {
            self.load(Load::Next);
        }
    }
    pub(crate) fn previous(self) {
        if self.offset.get_untracked() > 0 {
            self.load(Load::Previous);
        }
    }

    pub(crate) fn viewable(self, items: &ViewableItems) {
        // A list hidden behind search or Settings reports no rows; keep the place.
        let Some((first, last)) = items.viewable else {
            return;
        };
        self.viewable.replace(Some((first, last)));
        self.visible.replace(
            self.rows
                .with_untracked(|rows| rows.get(first).map(|row| row.conversation_id.clone())),
        );
    }

    fn load(self, load: Load) {
        let Some(scope) = self.active_scope() else {
            return;
        };
        if matches!(load, Load::Next) && !self.more.get_untracked()
            || matches!(load, Load::Previous) && self.offset.get_untracked() == 0
        {
            return;
        }
        if self.loading.get_untracked() {
            match load {
                Load::Refresh => {
                    self.dirty.replace(true);
                    return;
                }
                Load::Next | Load::Previous => {
                    self.deferred.replace(Some(load));
                    return;
                }
                _ => self.cancel(),
            }
        }
        let previous_offset = self.offset.get_untracked();
        let current_count = self.count.get_untracked();
        let (anchor, anchor_at) = self.rows.with_untracked(|rows| {
            let anchor = self
                .visible
                .with(|id| id.clone())
                .or_else(|| rows.first().map(|row| row.conversation_id.clone()));
            let at = anchor
                .as_ref()
                .and_then(|id| rows.iter().position(|row| row.conversation_id == *id));
            (anchor, at.unwrap_or_default() as i64)
        });
        let reset = matches!(load, Load::Reset);
        let top = matches!(load, Load::Refresh) && previous_offset == 0 && anchor_at == 0;
        let selected = self.navigation.selected_id.get_untracked();
        let ticket = self.request.get_untracked().wrapping_add(1);
        batch(|| {
            self.request.set(ticket);
            self.loading.set(true);
            self.failed.set(false);
        });
        // Reset keeps the old rows until the new ones arrive, without a blank frame.
        if reset {
            self.visible.replace(None);
            self.viewable.replace(None);
        }
        let account = self.account.with(|account| account.clone());
        let task = self.owner.run(move || {
            spawn(async move {
                let result = runtime::run(async move {
                    let access = account.conversation();
                    let locate = match &load {
                        Load::Locate(id) => Some(id.clone()),
                        Load::Reset => selected,
                        _ => None,
                    };
                    let count = match load {
                        Load::Reset => PAGE_SIZE,
                        Load::Next => (current_count + PAGE_SIZE).min(WINDOW_SIZE),
                        _ => current_count,
                    };
                    let mut offset = match load {
                        Load::Reset => 0,
                        Load::Next if current_count == WINDOW_SIZE => {
                            previous_offset.saturating_add(PAGE_SIZE)
                        }
                        Load::Previous => previous_offset.saturating_sub(PAGE_SIZE).max(0),
                        _ => previous_offset,
                    };
                    // At the top a new conversation must appear above the reader.
                    let anchored = matches!(load, Load::Refresh) && !top;
                    if let Some(id) = locate
                        .as_ref()
                        .or_else(|| anchored.then_some(anchor.as_ref()).flatten())
                        && let Some(position) = access
                            .conversation_position(
                                scope.category.into(),
                                scope.circle.clone(),
                                String::new(),
                                scope.unread,
                                id.clone(),
                            )
                            .await?
                    {
                        offset = if locate.is_some() {
                            position.saturating_sub(count / 2).max(0)
                        } else {
                            position.saturating_sub(anchor_at).max(0)
                        };
                    }
                    let mut rows = access
                        .conversations(
                            scope.category.into(),
                            scope.circle.clone(),
                            String::new(),
                            scope.unread,
                            count + 1,
                            offset,
                        )
                        .await?;
                    if rows.len() < count as usize && offset > 0 {
                        let total = access
                            .conversation_count(
                                scope.category.into(),
                                scope.circle.clone(),
                                String::new(),
                                scope.unread,
                            )
                            .await?;
                        offset = total.saturating_sub(count).max(0);
                        rows = access
                            .conversations(
                                scope.category.into(),
                                scope.circle,
                                String::new(),
                                scope.unread,
                                count + 1,
                                offset,
                            )
                            .await?;
                    }
                    let more = rows.len() > count as usize;
                    rows.truncate(count as usize);
                    Ok(Loaded {
                        rows,
                        offset,
                        count,
                        more,
                        locate,
                        paging: matches!(load, Load::Next | Load::Previous),
                        reset,
                        top,
                    })
                })
                .await;
                if !self.owner.is_live() || self.request.get_untracked() != ticket {
                    return;
                }
                match result {
                    Ok(page) => self.apply(page, ticket),
                    Err(error) => {
                        log::warn!("Could not load conversation window: {error}");
                        batch(|| {
                            if reset {
                                self.rows.set(Vec::new());
                                self.offset.set(0);
                                self.count.set(PAGE_SIZE);
                                self.more.set(false);
                            }
                            self.failed.set(true);
                        });
                        self.complete();
                    }
                }
            })
        });
        self.running.replace(Some(task));
    }

    fn apply(self, page: Loaded, ticket: u64) {
        let previous = self.rows.get_untracked();
        let anchor = self
            .visible
            .with(|id| id.clone())
            .filter(|_| !page.reset && !page.top);
        // An opposite-edge scroll must not evict the row reached meanwhile.
        if page.paging
            && anchor
                .as_ref()
                .is_some_and(|id| !page.rows.iter().any(|row| row.conversation_id == *id))
        {
            self.complete();
            return;
        }
        if let Some(phase) = splice_by(&previous, &page.rows, anchor.as_deref(), |row| {
            row.conversation_id.as_str()
        }) {
            self.rows.set(phase);
            after_answer(move || {
                if self.owner.is_live() && self.request.get_untracked() == ticket {
                    self.apply(page, ticket);
                }
            });
            return;
        }
        log::debug!(
            "NSH conversation window loaded: {} rows at offset {}",
            page.rows.len(),
            page.offset
        );
        let new_first = previous.first().map(|row| &row.conversation_id)
            != page.rows.first().map(|row| &row.conversation_id);
        batch(|| {
            self.offset.set(page.offset);
            self.count.set(page.count);
            self.more.set(page.more);
            self.rows.set(page.rows);
        });
        if let Some(id) = page.locate {
            self.scroll_to(id);
        } else if page.reset || (page.top && new_first) {
            after_answer(move || {
                if self.owner.is_live() {
                    self.list.scroll_to_offset(px(0.0), false);
                }
            });
        }
        self.complete();
    }

    fn complete(self) {
        self.loading.set(false);
        if self.failed.get_untracked() {
            self.deferred.replace(None);
            self.dirty.replace(false);
            return;
        }
        if self.dirty.with(std::mem::take) {
            self.load(Load::Refresh);
        } else if let Some(load) = self.deferred.replace(None) {
            self.load(load);
        }
    }

    fn scroll_to(self, id: String) {
        after_answer(move || {
            if !self.owner.is_live() || !self.is_selected(&id) {
                return;
            }
            // A row already in view stays under the pointer that chose it.
            if let Some(index) = self
                .rows
                .with_untracked(|rows| rows.iter().position(|row| row.conversation_id == id))
                && !self
                    .viewable
                    .get()
                    .is_some_and(|(first, last)| (first..=last).contains(&index))
            {
                self.list
                    .scroll_to_index(index as u32, percentage(0.5), px(0.0), false);
            }
        });
    }

    pub(crate) fn step(self, forward: bool) {
        let Some(scope) = self.active_scope() else {
            return;
        };
        let Some(id) = self.navigation.selected_id.get_untracked() else {
            return;
        };
        let local = self.rows.with_untracked(|rows| {
            let at = rows.iter().position(|row| row.conversation_id == id)?;
            let next = if forward {
                at.checked_add(1)?
            } else {
                at.checked_sub(1)?
            };
            rows.get(next).cloned()
        });
        if let Some(row) = local {
            self.navigation.select(row);
            return;
        }
        let ticket = self.step_request.get_untracked().wrapping_add(1);
        self.step_request.set(ticket);
        let version = self.request.get_untracked();
        let account = self.account.with(|account| account.clone());
        self.owner.run(move || {
            spawn(async move {
                let query_id = id.clone();
                let result = runtime::run(async move {
                    let access = account.conversation();
                    let Some(at) = access
                        .conversation_position(
                            scope.category.into(),
                            scope.circle.clone(),
                            String::new(),
                            scope.unread,
                            query_id,
                        )
                        .await?
                    else {
                        return Ok(None);
                    };
                    let next = if forward {
                        at.checked_add(1)
                    } else {
                        at.checked_sub(1).filter(|at| *at >= 0)
                    };
                    let Some(next) = next else { return Ok(None) };
                    Ok(access
                        .conversations(
                            scope.category.into(),
                            scope.circle,
                            String::new(),
                            scope.unread,
                            1,
                            next,
                        )
                        .await?
                        .into_iter()
                        .next())
                })
                .await;
                if !self.owner.is_live()
                    || self.step_request.get_untracked() != ticket
                    || self.request.get_untracked() != version
                    || !self.is_selected(&id)
                {
                    return;
                }
                match result {
                    Ok(Some(row)) => self.navigation.select(row),
                    Ok(None) => {}
                    Err(_) => self.failed.set(true),
                }
            })
        });
    }
}
