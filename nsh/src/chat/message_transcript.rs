//! Bounded forwarded-message reading and attachment-only context actions.
use futures::StreamExt as _;
use mixin_desktop_api::{MessageListView, TranscriptSummaryItem};
use nsh_base::theme;
use nsh_core::{file_dialog::SaveDialog, prelude::*};

use super::{
    message_actions::{Actions, Detail, attachment_name, item},
    message_content,
};
use crate::{i18n::Key, presentation, runtime, ui};

pub(super) fn summary(root: String, child: String, actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let identity = stored((root, child));
    let rows = signal_with(Vec::<TranscriptSummaryItem>::new(), SignalEquals::Never);
    let loading = signal(true);
    let failed = signal(false);
    let retry = signal(0_u64);
    effect(move || {
        retry.get();
        let (root, child) = identity.with(|identity| identity.clone());
        let client = actions.account.with(|account| account.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(client.message_changes()));
            loop {
                let client = client.clone();
                let root = root.clone();
                let child = child.clone();
                match runtime::run(
                    async move { client.message().transcript_summary(root, child).await },
                )
                .await
                {
                    Ok(items) => {
                        rows.set(items);
                        failed.set(false);
                    }
                    Err(_) => {
                        rows.set(Vec::new());
                        failed.set(true);
                    }
                }
                loading.set(false);
                if changes.next().await.is_none() {
                    break;
                }
            }
        });
    });
    view().flex_1().w_full().min_h_0().gap_3()
        .child(text(move || i18n.text(Key::Transcript)).text_lg().font_weight(FontWeight::SEMIBOLD))
        .child(text(move || i18n.tr("This forwarded message contains a summary only. Original messages and attachments are unavailable.", "这条转发消息只保留了摘要，无法查看原始消息和附件。"))
            .text_sm().text_color(move || theme().muted_foreground))
        .child(when(loading, move || text(move || i18n.text(Key::LoadingMessages))))
        .child(when(failed, move || view().gap_2()
            .child(text(move || i18n.tr("This forwarded summary is unavailable.", "无法读取这条转发摘要。")))
            .child(ui::action(move || i18n.text(Key::Retry), move || retry.update(|retry| *retry += 1)))))
        .child(virtual_list(move || rows.with(Vec::len), move |at| {
            view().px_4().py_2().gap_1()
                .child(text(move || rows.with(|rows| rows.get(at.index()).map(|row| row.sender_name.clone()).unwrap_or_default()))
                    .text_sm().text_color(move || theme().primary))
                .child(text(move || rows.with(|rows| rows.get(at.index())
                    .map(|row| presentation::preview(i18n, &row.category, &row.content)).unwrap_or_default()))
                    .text_sm().select_text())
        }).flex_1().min_h_0().estimated_item_size(px(72.0)))
}

pub(super) fn screen(id: String, actions: Actions) -> impl IntoElement {
    ui::owned(id.clone(), move || reader(id.clone(), actions))
}

