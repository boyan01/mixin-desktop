//! Bounded, sectioned global search. Business filtering and paging stay in Rust.
mod palette;
mod query;
mod remote;

pub(crate) use palette::surface as palette;
mod results;

use crate::{
    conversation::parts,
    i18n::{self, Key, tr},
    navigation::ChatNavigation,
    runtime,
    sidebar::Navigation,
    ui::{self, Typography as _},
};
use mixin_desktop_api::{AccountClient, ConversationListData, MessageListView, UserProfileItem};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    All,
    Users,
    Conversations,
    Messages,
}
impl Section {
    fn title(self) -> &'static str {
        match self {
            Self::All => tr("All", "全部"),
            Self::Users => tr("Contacts", "联系人"),
            Self::Conversations => tr("Conversations", "会话"),
            Self::Messages => tr("Messages", "消息"),
        }
    }
}

#[derive(Clone)]
enum Row {
    Header(Section, bool),
    User(UserProfileItem),
    Conversation(Box<ConversationListData>),
    Message(Box<MessageListView>),
}
impl Row {
    fn key(&self) -> String {
        match self {
            Self::Header(section, _) => format!("section:{}", *section as u8),
            Self::User(user) => format!("user:{}", user.user_id),
            Self::Conversation(row) => format!("chat:{}", row.conversation_id),
            Self::Message(message) => format!("message:{}", message.message_id),
        }
    }

    fn avatar_id(&self) -> String {
        match self {
            Self::Header(..) => String::new(),
            Self::User(user) => user.user_id.clone(),
            Self::Conversation(row) => row.owner_id.clone(),
            Self::Message(message) => message.sender_id.clone(),
        }
    }

    fn title(&self) -> String {
        match self {
            Self::Header(..) => String::new(),
            Self::User(user) => user.full_name.clone(),
            Self::Conversation(row) => row.name.clone(),
            Self::Message(message) => message.sender_name.clone(),
        }
    }

    fn avatar_url(&self) -> String {
        match self {
            Self::Header(..) => String::new(),
            Self::User(user) => user.avatar_url.clone(),
            Self::Conversation(row) => row.avatar_url.clone(),
            Self::Message(message) => message.sender_avatar_url.clone(),
        }
    }
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    query: Signal<String>,
    composing: Signal<bool>,
    selected: Signal<Option<ConversationListData>>,
    unread: Signal<bool>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let nav = use_context::<Navigation>().expect("Home provides category navigation");
    let navigation = use_context::<ChatNavigation>().expect("Home provides chat navigation");
    let owner = Owner::current();
    let account = stored(account);
    let state = query::new(account, query, composing, nav.category, nav.circle, unread);
    let query::State {
        section,
        page,
        more,
        rows,
        users_empty,
        loading,
        failed,
        ..
    } = state;
    // An open is in flight; a second press must not race it.
    let pending = signal(false);
    let open_failed = signal(false);
    let profile = signal(None::<String>);
    let profile_presence = crate::motion::Presence::modal(move || profile.with(Option::is_some));
    effect(move || {
        query.with(|_| ());
        pending.set(false);
    });
    effect(move || {
        if open_failed.get() {
            pending.set(false);
        }
    });
    let open = move |row: Row| {
        open_failed.set(false);
        let keyword = query.get_untracked();
        // Search closes on arrival, unless the reader has typed something else.
        let opened = move || {
            pending.set(false);
            if query.with_untracked(|query| *query == keyword) {
                query.set(String::new());
            }
        };
        match row {
            Row::Header(next, _) => state.show(next),
            Row::Conversation(row) => {
                navigation.select(*row);
                query.set(String::new());
            }
            Row::User(user) => {
                let open_chat = selected.with_untracked(|row| {
                    row.as_ref()
                        .filter(|row| row.category != "GROUP" && row.owner_id == user.user_id)
                        .cloned()
                });
                if let Some(row) = open_chat {
                    navigation.select(row);
                    query.set(String::new());
                } else {
                    pending.set(true);
                    owner.run(|| {
                        navigation.open_user_then(
                            account.with(|account| account.clone()),
                            user.user_id,
                            open_failed,
                            opened,
                        )
                    });
                }
            }
            Row::Message(message) => {
                pending.set(true);
                owner.run(|| {
                    navigation.open_search_message(
                        account.with(|account| account.clone()),
                        message.conversation_id,
                        message.message_id,
                        query.get_untracked().trim().to_owned(),
                        open_failed,
                        opened,
                    )
                });
            }
        }
    };
    view()
        .flex_1()
        .min_h_0()
        .child(
            view().px_3().pb_2().flex_row().gap_1().children(
                [
                    Section::All,
                    Section::Users,
                    Section::Conversations,
                    Section::Messages,
                ]
                .into_iter()
                .map(move |value| {
                    ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                        .flex_1()
                        .selected(move || section.get() == value)
                        .on_press(move || state.show(value))
                        .child(text(move || value.title()).text_role(ui::TextRole::Caption))
                }),
            ),
        )
        .child(remote::banner(
            account.with(|account| account.clone()),
            query,
            composing,
            users_empty,
            profile,
        ))
        .child(when(open_failed, move || {
            ui::error_text(move || {
                tr(
                    "Couldn’t open this result. Try again.",
                    "无法打开此结果，请重试。",
                )
            })
            .px_4()
            .py_2()
        }))
        // Results stay in place while they refresh, so only a first load shows progress.
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            move || {
                parts::status(move || i18n.text(Key::LoadingChats))
                    .px_4()
                    .py_2()
            },
        ))
        .child(when(failed, move || {
            view()
                .px_4()
                .py_2()
                .gap_2()
                .child(ui::error_text(move || {
                    tr(
                        "Some results could not be loaded. Try again.",
                        "部分搜索结果加载失败，请重试。",
                    )
                }))
                .child(ui::action(
                    move || i18n.text(Key::Retry),
                    move || state.retry(),
                ))
        }))
        .child(when(
            move || !loading.get() && !failed.get() && rows.with(Vec::is_empty),
            move || {
                ui::placeholder(move || match section.get() {
                    Section::All => tr("No results", "未找到相关结果"),
                    Section::Users => i18n.text(Key::NoContacts),
                    Section::Conversations => i18n.text(Key::NoMatches),
                    Section::Messages => tr("No messages found", "未找到消息"),
                })
            },
        ))
        .child(when(
            move || !rows.with(Vec::is_empty),
            move || results::list(state, pending, open),
        ))
        .child(when(
            move || section.get() != Section::All && (page.get() > 0 || more.get()),
            move || {
                view()
                    .flex_row()
                    .px_3()
                    .py_2()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        ui::action(
                            move || tr("Previous page", "上一页"),
                            move || state.previous_page(),
                        )
                        .disabled(move || loading.get() || page.get() == 0),
                    )
                    .child(text(move || (page.get() + 1).to_string()).text_role(ui::TextRole::Body))
                    .child(
                        ui::action(move || tr("Next page", "下一页"), move || state.next_page())
                            .disabled(move || loading.get() || !more.get()),
                    )
            },
        ))
        .child(when(
            move || profile_presence.mounted(),
            move || {
                let id = profile.get_untracked().unwrap_or_default();
                ui::dialog(
                    profile_presence,
                    move || profile.set(None),
                    ui::dialog_card(ui::DialogWidth::Compact)
                        .p_0()
                        .gap_0()
                        .overflow_hidden()
                        .child(crate::conversation::profile(
                            account.with(|account| account.clone()),
                            id,
                            move || profile.set(None),
                        )),
                )
            },
        ))
}
