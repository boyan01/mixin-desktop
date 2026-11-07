use super::*;

pub(super) struct State {
    pub(super) section: Signal<Section>,
    pub(super) page: Signal<i64>,
    pub(super) cursors: Signal<Vec<Option<String>>>,
    pub(super) next_cursor: Signal<Option<String>>,
    pub(super) more: Signal<bool>,
    pub(super) rows: Signal<Vec<Row>>,
    pub(super) users_empty: Signal<bool>,
    pub(super) loading: Signal<bool>,
    pub(super) failed: Signal<bool>,
    pub(super) revision: Signal<u64>,
}

pub(super) fn new(
    account: Stored<Arc<AccountClient>>,
    query: Signal<String>,
    composing: Signal<bool>,
    category: Signal<&'static str>,
    circle: Signal<Option<String>>,
    unread: Signal<bool>,
) -> State {
    let section = signal(Section::All);
    let page = signal(0_i64);
    let cursors = signal(vec![None::<String>]);
    let next_cursor = signal(None::<String>);
    let more = signal(false);
    let rows = signal_with(Vec::<Row>::new(), SignalEquals::Never);
    let users_empty = signal(true);
    let loading = signal(true);
    let failed = signal(false);
    let revision = signal(0_u64);
    let request = signal(0_u64);
    effect(move || {
        let _query = query.get();
        let _category = category.get();
        let _circle = circle.get();
        let _unread = unread.get();
        batch(|| {
            section.set(Section::All);
            page.set(0);
            cursors.set(vec![None]);
            rows.set(Vec::new());
        });
    });
    let event_account = account.with(|account| account.clone());
    spawn(async move {
        let mut changes = Box::pin(runtime::observe(event_account.conversation_changes()));
        use futures::StreamExt as _;
        while changes.next().await.is_some() {
            revision.update(|revision| *revision += 1);
        }
    });
    effect(move || {
        if composing.get() {
            return;
        }
        let keyword = query.get().trim().to_owned();
        let category = category.get();
        let circle = circle.get();
        let unread_only = unread.get();
        let scope = section.get();
        let offset = page.get();
        let cursor = cursors.with(|cursors| cursors.get(offset as usize).cloned().flatten());
        let _revision = revision.get();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        let client = account.with(|account| account.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            timer(std::time::Duration::from_millis(150)).await;
            let count = if scope == Section::All { 3 } else { 60 };
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
                            offset * count,
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
                                offset * count,
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
                                cursor,
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
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok((users, conversations, messages)) => {
                    let mut values = Vec::new();
                    let mut has_more = false;
                    failed.set(users.is_err() || conversations.is_err() || messages.is_err());
                    let mut users = users.unwrap_or_default();
                    users_empty.set(users.is_empty());
                    let mut conversations = conversations.unwrap_or_default();
                    let mut messages = messages.unwrap_or_default();
                    let limit = count as usize;
                    if !users.is_empty() {
                        let extra = users.len() > limit;
                        has_more |= extra;
                        values.push(Row::Header(Section::Users, extra));
                        users.truncate(limit);
                        values.extend(users.into_iter().map(Row::User));
                    }
                    if !conversations.is_empty() {
                        let extra = conversations.len() > limit;
                        has_more |= extra;
                        values.push(Row::Header(Section::Conversations, extra));
                        conversations.truncate(limit);
                        values.extend(
                            conversations
                                .into_iter()
                                .map(|row| Row::Conversation(Box::new(row))),
                        );
                    }
                    if !messages.is_empty() {
                        let extra = messages.len() > limit;
                        has_more |= extra;
                        values.push(Row::Header(Section::Messages, extra));
                        messages.truncate(limit);
                        next_cursor.set(messages.last().map(|message| message.message_id.clone()));
                        values.extend(messages.into_iter().map(|row| Row::Message(Box::new(row))));
                    } else {
                        next_cursor.set(None);
                    }
                    rows.set(values);
                    more.set(has_more);
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    State {
        section,
        page,
        cursors,
        next_cursor,
        more,
        rows,
        users_empty,
        loading,
        failed,
        revision,
    }
}