fn reader(id: String, actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let media_viewport = super::message_media::viewport();
    let id = stored(id);
    let rows = signal(Vec::<MessageListView>::new());
    super::mentions::MentionNames::mount(
        actions.account.with(|account| account.clone()),
        rows,
        None,
    );
    let offset = signal(0_i64);
    let more = signal(false);
    let loading = signal(true);
    let failed = signal(false);
    let retry = signal(0_u64);
    let visible = signal(None::<(usize, usize)>);
    let busy = signal(false);
    let owner = Owner::current();
    let active = move || {
        actions.preview.with(|preview| {
        matches!(preview, Some(Detail::Message(message))
            if message.category.ends_with("_TRANSCRIPT") && id.with(|root| message.message_id == *root))
    })
    };
    effect(move || {
        if !active() {
            return;
        }
        let page_offset = offset.get();
        let revision = retry.get();
        let transcript_id = id.with(|id| id.clone());
        let client = actions.account.with(|a| a.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            let mut changes = Box::pin(crate::runtime::observe(client.message_changes()));
            loop {
                let client = client.clone();
                let id = transcript_id.clone();
                let result = runtime::run(async move {
                    client
                        .message()
                        .transcript_messages_page(id, page_offset, 101)
                        .await
                })
                .await;
                if !owner.is_live()
                    || !active()
                    || offset.get_untracked() != page_offset
                    || retry.get_untracked() != revision
                {
                    return;
                }
                match result {
                    Ok(mut values) => {
                        more.set(values.len() > 100);
                        values.truncate(100);
                        rows.set(values);
                        failed.set(false);
                    }
                    Err(_) => failed.set(true),
                }
                loading.set(false);
                if changes.next().await.is_none() {
                    break;
                }
            }
        });
    });
    view()
        .flex_1()
        .w_full()
        .min_h_0()
        .gap_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || i18n.text(Key::Transcript))
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .flex_1(),
                )
                .child(ui::busy_indicator(
                    move || busy.get(),
                    move || i18n.tr("Loading…", "加载中…").into(),
                )),
        )
        .child(when(loading, move || {
            text(move || i18n.text(Key::LoadingMessages))
        }))
        .child(when(failed, move || {
            view()
                .gap_2()
                .child(text(move || i18n.text(Key::MessagesFailed)))
                .child(ui::action(
                    move || i18n.text(Key::Retry),
                    move || retry.update(|v| *v += 1),
                ))
        }))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    crate::sticker::visibility(move || {
                        active()
                            && actions.forwarding.with(Vec::is_empty)
                            && visible
                                .get()
                                .is_some_and(|(first, last)| (first..=last).contains(&at.index()))
                    });
                    view()
                        .px_4()
                        .py_2()
                        .gap_1()
                        .child(when(
                            move || {
                                rows.with(|rows| {
                                    rows.get(at.index()).is_some_and(|row| {
                                        at.index() == 0
                                            || rows.get(at.index() - 1).is_none_or(|previous| {
                                                presentation::day(i18n, previous.created_at_micros)
                                                    != presentation::day(
                                                        i18n,
                                                        row.created_at_micros,
                                                    )
                                            })
                                    })
                                })
                            },
                            move || {
                                view().items_center().py_2().child(
                                    text(move || {
                                        rows.with(|rows| {
                                            rows.get(at.index())
                                                .map(|row| {
                                                    presentation::day(i18n, row.created_at_micros)
                                                })
                                                .unwrap_or_default()
                                        })
                                    })
                                    .text_xs()
                                    .text_color(move || theme().muted_foreground),
                                )
                            },
                        ))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    text(move || {
                                        rows.with(|rows| {
                                            rows.get(at.index())
                                                .map(|m| m.sender_name.clone())
                                                .unwrap_or_default()
                                        })
                                    })
                                    .flex_1()
                                    .min_w_0()
                                    .number_of_lines(1)
                                    .text_sm()
                                    .text_color(move || theme().primary),
                                )
                                .child(
                                    text(move || {
                                        rows.with(|rows| {
                                            rows.get(at.index())
                                                .map(|row| {
                                                    presentation::time(
                                                        i18n,
                                                        row.created_at_micros / 1000,
                                                    )
                                                })
                                                .unwrap_or_default()
                                        })
                                    })
                                    .text_xs()
                                    .text_color(move || theme().muted_foreground),
                                ),
                        )
                        .child(dynamic(move || {
                            let row = rows.with(|rows| rows.get(at.index()).cloned());
                            untracked(|| match row {
                                Some(row) => {
                                    let root = id.with(|id| id.clone());
                                    attachment_menu(
                                        row.clone(),
                                        id,
                                        actions,
                                        busy,
                                        owner,
                                        message_content::body(
                                            row,
                                            actions,
                                            message_content::Placement {
                                                transcript: Some(root),
                                                ..Default::default()
                                            },
                                        ),
                                    )
                                    .into_element()
                                }
                                None => view().hidden().into_element(),
                            })
                        }))
                },
            )
            .keys(move || {
                rows.with(|rows| {
                    rows.iter()
                        .map(|m| m.message_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .flex_1()
            .min_h_0()
            .estimated_item_size(px(90.0))
            .on_viewable_items_change(move |items: &ViewableItems| visible.set(items.viewable))
            .on_layout(media_viewport),
        )
        .child(
            view()
                .flex_row()
                .justify_end()
                .gap_2()
                .child(
                    ui::action(
                        move || i18n.tr("Previous", "上一页"),
                        move || offset.update(|v| *v = (*v - 100).max(0)),
                    )
                    .disabled(move || offset.get() == 0 || loading.get()),
                )
                .child(
                    ui::action(
                        move || i18n.tr("Next", "下一页"),
                        move || offset.update(|v| *v += 100),
                    )
                    .disabled(move || !more.get() || loading.get()),
                ),
        )
}

fn attachment_menu(
    row: MessageListView,
    root: Stored<String>,
    actions: Actions,
    busy: Signal<bool>,
    owner: Owner,
    body: impl IntoElement + 'static,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let active = move || {
        actions.preview.with(|preview| {
        matches!(preview, Some(Detail::Message(message))
            if message.category.ends_with("_TRANSCRIPT") && root.with(|root| message.message_id == *root))
    })
    };
    let downloadable = message_content::is_attachment(&row) && message_content::available(&row);
    let copyable = !matches!(row.status.as_str(), "UNKNOWN" | "FAILED")
        && (((row.category.ends_with("_TEXT") || row.category.ends_with("_POST"))
            && !row.content.is_empty())
            || row
                .caption
                .as_ref()
                .is_some_and(|caption| !caption.is_empty()));
    ui::native_context_menu(body, move || {
        if busy.get_untracked() || !active() || (!copyable && !downloadable) {
            return Vec::new();
        }
        let mut items = Vec::new();
        let is_text = row.category.ends_with("_TEXT");
        let has_caption = row
            .caption
            .as_ref()
            .is_some_and(|caption| !caption.is_empty());
        let selected = (is_text || has_caption)
            .then(|| root.with(|root| actions.text_selection.selected(&row.message_id, Some(root))))
            .flatten();
        let has_selection = selected.is_some();
        let copy = if matches!(row.status.as_str(), "UNKNOWN" | "FAILED") {
            String::new()
        } else if is_text || row.category.ends_with("_POST") {
            selected.unwrap_or_else(|| row.content.clone())
        } else {
            selected.unwrap_or_else(|| row.caption.clone().unwrap_or_default())
        };
        if !copy.is_empty() {
            if is_text {
                let qr = copy.clone();
                items.push(item(
                    i18n.tr("Generate QR code", "生成二维码"),
                    move || {
                        if !busy.get_untracked() && active() {
                            actions.open_text_qr(qr.clone());
                        }
                    },
                ));
            }
            items.push(item(
                if has_selection {
                    i18n.tr("Copy selected text", "复制选中文字")
                } else {
                    i18n.tr("Copy text", "复制文字")
                },
                move || {
                    if busy.get_untracked() || !active() {
                        return;
                    }
                    busy.set(true);
                    let content = copy.clone();
                    let account = actions.account.with(|account| account.clone());
                    owner.run(move || {
                        spawn(async move {
                            let result = if is_text && !has_selection {
                                runtime::run(async move {
                                    Ok(account
                                        .user()
                                        .replace_mentions(vec![content])
                                        .await?
                                        .into_iter()
                                        .next()
                                        .unwrap_or_default())
                                })
                                .await
                            } else {
                                Ok(content)
                            };
                            if owner.is_live() {
                                match result {
                                    Ok(content) => {
                                        set_clipboard(content);
                                        actions.failed.set(false);
                                    }
                                    Err(_) => actions.failed.set(true),
                                }
                                busy.set(false);
                            }
                        })
                    });
                },
            ));
        }
        if message_content::is_attachment(&row) && message_content::available(&row) {
            if row.category.ends_with("_IMAGE") {
                let root = root.with(|root| root.clone());
                let id = row.message_id.clone();
                items.push(item(i18n.tr("Copy image", "复制图片"), move || {
                    if busy.get_untracked() || !active() {
                        return;
                    }
                    busy.set(true);
                    let root = root.clone();
                    let id = id.clone();
                    let account = actions.account.with(|account| account.clone());
                    owner.run(move || {
                        spawn(async move {
                            let result = runtime::run(async move {
                                account
                                    .attachment()
                                    .attachment_image_data_uri(id, Some(root))
                                    .await
                            })
                            .await;
                            if owner.is_live() {
                                match result {
                                    Ok(uri) => {
                                        set_clipboard_image(uri);
                                        actions.failed.set(false);
                                    }
                                    Err(_) => actions.failed.set(true),
                                }
                                busy.set(false);
                            }
                        })
                    });
                }));
            }
            let root = root.with(|root| root.clone());
            let id = row.message_id.clone();
            let name = attachment_name(&row);
            items.push(item(i18n.tr("Save as…", "另存为…"), move || {
                if busy.get_untracked() || !active() {
                    return;
                }
                busy.set(true);
                let root = root.clone();
                let id = id.clone();
                let name = name.clone();
                let account = actions.account.with(|account| account.clone());
                owner.run(move || {
                    spawn(async move {
                        let result = match pick_save_path(SaveDialog::new().file_name(name)).await {
                            Ok(Some(path)) => {
                                runtime::commit(async move {
                                    account
                                        .attachment()
                                        .save_transcript_attachment_as(
                                            root,
                                            id,
                                            path.to_string_lossy().into_owned(),
                                        )
                                        .await
                                })
                                .await
                            }
                            Ok(None) => Ok(()),
                            Err(_) => Err(mixin_desktop_api::ClientError::Internal(
                                "Could not choose an attachment destination".into(),
                            )),
                        };
                        if owner.is_live() {
                            actions.failed.set(result.is_err());
                            busy.set(false);
                        }
                    })
                });
            }));
        }
        items
    })
}
