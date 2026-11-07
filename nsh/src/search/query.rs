use super::*;

use std::time::Duration;

const DEBOUNCE: Duration = Duration::from_millis(150);
const REFRESH_INTERVAL: Duration = Duration::from_millis(333);

#[derive(Clone, Copy)]
pub(super) struct State {
    pub(super) section: Signal<Section>,
    /// Pages of a single section loaded so far; scrolling to the end adds one.
    pub(super) pages: Signal<i64>,
    pub(super) more: Signal<bool>,
    extending: Signal<bool>,
    pub(super) rows: Signal<Vec<Row>>,
    pub(super) users_empty: Signal<bool>,
    pub(super) loading: Signal<bool>,
    pub(super) failed: Signal<bool>,
    pub(super) revision: Signal<u64>,
    pub(super) list: ElementRef,
}

impl State {
    pub(super) fn show(self, section: Section) {
        batch(|| {
            self.section.set(section);
            self.pages.set(1);
        });
    }

    pub(super) fn load_more(self) {
        if self.section.get_untracked() == Section::All
            || !self.more.get_untracked()
            || self.loading.get_untracked()
            || self.extending.get_untracked()
        {
            return;
        }
        self.extending.set(true);
        self.pages.update(|pages| *pages += 1);
    }

    pub(super) fn retry(self) {
        self.revision.update(|revision| *revision += 1);
    }

    fn apply(self, results: Results, scope: Section, background: bool) {
        batch(|| {
            if matches!(scope, Section::All | Section::Users) {
                self.users_empty.set(results.users_empty);
            }
            self.more.set(results.more);
            self.rows.set(results.rows);
        });
        if !background {
            after_answer(move || self.list.scroll_to_offset(Points::ZERO, false));
        }
    }
}

struct Results {
    rows: Vec<Row>,
    more: bool,
    users_empty: bool,
}

impl Results {
    fn new(
        mut users: Vec<UserProfileItem>,
        mut conversations: Vec<ConversationListData>,
        mut messages: Vec<MessageListView>,
        limit: usize,
    ) -> Self {
        let mut results = Self {
            rows: Vec::new(),
            more: false,
            users_empty: users.is_empty(),
        };
        if !users.is_empty() {
            results.header(Section::Users, users.len() > limit);
            users.truncate(limit);
            results.rows.extend(users.into_iter().map(Row::User));
        }
        if !conversations.is_empty() {
            results.header(Section::Conversations, conversations.len() > limit);
            conversations.truncate(limit);
            results.rows.extend(
                conversations
                    .into_iter()
                    .map(|row| Row::Conversation(Box::new(row))),
            );
        }
        if !messages.is_empty() {
            results.header(Section::Messages, messages.len() > limit);
            messages.truncate(limit);
            results
                .rows
                .extend(messages.into_iter().map(|row| Row::Message(Box::new(row))));
        }
        results
    }

    fn header(&mut self, section: Section, more: bool) {
        self.more |= more;
        self.rows.push(Row::Header(section, more));
    }
}

pub(super) fn new(
    account: Stored<Arc<AccountClient>>,
    query: Signal<String>,
    composing: Signal<bool>,
    category: Signal<&'static str>,
    circle: Signal<Option<String>>,
    unread: Signal<bool>,
) -> State {
    let state = State {
        section: signal(Section::All),
        pages: signal(1_i64),
        more: signal(false),
        extending: signal(false),
        rows: signal_with(Vec::<Row>::new(), SignalEquals::Never),
        users_empty: signal(true),
        loading: signal(true),
        failed: signal(false),
        revision: signal(0_u64),
        list: element_ref(),
    };
    effect(move || {
        let _query = query.get();
        let _category = category.get();
        let _circle = circle.get();
        let _unread = unread.get();
        state.show(Section::All);
    });
    let event_account = account.with(|account| account.clone());
    spawn(async move {
        use futures::StreamExt as _;
        let mut changes = Box::pin(runtime::throttled(
            event_account.conversation_changes(),
            REFRESH_INTERVAL,
        ));
        while changes.next().await.is_some() {
            state.revision.update(|revision| *revision += 1);
        }
    });
    let seen_revision = stored(0_u64);
    let seen_pages = stored(1_i64);
    effect(move || {
        if composing.get() {
            return;
        }
        let keyword = query.get().trim().to_owned();
        let category = category.get();
        let circle = circle.get();
        let unread_only = unread.get();
        let scope = state.section.get();
        let pages = state.pages.get();
        let revision = state.revision.get();
        let grew = seen_pages.replace(pages) < pages;
        // New data or a further page under an unchanged query updates in place:
        // no spinner, no debounce, and the scroll position stays.
        let background = (seen_revision.replace(revision) != revision || grew)
            && !state.failed.get_untracked()
            && !state.loading.get_untracked();
        if !background {
            state.loading.set(true);
            state.failed.set(false);
        }
        let client = account.with(|account| account.clone());
        // Rerunning this effect cancels the previous task and its business call.
        spawn(async move {
            if !background {
                timer(DEBOUNCE).await;
            }
            let count = if scope == Section::All { 3 } else { 60 * pages };
            let result = runtime::run(async move {
                let access = client.conversation();
                let user = client.user();
                let message = client.message();
                let users = async {
                    if !unread_only && matches!(scope, Section::All | Section::Users) {
                        user.search_local_users_page(
                            keyword.clone(),
                            category.into(),
                            circle.clone(),
                            0,
                            count + 1,
                        )
                        .await
                    } else {
                        Ok(Vec::new())
                    }
                };
                let conversations = async {
                    if matches!(scope, Section::All | Section::Conversations) {
                        access
                            .conversations(
                                category.into(),
                                circle.clone(),
                                keyword.clone(),
                                unread_only,
                                count + 1,
                                0,
                            )
                            .await
                    } else {
                        Ok(Vec::new())
                    }
                };
                let messages = async {
                    if matches!(scope, Section::All | Section::Messages) {
                        message
                            .search_scoped_messages(
                                keyword.clone(),
                                category.into(),
                                circle.clone(),
                                unread_only,
                                None,
                                (count + 1) as u32,
                            )
                            .await
                    } else {
                        Ok(Vec::new())
                    }
                };
                Ok(futures::join!(users, conversations, messages))
            })
            .await;
            match result {
                Ok((users, conversations, messages)) => {
                    state
                        .failed
                        .set(users.is_err() || conversations.is_err() || messages.is_err());
                    let results = Results::new(
                        users.unwrap_or_default(),
                        conversations.unwrap_or_default(),
                        messages.unwrap_or_default(),
                        count as usize,
                    );
                    state.apply(results, scope, background);
                }
                Err(_) => state.failed.set(true),
            }
            state.extending.set(false);
            state.loading.set(false);
        });
    });
    state
}
