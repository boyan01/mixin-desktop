use mixin_desktop_api::{AccountClient, ConversationListData, UserProfileItem};
use std::sync::Arc;

use crate::{i18n::tr, runtime, ui};
use nsh_base::theme;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) enum Recipient {
    Chat(Box<ConversationListData>),
    User(UserProfileItem),
}
impl Recipient {
    fn key(&self) -> String {
        match self {
            Self::Chat(row) => format!("chat:{}", row.conversation_id),
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
    let account = stored(account);
    let contacts = signal(false);
    let query = signal(String::new());
    let composing = signal(false);
    let page = signal(0_i64);
    let rows = signal_with(Vec::<Recipient>::new(), SignalEquals::Never);
    let more = signal(false);
    let loading = signal(true);
    let failed = signal(false);
    let request = signal(0_u64);
    let retry = signal(0_u64);
    effect(move || {
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
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
            let result = runtime::run(async move {
                if contacts {
                    Ok(client
                        .user()
                        .selectable_users_page(query, page * 60, 61)
                        .await?
                        .into_iter()
                        .map(Recipient::User)
                        .collect::<Vec<_>>())
                } else {
                    Ok(client
                        .conversation()
                        .conversations("chats".into(), None, query, false, 61, page * 60)
                        .await?
                        .into_iter()
                        .map(|row| Recipient::Chat(Box::new(row)))
                        .collect())
                }
            })
            .await;
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok(mut values) => {
                    more.set(values.len() > 60);
                    values.truncate(60);
                    rows.set(values);
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    view()
        .flex_shrink(1.0)
        .min_h_0()
        .gap_3()
        .child(
            view().flex_none().flex_row().gap_2().children(
                [
                    (false, "Conversations", "会话"),
                    (true, "Contacts", "联系人"),
                ]
                .into_iter()
                .map(move |(value, en, zh)| {
                    ui::command()
                        .flex_1()
                        .selected(move || contacts.get() == value)
                        .selected_style(|style| style.bg(move || theme().accent))
                        .disabled(move || disabled() || composing.get())
                        .on_press(move || {
                            if disabled()
                                || composing.get_untracked()
                                || contacts.get_untracked() == value
                            {
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
                .label(move || tr("Choose recipient", "选择接收方"))
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
                        rows.set(Vec::new());
                    })
                })
                .on_composition_change(move |event: &Composition| composing.set(event.composing)),
        )
        .child(when(loading, || {
            text(move || tr("Loading…", "加载中…")).text_sm()
        }))
        .child(when(failed, move || {
            view()
                .gap_2()
                .child(
                    text(move || tr("Couldn’t load recipients.", "无法加载接收方。"))
                        .text_sm()
                        .text_color(move || theme().destructive),
                )
                .child(
                    ui::action(
                        move || tr("Retry", "重试"),
                        move || {
                            if !disabled() && !composing.get_untracked() {
                                retry.update(|retry| *retry += 1);
                            }
                        },
                    )
                    .disabled(move || disabled() || composing.get()),
                )
        }))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    let value = move || rows.with(|rows| rows.get(at.index()).cloned());
                    ui::command()
                        .justify_start()
                        .min_h_16()
                        .gap_3()
                        .disabled(move || {
                            loading.get() || failed.get() || disabled() || composing.get()
                        })
                        .child(ui::avatar_view(
                            move || value().map(|value| value.name().into()).unwrap_or_default(),
                            move || value().map(|value| value.avatar()).unwrap_or_default(),
                        ))
                        .child(
                            text(move || value().map(|value| value.name()).unwrap_or_default())
                                .text_sm()
                                .number_of_lines(1)
                                .flex_1(),
                        )
                        .on_press(move || {
                            if loading.get_untracked()
                                || failed.get_untracked()
                                || disabled()
                                || composing.get_untracked()
                            {
                                return;
                            }
                            if let Some(value) = value() {
                                choose(value);
                            }
                        })
                },
            )
            .keys(move || rows.with(|rows| rows.iter().map(Recipient::key).collect::<Vec<_>>()))
            .estimated_item_size(px(64.0))
            .h(move || px(rows.with(|rows| rows.len().min(5)) as f32 * 64.0))
            .flex_shrink(1.0)
            .min_h_0()
            .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || !loading.get() && !failed.get() && rows.with(Vec::is_empty),
            || {
                view().min_h_24().items_center().justify_center().child(
                    text(move || tr("No recipients found", "没有找到接收方"))
                        .text_sm()
                        .text_color(move || theme().muted_foreground),
                )
            },
        ))
        .child(when(
            move || page.get() > 0 || more.get(),
            move || {
                view()
                    .flex_none()
                    .flex_row()
                    .justify_between()
                    .gap_2()
                    .child(
                        ui::action(
                            move || tr("Previous page", "上一页"),
                            move || {
                                if !disabled()
                                    && !composing.get_untracked()
                                    && !loading.get_untracked()
                                    && page.get_untracked() > 0
                                {
                                    page.update(|page| *page = page.saturating_sub(1));
                                }
                            },
                        )
                        .disabled(move || {
                            disabled() || composing.get() || loading.get() || page.get() == 0
                        }),
                    )
                    .child(
                        ui::action(
                            move || tr("Next page", "下一页"),
                            move || {
                                if !disabled()
                                    && !composing.get_untracked()
                                    && !loading.get_untracked()
                                    && more.get_untracked()
                                {
                                    page.update(|page| *page += 1);
                                }
                            },
                        )
                        .disabled(move || {
                            disabled() || composing.get() || loading.get() || !more.get()
                        }),
                    )
            },
        ))
}
