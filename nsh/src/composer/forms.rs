use mixin_desktop_api::{AccountClient, MessageListView, UserProfileItem};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

use super::tr;
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

pub(super) fn contact(
    account: Arc<AccountClient>,
    conversation: String,
    quote: Signal<Option<MessageListView>>,
    close: impl Fn() + Copy + 'static,
    failed: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let account = stored(account);
    let conversation = stored(conversation);
    let query = signal(String::new());
    let offset = signal(0_i64);
    let rows = signal_with(Vec::<UserProfileItem>::new(), SignalEquals::Never);
    let more = signal(false);
    let loading = signal(true);
    let local_failed = signal(false);
    let retry = signal(0_u64);
    let busy = signal(false);
    let composing = signal(false);
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            close();
        }
    };
    effect(move || {
        let query = query.get();
        let offset = offset.get();
        retry.get();
        let client = account.with(|a| a.clone());
        loading.set(true);
        rows.set(Vec::new());
        local_failed.set(false);
        spawn(async move {
            match runtime::run(async move {
                client
                    .user()
                    .selectable_users_page(query, offset, 101)
                    .await
            })
            .await
            {
                Ok(mut values) => {
                    more.set(values.len() > 100);
                    values.truncate(100);
                    rows.set(values);
                }
                Err(_) => local_failed.set(true),
            }
            loading.set(false);
        });
    });
    ui::modal(
        presence,
        cancel,
        view()
            .w_96()
            .max_w(relative(1.0))
            .max_h(relative(0.9))
            .min_h_0()
            .overflow_hidden()
            .p_6()
            .gap_3()
            .rounded_xl()
            .bg(move || theme().popover)
            .child(
                text(move || tr(i18n, "Share contact", "分享联系人"))
                    .flex_none()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                ui::field()
                    .flex_none()
                    .label(move || i18n.text(Key::SearchContact))
                    .placeholder(move || i18n.text(Key::SearchContact).to_owned())
                    .auto_focus(true)
                    .value(move || query.get())
                    .readonly(busy)
                    .on_key_press(move |key: &str| {
                        if key == "Escape" {
                            cancel();
                        }
                    })
                    .on_composition_change(move |event: &Composition| {
                        composing.set(event.composing)
                    })
                    .on_change(move |value: &str| {
                        batch(|| {
                            query.set(value.to_owned());
                            offset.set(0);
                        })
                    }),
            )
            .child(
                view()
                    .flex_shrink(1.0)
                    .min_h_0()
                    .gap_3()
                    .child(when(loading, move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(ui::busy_indicator(
                                move || loading.get(),
                                move || i18n.text(Key::LoadingChats).into(),
                            ))
                            .child(text(move || i18n.text(Key::LoadingChats)).text_sm())
                    }))
                    .child(when(local_failed, move || {
                        view()
                            .gap_2()
                            .child(
                                text(move || i18n.text(Key::CommandFailed))
                                    .text_sm()
                                    .text_color(move || theme().destructive),
                            )
                            .child(ui::action(
                                move || i18n.text(Key::Retry),
                                move || retry.update(|v| *v += 1),
                            ))
                    }))
                    .child(when(
                        move || !loading.get() && !local_failed.get() && rows.with(Vec::is_empty),
                        move || {
                            view().min_h_24().items_center().justify_center().child(
                                text(move || i18n.text(Key::NoContacts))
                                    .text_sm()
                                    .text_color(move || theme().muted_foreground),
                            )
                        },
                    ))
                    .child(
                        virtual_list(
                            move || rows.with(Vec::len),
                            move |at| {
                                ui::command()
                                    .justify_start()
                                    .gap_3()
                                    .disabled(busy)
                                    .child(ui::avatar_view(
                                        move || {
                                            rows.with(|rows| {
                                                rows.get(at.index())
                                                    .map(|u| u.full_name.clone().into())
                                                    .unwrap_or_default()
                                            })
                                        },
                                        move || {
                                            rows.with(|rows| {
                                                rows.get(at.index())
                                                    .map(|u| u.avatar_url.clone())
                                                    .unwrap_or_default()
                                            })
                                        },
                                    ))
                                    .child(
                                        view()
                                            .flex_1()
                                            .min_w_0()
                                            .child(
                                                text(move || {
                                                    rows.with(|rows| {
                                                        rows.get(at.index())
                                                            .map(|u| u.full_name.clone())
                                                            .unwrap_or_default()
                                                    })
                                                })
                                                .text_sm()
                                                .number_of_lines(1),
                                            )
                                            .child(
                                                text(move || {
                                                    rows.with(|rows| {
                                                        rows.get(at.index())
                                                            .map(|u| u.identity_number.clone())
                                                            .unwrap_or_default()
                                                    })
                                                })
                                                .text_xs()
                                                .text_color(move || theme().muted_foreground),
                                            ),
                                    )
                                    .on_press(move || {
                                        if !presence.interactive() {
                                            return;
                                        }
                                        let Some(user) =
                                            rows.with(|rows| rows.get(at.index()).cloned())
                                        else {
                                            return;
                                        };
                                        let client = account.with(|a| a.clone());
                                        let id = conversation.with(|id| id.clone());
                                        let sent_quote = quote
                                            .with(|q| q.as_ref().map(|m| m.message_id.clone()));
                                        let quote_id = sent_quote.clone();
                                        busy.set(true);
                                        spawn(async move {
                                            let result = runtime::commit(async move {
                                                client
                                                    .message()
                                                    .send_contact(id, user.user_id, quote_id, false)
                                                    .await
                                            })
                                            .await;
                                            busy.set(false);
                                            if result.is_ok() {
                                                if quote.with(|q| {
                                                    q.as_ref().map(|m| m.message_id.clone())
                                                }) == sent_quote
                                                {
                                                    quote.set(None);
                                                }
                                                close();
                                            } else {
                                                failed.set(true);
                                                local_failed.set(true);
                                            }
                                        });
                                    })
                            },
                        )
                        .keys(move || {
                            rows.with(|rows| {
                                rows.iter().map(|u| u.user_id.clone()).collect::<Vec<_>>()
                            })
                        })
                        .estimated_item_size(px(56.0))
                        .h(move || px(rows.with(|rows| rows.len().min(5)) as f32 * 56.0))
                        .flex_shrink(1.0)
                        .min_h_0()
                        .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
                    ),
            )
            .child(
                view()
                    .flex_none()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(when(
                        move || offset.get() > 0 || more.get(),
                        move || {
                            view()
                                .flex_row()
                                .gap_2()
                                .child(
                                    ui::action(
                                        move || tr(i18n, "Previous page", "上一页"),
                                        move || offset.update(|v| *v = (*v - 100).max(0)),
                                    )
                                    .disabled(move || {
                                        offset.get() == 0 || loading.get() || busy.get()
                                    }),
                                )
                                .child(
                                    ui::action(
                                        move || tr(i18n, "Next page", "下一页"),
                                        move || offset.update(|v| *v += 100),
                                    )
                                    .disabled(move || !more.get() || loading.get() || busy.get()),
                                )
                        },
                    ))
                    .child(ui::action(move || i18n.text(Key::Cancel), cancel).disabled(busy)),
            ),
    )
}

