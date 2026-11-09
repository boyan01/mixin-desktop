//! Bounded forwarded-message reading and attachment-only context actions.
use futures::StreamExt as _;
use mixin_desktop_api::{MessageListView, TranscriptSummaryItem};
use nsh_core::{file_dialog::SaveDialog, prelude::*};

use super::{
    message_actions::{Actions, Detail, attachment_name, item},
    message_content,
};
use crate::{
    i18n::Key,
    runtime,
    ui::{self, Typography},
};

pub(super) fn summary(root: String, child: String, actions: Actions) -> impl IntoElement {
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
            let mut changes = Box::pin(runtime::observe(client.message_changes(None)));
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
    summary_view(actions, rows, loading, failed, retry)
}

pub(super) fn screen(id: String, actions: Actions) -> impl IntoElement {
    ui::owned(id.clone(), move || reader(id.clone(), actions))
}

fn reader(id: String, actions: Actions) -> impl IntoElement {
    let id = stored(id);
    let rows = signal(Vec::<MessageListView>::new());
    super::mentions::MentionNames::mount(
        actions.account.with(|account| account.clone()),
        rows,
        None,
    );
    let limit = signal(100_i64);
    let more = signal(false);
    let loading = signal(true);
    let failed = signal(false);
    let retry = signal(0_u64);
    let visible = signal(None::<(usize, usize)>);
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
        let count = limit.get();
        let revision = retry.get();
        let transcript_id = id.with(|id| id.clone());
        let client = actions.account.with(|a| a.clone());
        loading.set(true);
        failed.set(false);
        spawn(async move {
            let mut changes = Box::pin(crate::runtime::observe(client.message_changes(None)));
            loop {
                let client = client.clone();
                let id = transcript_id.clone();
                let result = runtime::run(async move {
                    let mut rows = Vec::new();
                    let mut more = false;
                    // Each query stays bounded; refresh all loaded pages when
                    // attachment, sticker or message events change their contents.
                    for offset in (0..count).step_by(100) {
                        let mut page = client
                            .message()
                            .transcript_messages_page(id.clone(), offset, 101)
                            .await?;
                        more = page.len() > 100;
                        page.truncate(100);
                        rows.extend(page);
                        if !more {
                            break;
                        }
                    }
                    Ok((rows, more))
                })
                .await;
                if !owner.is_live()
                    || !active()
                    || limit.get_untracked() != count
                    || retry.get_untracked() != revision
                {
                    return;
                }
                match result {
                    Ok((values, has_more)) => {
                        more.set(has_more);
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
    reader_view(
        actions, id, rows, limit, more, loading, failed, retry, visible, active,
    )
}

pub(super) fn row_menu(
    row: impl Fn() -> Option<MessageListView> + Copy + 'static,
    root: Stored<String>,
    actions: Actions,
    body: impl IntoElement + 'static,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let busy = actions.busy;
    let owner = Owner::current();
    let active = move || {
        actions.preview.with(|preview| {
        matches!(preview, Some(Detail::Message(message))
            if message.category.ends_with("_TRANSCRIPT") && root.with(|root| message.message_id == *root))
    })
    };
    ui::native_context_menu(body, move || {
        if busy.get_untracked() || !active() {
            return Vec::new();
        }
        let Some(row) = row() else {
            return Vec::new();
        };
        let mut copy_group = Vec::new();
        if row.category.ends_with("_IMAGE") {
            let root = root.with(|root| root.clone());
            let id = row.message_id.clone();
            copy_group.push(
                item(
                    ui::MenuIcon::Copy,
                    i18n.tr("Copy Image", "复制图片"),
                    move || {
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
                    },
                )
                .disabled(!message_content::available(&row)),
            );
        }
        let is_text = row.category.ends_with("_TEXT");
        let is_post = row.category.ends_with("_POST");
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
        } else if is_text || is_post {
            selected.unwrap_or_else(|| row.content.clone())
        } else {
            selected.unwrap_or_else(|| row.caption.clone().unwrap_or_default())
        };
        if !copy.is_empty() {
            let qr = is_text.then(|| copy.clone());
            copy_group.push(item(
                ui::MenuIcon::Copy,
                if has_selection {
                    i18n.tr("Copy Selected Text", "复制已选择的文本")
                } else if is_text || is_post {
                    i18n.tr("Copy", "复制")
                } else {
                    i18n.tr("Copy Text", "复制文字")
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
            if let Some(qr) = qr {
                copy_group.push(item(
                    ui::MenuIcon::QrCode,
                    i18n.tr("Generate QR Code", "生成二维码"),
                    move || {
                        if !busy.get_untracked() && active() {
                            actions.open_text_qr(qr.clone());
                        }
                    },
                ));
            }
        }
        let mut save = Vec::new();
        if message_content::is_attachment(&row) && message_content::available(&row) {
            let root = root.with(|root| root.clone());
            let id = row.message_id.clone();
            let name = attachment_name(&row);
            save.push(item(
                ui::MenuIcon::Download,
                i18n.tr("Save as", "另存为"),
                move || {
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
                            let result =
                                match pick_save_path(SaveDialog::new().file_name(name)).await {
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
                },
            ));
        }
        let mut sticker = Vec::new();
        if row.category.ends_with("_STICKER")
            && let Some(id) = row.sticker_id.clone()
        {
            sticker.push(item(
                ui::MenuIcon::Sticker,
                i18n.tr("Add Sticker", "添加贴纸"),
                move || {
                    if busy.get_untracked() || !active() {
                        return;
                    }
                    let client = actions.account.with(|account| account.clone());
                    let id = id.clone();
                    actions.commit(async move { client.sticker().add_sticker(id).await });
                },
            ));
        }
        ui::menu_groups([copy_group, save, sticker])
    })
}

pub(crate) fn summary_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .gap_2()
        .child(text(move || {
            i18n.tr(
                "This forwarded summary is unavailable.",
                "无法读取这条转发摘要。",
            )
        }))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}

pub(crate) fn load_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .gap_2()
        .child(text(move || i18n.text(Key::MessagesFailed)))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}

pub(super) fn header(actions: Actions) -> View {
    let i18n = actions.i18n;
    view()
        .flex_row()
        .items_center()
        .px_4()
        .py_3()
        .child(
            ui::round_button()
                .size_10()
                .accessibility_label(move || {
                    Some(if actions.preview_history.with(Vec::is_empty) {
                        i18n.tr("Close", "关闭").into()
                    } else {
                        i18n.text(Key::Back).into()
                    })
                })
                .on_press(move || actions.back_preview())
                .child(dynamic(move || {
                    ui::icon(if actions.preview_history.with(Vec::is_empty) {
                        ui::Icon::Close
                    } else {
                        ui::Icon::ArrowLeft
                    })
                    .size(ui::icon_lg())
                })),
        )
        .child(
            ui::dialog_title(move || i18n.text(Key::Transcript))
                .text_role(ui::TextRole::Title)
                .text_center()
                .flex_1()
                .min_w_0(),
        )
        .child(view().size_10().flex_shrink_0())
}

/// Same bounds and background as Flutter's transcript dialog, constrained by the window.
pub(super) fn surface<S: Styled>(style: S) -> S {
    style
        .w(ui::content_max_width())
        .max_w(relative(1.0))
        .h_full()
        .max_h(relative(1.0))
        .min_h_0()
        .rounded(ui::radius_lg())
        .bg(ui::surface_chat)
        .overflow_hidden()
        .p_0()
        .gap_0()
}

#[allow(clippy::too_many_arguments)]
fn reader_view(
    actions: Actions,
    id: Stored<String>,
    rows: Signal<Vec<MessageListView>>,
    limit: Signal<i64>,
    more: Signal<bool>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
    visible: Signal<Option<(usize, usize)>>,
    active: impl Fn() -> bool + Copy + 'static,
) -> View {
    let i18n = actions.i18n;
    let media_viewport = super::message_media::viewport();
    let lines = super::message_row::Rows {
        actions,
        messages: rows,
        target: signal(None),
        highlight: signal(None),
        highlight_opacity: animated_value(0.0),
        unread: None,
        history: element_ref(),
        own_id: stored(actions.account.id()),
        sender_names: computed(|| true),
        viewable: visible,
        unobstructed: computed(move || active() && actions.forwarding.with(Vec::is_empty)),
        transcript: Some(id),
    };
    view()
        .flex_1()
        .w_full()
        .min_h_0()
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            move || {
                view().px_4().py_2().child(ui::busy_indicator(
                    || true,
                    move || i18n.text(Key::LoadingMessages).into(),
                ))
            },
        ))
        .child(when(actions.busy, move || {
            view().px_4().py_2().child(ui::busy_indicator(
                || true,
                move || i18n.tr("Loading…", "加载中…").into(),
            ))
        }))
        .child(when(failed, move || {
            view()
                .px_4()
                .py_2()
                .child(load_error(move || retry.update(|v| *v += 1)))
        }))
        .child(
            virtual_list(move || rows.with(Vec::len), move |at| lines.row(at))
                .keys(move || {
                    rows.with(|rows| {
                        rows.iter()
                            .map(|row| row.message_id.clone())
                            .collect::<Vec<_>>()
                    })
                })
                .flex_1()
                .min_h_0()
                .pb_4()
                .estimated_item_size(ui::list_row_height())
                .on_viewable_items_change(move |items: &ViewableItems| {
                    visible.set(items.viewable);
                    if active()
                        && !loading.get_untracked()
                        && !failed.get_untracked()
                        && more.get_untracked()
                        && items.viewable.is_some_and(|(_, last)| {
                            rows.with_untracked(|rows| last.saturating_add(10) >= rows.len())
                        })
                    {
                        limit.update(|limit| *limit += 100);
                    }
                })
                .on_layout(media_viewport),
        )
}

#[cfg(debug_assertions)]
pub(super) fn detached(
    actions: Actions,
    mut rows: Vec<MessageListView>,
    failed: Signal<bool>,
) -> View {
    rows.reverse();
    let body = reader_view(
        actions,
        stored("preview-transcript".into()),
        signal(rows),
        signal(100),
        signal(false),
        signal(false),
        failed,
        signal(0),
        signal(None),
        || true,
    );
    surface(view()).child(header(actions)).child(body)
}

fn summary_view(
    actions: Actions,
    rows: Signal<Vec<TranscriptSummaryItem>>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
) -> View {
    let i18n = actions.i18n;
    view()
        .flex_1()
        .w_full()
        .min_h_0()
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            move || {
                view().px_4().py_2().child(ui::busy_indicator(
                    || true,
                    move || i18n.text(Key::LoadingMessages).into(),
                ))
            },
        ))
        .child(when(failed, move || {
            view()
                .px_4()
                .py_2()
                .child(summary_error(move || retry.update(|retry| *retry += 1)))
        }))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    let first = move || {
                        rows.with(|rows| {
                            rows.get(at.index()).is_some_and(|row| {
                                at.index()
                                    .checked_sub(1)
                                    .and_then(|i| rows.get(i))
                                    .is_none_or(|previous| previous.sender_name != row.sender_name)
                            })
                        })
                    };
                    let last = move || {
                        rows.with(|rows| {
                            rows.get(at.index()).is_some_and(|row| {
                                rows.get(at.index() + 1)
                                    .is_none_or(|next| next.sender_name != row.sender_name)
                            })
                        })
                    };
                    super::message_row::summary(
                        move || rows.with(|rows| rows.get(at.index()).cloned()),
                        first,
                        last,
                        actions,
                    )
                },
            )
            .flex_1()
            .min_h_0()
            .pb_4()
            .estimated_item_size(ui::list_row_height()),
        )
        .child(
            view().px_6().py_3().child(
                text(move || {
                    i18n.tr(
                        "Summary only · Original messages and attachments are unavailable.",
                        "仅保留消息摘要，无法查看原始消息和附件。",
                    )
                })
                .text_role(ui::TextRole::Caption)
                .text_color(ui::fg_secondary),
            ),
        )
}

#[cfg(debug_assertions)]
pub(super) fn preview_summary(
    actions: Actions,
    rows: Vec<MessageListView>,
    failed: Signal<bool>,
) -> View {
    let rows = rows
        .into_iter()
        .rev()
        .map(|row| {
            let mut item = TranscriptSummaryItem::default();
            item.sender_name = row.sender_name;
            item.category = row.category;
            item.content = row.content;
            item
        })
        .collect();
    surface(view()).child(header(actions)).child(summary_view(
        actions,
        signal_with(rows, SignalEquals::Never),
        signal(false),
        failed,
        signal(0),
    ))
}
