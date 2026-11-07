//! Bounded, sectioned global search. Business filtering and paging stay in Rust.
mod palette;
mod query;
mod remote;

pub(crate) use palette::surface as palette;
mod results;

use crate::{
    i18n::{self, Key},
    navigation::ChatNavigation,
    runtime,
    sidebar::Navigation,
    ui,
};
use mixin_desktop_api::{AccountClient, ConversationListData, MessageListView, UserProfileItem};
use nsh_base::theme;
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
}

fn tr(en: &'static str, zh: &'static str) -> &'static str {
    crate::i18n::tr(en, zh)
}

#[derive(Clone, PartialEq)]
enum Pending {
    User(String),
    Message(String),
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    query: Signal<String>,
    composing: Signal<bool>,
    category: Signal<&'static str>,
    selected: Signal<Option<ConversationListData>>,
    _locate: Signal<Option<String>>,
    unread: Signal<bool>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let nav = use_context::<Navigation>().expect("Home provides category navigation");
    let navigation = use_context::<ChatNavigation>().expect("Home provides chat navigation");
    let owner = Owner::current();
    let account = stored(account);
    let query::State {
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
    } = query::new(account, query, composing, category, nav.circle, unread);
    let pending = signal(None::<Pending>);
    let open_failed = signal(false);
    let profile = signal(None::<String>);
    effect(move || {
        let _query = query.get();
        let _category = category.get();
        let _circle = nav.circle.get();
        let _unread = unread.get();
        pending.set(None);
    });
    effect(move || {
        if open_failed.get() {
            pending.set(None);
        }
    });
    effect(move || {
        let current = selected.get();
        // Observe completed navigation, including a deleted message's latest-page fallback.
        // Starting a request alone must not dismiss the search.
        untracked(|| {
            let opened = match (pending.get_untracked(), current) {
                (Some(Pending::User(user)), Some(row)) => row.owner_id == user,
                (Some(Pending::Message(conversation)), Some(row)) => {
                    row.conversation_id == conversation
                }
                _ => false,
            };
            if opened {
                pending.set(None);
                query.set(String::new());
            }
        });
    });
    let open = move |row: Row| {
        open_failed.set(false);
        match row {
            Row::Header(next, _) => batch(|| {
                section.set(next);
                page.set(0);
                cursors.set(vec![None]);
                rows.set(Vec::new());
            }),
            Row::Conversation(row) => {
                navigation.select(*row);
                query.set(String::new());
            }
            Row::User(user) => {
                if let Some(row) = selected.with_untracked(|row| {
                    row.as_ref()
                        .filter(|row| row.owner_id == user.user_id)
                        .cloned()
                }) {
                    navigation.select(row);
                    query.set(String::new());
                } else {
                    pending.set(Some(Pending::User(user.user_id.clone())));
                    owner.run(|| {
                        navigation.open_user(
                            account.with(|account| account.clone()),
                            user.user_id,
                            open_failed,
                        )
                    });
                }
            }
            Row::Message(message) => {
                pending.set(Some(Pending::Message(message.conversation_id.clone())));
                owner.run(|| {
                    navigation.open_search_message(
                        account.with(|account| account.clone()),
                        message.conversation_id,
                        message.message_id,
                        query.get_untracked().trim().to_owned(),
                        open_failed,
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
                    ui::command()
                        .px_2()
                        .min_h_8()
                        .flex_1()
                        .selected(move || section.get() == value)
                        .selected_style(|style| style.bg(move || theme().accent))
                        .on_press(move || {
                            batch(|| {
                                section.set(value);
                                page.set(0);
                                cursors.set(vec![None]);
                                rows.set(Vec::new());
                            })
                        })
                        .child(text(move || value.title()).text_xs())
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
            text(move || {
                tr(
                    "Couldn’t open this result. Try again.",
                    "无法打开此结果，请重试。",
                )
            })
            .px_4()
            .py_2()
            .text_sm()
            .text_color(move || theme().destructive)
        }))
        .child(when(loading, move || {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .child(ui::busy_indicator(
                    move || loading.get(),
                    move || i18n.text(Key::LoadingChats).into(),
                ))
                .child(text(move || i18n.text(Key::LoadingChats)).text_sm())
        }))
        .child(when(failed, move || {
            view()
                .px_4()
                .py_2()
                .gap_2()
                .child(
                    text(move || {
                        tr(
                            "Some results could not be loaded. Try again.",
                            "部分搜索结果加载失败，请重试。",
                        )
                    })
                    .text_sm()
                    .text_color(move || theme().destructive),
                )
                .child(ui::action(
                    move || i18n.text(Key::Retry),
                    move || revision.update(|revision| *revision += 1),
                ))
        }))
        .child(when(
            move || !loading.get() && !failed.get() && rows.with(Vec::is_empty),
            move || {
                ui::centered(move || match section.get() {
                    Section::All => tr("No results", "未找到相关结果"),
                    Section::Users => i18n.text(Key::NoContacts),
                    Section::Conversations => i18n.text(Key::NoMatches),
                    Section::Messages => tr("No messages found", "未找到消息"),
                })
            },
        ))
        .child(when(
            move || !rows.with(Vec::is_empty),
            move || results::list(rows, section, pending, loading, open),
        ))
        .child(when(
            move || section.get() != Section::All && (page.get() > 0 || more.get()),
            move || {
                view()
                    .flex_row()
                    .px_3()
                    .py_2()
                    .justify_between()
                    .gap_2()
                    .child(
                        ui::action(
                            move || tr("Previous page", "上一页"),
                            move || page.update(|page| *page = page.saturating_sub(1)),
                        )
                        .disabled(move || loading.get() || page.get() == 0),
                    )
                    .child(text(move || (page.get() + 1).to_string()).text_sm())
                    .child(
                        ui::action(
                            move || tr("Next page", "下一页"),
                            move || {
                                if section.get_untracked() == Section::Messages {
                                    let cursor = next_cursor.get_untracked();
                                    let next = page.get_untracked() as usize + 1;
                                    cursors.update(|cursors| {
                                        cursors.truncate(next);
                                        cursors.push(cursor);
                                    });
                                }
                                page.update(|page| *page += 1);
                            },
                        )
                        .disabled(move || loading.get() || !more.get()),
                    )
            },
        ))
        .child(when(
            move || profile.with(Option::is_some),
            move || {
                let id = profile.get_untracked().unwrap_or_default();
                nsh_base::dialog()
                    .on_cancel(move || profile.set(None))
                    .flex_1()
                    .p_6()
                    .items_center()
                    .justify_center()
                    .bg(move || theme().foreground.opacity(0.2))
                    .child(
                        view()
                            .w_96()
                            .max_w(relative(1.0))
                            .h(px(640.0))
                            .max_h(relative(0.9))
                            .rounded_xl()
                            .bg(move || theme().popover)
                            .child(crate::conversation::profile(
                                account.with(|account| account.clone()),
                                id,
                                move || profile.set(None),
                            )),
                    )
            },
        ))
}