pub(super) fn post(
    account: Arc<AccountClient>,
    conversation: String,
    close: impl Fn() + Copy + 'static,
    quote: Signal<Option<MessageListView>>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let content = signal(String::new());
    let busy = signal(false);
    let composing = signal(false);
    let failed = signal(false);
    let client = stored(account);
    let id = stored(conversation);
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            close();
        }
    };
    ui::modal(
        presence,
        cancel,
        view()
            .w(px(640.0))
            .max_w(relative(1.0))
            .p_6()
            .gap_4()
            .rounded_xl()
            .bg(move || theme().popover)
            .child(
                text(move || tr(i18n, "Write a post", "撰写文章"))
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                ui::field()
                    .multiline(true)
                    .auto_focus(true)
                    .h_64()
                    .label(move || i18n.text(Key::Post))
                    .readonly(busy)
                    .placeholder(move || {
                        tr(i18n, "Write in Markdown…", "使用 Markdown 撰写…").to_owned()
                    })
                    .value(move || content.get())
                    .on_key_press(move |key: &str| {
                        if key == "Escape" {
                            cancel();
                        }
                    })
                    .on_composition_change(move |event: &Composition| {
                        composing.set(event.composing)
                    })
                    .on_change(move |value: &str| content.set(value.to_owned())),
            )
            .child(when(failed, move || {
                text(move || i18n.text(Key::CommandFailed))
                    .text_sm()
                    .text_color(move || theme().destructive)
            }))
            .child(
                view()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        ui::action(move || i18n.text(Key::Cancel), cancel)
                            .disabled(move || busy.get() || composing.get()),
                    )
                    .child(
                        ui::primary()
                            .disabled(move || busy.get() || content.with(|s| s.trim().is_empty()))
                            .child(text(move || i18n.text(Key::Send)))
                            .on_press(move || {
                                if !presence.interactive() {
                                    return;
                                }
                                let client = client.with(|a| a.clone());
                                let id = id.with(|id| id.clone());
                                let body = content.get_untracked();
                                let sent_quote =
                                    quote.with(|q| q.as_ref().map(|m| m.message_id.clone()));
                                busy.set(true);
                                failed.set(false);
                                spawn(async move {
                                    let result = runtime::commit(async move {
                                        client.message().send_post(id, body).await
                                    })
                                    .await;
                                    busy.set(false);
                                    if result.is_ok() {
                                        if quote.with(|q| q.as_ref().map(|m| m.message_id.clone()))
                                            == sent_quote
                                        {
                                            quote.set(None);
                                        }
                                        close();
                                    } else {
                                        failed.set(true);
                                    }
                                });
                            }),
                    ),
            ),
    )
}
