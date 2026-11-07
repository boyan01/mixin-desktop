use mixin_desktop_api::{AccountClient, ConversationListData, UserProfileItem};
use std::sync::Arc;

use crate::{conversation::parts, i18n::tr, runtime, ui};
use nsh_core::prelude::*;

const PAGE_SIZE: i64 = 60;

#[derive(Clone)]
pub(crate) enum Recipient {
    Chat(Box<ConversationListData>),
    User(UserProfileItem),
}
impl Recipient {
    /// A person has one key, whether picked from chats or from contacts.
    pub(crate) fn key(&self) -> String {
        match self {
            Self::Chat(row) if row.category == "GROUP" => format!("chat:{}", row.conversation_id),
            Self::Chat(row) => format!("user:{}", row.owner_id),
            Self::User(user) => format!("user:{}", user.user_id),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Chat(row) => row.name.clone(),
            Self::User(user) => user.full_name.clone(),
        }
    }
    fn avatar(&self) -> String {
        match self {
            Self::Chat(row) => row.avatar_url.clone(),
            Self::User(user) => user.avatar_url.clone(),
        }
    }
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    disabled: impl Fn() -> bool + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
    choose: impl Fn(Recipient) + Copy + 'static,
) -> impl IntoElement {
    picker(account, disabled, |_| false, cancel, choose)
}

/// `selected` marks rows of a multi-selection; `choose` toggles them.
pub(crate) fn picker(
    account: Arc<AccountClient>,
    disabled: impl Fn() -> bool + Copy + 'static,
    selected: impl Fn(&Recipient) -> bool + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
    choose: impl Fn(Recipient) + Copy + 'static,
) -> impl IntoElement {
    let account = stored(account);
    let contacts = signal(false);
    let query = signal(String::new());
    let composing = signal(false);
    let page = signal(0_i64);
    let rows = signal_with(Vec::<Recipient>::new(), SignalEquals::Never);
    let more = signal(false);
    let loading = signal(true);
    let failed = signal(false);
    let retry = signal(0_u64);
    effect(move || {
        if composing.get() {
            loading.set(true);
            return;
        }
        let query = query.get();
        let page = page.get();
        let contacts = contacts.get();
        let _retry = retry.get();
        let client = account.with(|account| account.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            if !query.trim().is_empty() {
                timer(parts::SEARCH_DELAY).await;
            }
            let result = runtime::run(async move {
                if contacts {
                    Ok(client
                        .user()
                        .selectable_users_page(query, page * PAGE_SIZE, PAGE_SIZE + 1)
                        .await?
                        .into_iter()
                        .map(Recipient::User)
                        .collect::<Vec<_>>())
                } else {
                    Ok(client
                        .conversation()
                        .conversations(
                            "chats".into(),
                            None,
                            query,
                            false,
                            PAGE_SIZE + 1,
                            page * PAGE_SIZE,
                        )
                        .await?
                        .into_iter()
                        .map(|row| Recipient::Chat(Box::new(row)))
                        .collect())
                }
            })
            .await;
            match result {
                Ok(mut values) => {
                    more.set(values.len() > PAGE_SIZE as usize);
                    values.truncate(PAGE_SIZE as usize);
                    rows.set(values);
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    let unavailable = move || disabled() || composing.get();
    view()
        .flex_shrink(1.0)
        .min_h_0()
        .gap_3()
        .child(
            view().flex_none().flex_row().gap_2().children(
                [
                    (false, "Recent chats", "最近聊天"),
                    (true, "Contacts", "联系人"),
                ]
                .into_iter()
                .map(move |(value, en, zh)| {
                    ui::command()
                        .flex_1()
                        .selected(move || contacts.get() == value)
                        .disabled(unavailable)
                        .on_press(move || {
                            if contacts.get_untracked() == value {
                                return;
                            }
                            batch(|| {
                                contacts.set(value);
                                page.set(0);
                                rows.set(Vec::new());
                            })
                        })
                        .child(text(move || tr(en, zh)))
                }),
            ),
        )
        .child(
            ui::field()
                .flex_none()
                .label(move || tr("Search", "搜索"))
                .placeholder(move || tr("Search", "搜索").to_owned())
                .value(move || query.get())
                .disabled(disabled)
                .auto_focus(true)
                .on_key_press(move |key: &str| {
                    if key == "Escape" && !disabled() && !composing.get_untracked() {
                        cancel();
                    }
                })
                .on_change(move |value: &str| {
                    if query.with_untracked(|query| query == value) {
                        return;
                    }
                    batch(|| {
                        query.set(value.into());
                        page.set(0);
                    })
                })
                .on_composition_change(move |event: &Composition| composing.set(event.composing)),
        )
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            || parts::status(move || tr("Loading…", "加载中…")),
        ))
        .child(when(failed, move || {
            parts::failure(
                move || tr("Couldn’t load chats. Try again.", "聊天加载失败，请重试。"),
                move || retry.update(|retry| *retry += 1),
            )
        }))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    let value = move || rows.with(|rows| rows.get(at.index()).cloned());
                    parts::entity_row(
                        move || value().map(|value| value.name()).unwrap_or_default(),
                        move || value().map(|value| value.avatar()).unwrap_or_default(),
                        String::new,
                        move || value().is_some_and(|value| selected(&value)),
                    )
                    .px_3()
                    .disabled(unavailable)
                    .on_press(move || {
                        if let Some(value) = value() {
                            choose(value);
                        }
                    })
                },
            )
            .keys(move || rows.with(|rows| rows.iter().map(Recipient::key).collect::<Vec<_>>()))
            .estimated_item_size(parts::row_height())
            .h(move || parts::dialog_list_height(rows.with(Vec::len)))
            .flex_shrink(1.0)
            .min_h_0()
            .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || !loading.get() && !failed.get() && rows.with(Vec::is_empty),
            || ui::placeholder(move || tr("No chats found", "没有找到聊天")).min_h_24(),
        ))
        .child(parts::pager(
            page,
            move || more.get(),
            move || unavailable() || loading.get(),
        ))
}
