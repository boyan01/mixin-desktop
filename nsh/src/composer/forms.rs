use mixin_desktop_api::UserProfileItem;
use nsh_core::prelude::*;

use super::{attachments::Host, clear_quote, quote_id, tr, wide_dialog};
use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

pub(super) fn contact(host: Host) -> impl IntoElement {
    let i18n = i18n::current();
    let presence = host.presence;
    let query = signal(String::new());
    let offset = signal(0_i64);
    let rows = signal_with(Vec::<UserProfileItem>::new(), SignalEquals::Never);
    let more = signal(false);
    let loading = signal(true);
    let load_failed = signal(false);
    let send_failed = signal(false);
    let retry = signal(0_u64);
    let busy = signal(false);
    let composing = signal(false);
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            host.close();
        }
    };
    effect(move || {
        let query = query.get();
        let offset = offset.get();
        retry.get();
        let client = host.account();
        loading.set(true);
        load_failed.set(false);
        // A rerun cancels this query. Rows stay until their replacement arrives.
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
                Err(_) => {
                    rows.set(Vec::new());
                    load_failed.set(true);
                }
            }
            loading.set(false);
        });
    });
    // A recycled list cell must not take the send, and `busy`, with it.
    let owner = Owner::current();
    let share = move |user: UserProfileItem| {
        if !presence.interactive() || busy.get_untracked() {
            return;
        }
        let client = host.account();
        let id = host.conversation();
        let sent_quote = quote_id(host.quote);
        let quote = sent_quote.clone();
        busy.set(true);
        send_failed.set(false);
        owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move {
                    client
                        .message()
                        .send_contact(id, user.user_id, quote, false)
                        .await
                })
                .await;
                busy.set(false);
                if result.is_ok() {
                    clear_quote(host.quote, sent_quote);
                    host.close();
                } else {
                    send_failed.set(true);
                }
            });
        });
    };
    // A 40pt avatar with the button's `space_sm` above and below it.
    let row_height = ui::field_height() + ui::space_lg();
    ui::modal(
        presence,
        cancel,
        ui::modal_card()
            .w_96()
            .overflow_hidden()
            .child(ui::modal_title(move || {
                tr(i18n, "Share contact", "分享联系人")
            }))
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
                    .child(when(
                        move || loading.get() && rows.with(Vec::is_empty),
                        move || {
                            let label = move || tr(i18n, "Loading contacts…", "正在加载联系人…");
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(ui::busy_indicator(
                                    move || loading.get(),
                                    move || label().into(),
                                ))
                                .child(text(label).text_role(ui::TextRole::Body))
                        },
                    ))
                    .child(when(load_failed, move || {
                        view()
                            .gap_2()
                            .child(ui::error_text(move || i18n.text(Key::CommandFailed)))
                            .child(ui::action(
                                move || i18n.text(Key::Retry),
                                move || retry.update(|v| *v += 1),
                            ))
                    }))
                    .child(when(send_failed, move || {
                        ui::error_text(move || i18n.text(Key::SendFailed))
                    }))
                    .child(when(
                        move || !loading.get() && !load_failed.get() && rows.with(Vec::is_empty),
                        move || ui::placeholder(move || i18n.text(Key::NoContacts)).min_h_24(),
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
                                        ui::AvatarSize::Medium,
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
                                                .text_role(ui::TextRole::Body)
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
                                                .text_role(ui::TextRole::Caption)
                                                .text_color(ui::fg_secondary),
                                            ),
                                    )
                                    .on_press(move || {
                                        if let Some(user) =
                                            rows.with(|rows| rows.get(at.index()).cloned())
                                        {
                                            share(user);
                                        }
                                    })
                            },
                        )
                        .keys(move || {
                            rows.with(|rows| {
                                rows.iter().map(|u| u.user_id.clone()).collect::<Vec<_>>()
                            })
                        })
                        .estimated_item_size(row_height)
                        .h(move || row_height * rows.with(|rows| rows.len().min(5)) as f32)
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

pub(super) fn post(host: Host) -> impl IntoElement {
    let i18n = i18n::current();
    let presence = host.presence;
    let content = signal(String::new());
    let busy = signal(false);
    let composing = signal(false);
    let failed = signal(false);
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            host.close();
        }
    };
    // Only the explicit Cancel discards writing; Escape and the backdrop keep it.
    let dismiss = move || {
        if content.with_untracked(|text| text.trim().is_empty()) {
            cancel();
        }
    };
    let send = move || {
        if !presence.interactive() || busy.get_untracked() {
            return;
        }
        let client = host.account();
        let id = host.conversation();
        let body = content.get_untracked();
        busy.set(true);
        failed.set(false);
        spawn(async move {
            let result =
                runtime::commit(async move { client.message().send_post(id, body).await }).await;
            busy.set(false);
            // A post carries no quote, so a pending reply stays for the next message.
            if result.is_ok() {
                host.close();
            } else {
                failed.set(true);
            }
        });
    };
    ui::modal(
        presence,
        dismiss,
        wide_dialog()
            .child(ui::modal_title(move || {
                tr(i18n, "Write a post", "撰写文章")
            }))
            .child(
                ui::textarea()
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
                            dismiss();
                        }
                    })
                    .on_composition_change(move |event: &Composition| {
                        composing.set(event.composing)
                    })
                    .on_change(move |value: &str| content.set(value.to_owned())),
            )
            .child(when(failed, move || {
                ui::error_text(move || i18n.text(Key::SendFailed))
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
                            .child(text(move || {
                                i18n.text(if busy.get() { Key::Sending } else { Key::Send })
                            }))
                            .on_press(send),
                    ),
            ),
    )
}
