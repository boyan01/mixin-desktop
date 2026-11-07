//! One full reader for posts and downloaded Markdown attachments.
use mixin_desktop_api::MessageListView;
use nsh_base::{TextViewLinkPressEvent, text_view, theme};
use nsh_core::prelude::*;

use super::{
    message_actions::{Actions, Detail},
    message_links,
};
use crate::{i18n::Key, runtime, ui};

pub(super) fn reader(
    content: String,
    conversation_id: String,
    actions: Actions,
) -> impl IntoElement {
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    view().flex_1().min_h_0().w_full().items_center().child(
        scroll().flex_1().min_h_0().w_full().max_w(px(800.0)).child(
            text_view()
                .markdown(Some(SharedString::from(content)))
                .selectable(true)
                .on_link_press(move |event: &TextViewLinkPressEvent| {
                    message_links::open(actions, event.url.to_string(), conversation_id.clone())
                })
                .text_size(move || appearance.primary_size()),
        ),
    )
}

pub(super) fn attachment(
    row: MessageListView,
    transcript: Option<String>,
    actions: Actions,
) -> impl IntoElement {
    let i18n = actions.i18n;
    let owner = Owner::current();
    let row = stored(row);
    let transcript = stored(transcript);
    let loading = signal(true);
    let failed = signal(false);
    let content = signal(None::<String>);
    let retry = signal(0_u64);
    let active = move || {
        actions.preview.with(|detail| {
            matches!(detail, Some(Detail::Markdown { message, transcript: current_root })
            if row.with(|row| row.message_id == message.message_id)
                && transcript.with(|root| root == current_root))
        })
    };
    effect(move || {
        let revision = retry.get();
        let id = row.with(|row| row.message_id.clone());
        let root = transcript.with(|root| root.clone());
        let account = actions.account.with(|account| account.clone());
        batch(|| {
            loading.set(true);
            failed.set(false);
        });
        spawn(async move {
            let result =
                runtime::run(async move { account.attachment().markdown_content(id, root).await })
                    .await;
            if !owner.is_live() || !active() || retry.get_untracked() != revision {
                return;
            }
            batch(|| {
                loading.set(false);
                match result {
                    Ok(value) => content.set(Some(value)),
                    Err(error) => {
                        log::warn!("Could not read Markdown attachment: {error}");
                        failed.set(true);
                    }
                }
            });
        });
    });
    view()
        .flex_1()
        .min_h_0()
        .w_full()
        .gap_3()
        .child(
            text(row.with(|row| super::message_actions::attachment_name(row)))
                .text_sm()
                .number_of_lines(2)
                .select_text(),
        )
        .child(when(loading, move || {
            text(move || i18n.tr("Loading...", "加载中...")).text_sm()
        }))
        .child(when(failed, move || {
            view()
                .gap_3()
                .child(
                    text(move || {
                        i18n.format(
                            "Unable to open file: {arg0}",
                            "无法打开文件：{arg0}",
                            &[(
                                "arg0",
                                &row.with(|row| super::message_actions::attachment_name(row)),
                            )],
                        )
                    })
                    .text_sm()
                    .text_color(move || theme().destructive),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .child(ui::action(
                            move || i18n.text(Key::Retry),
                            move || retry.update(|value| *value = value.wrapping_add(1)),
                        ))
                        .child(ui::action(
                            move || i18n.tr("Open with default app", "使用默认应用打开"),
                            move || {
                                let path = row.with(|row| row.media_url.clone());
                                spawn(async move {
                                    if let Some(path) = path {
                                        actions.failed.set(open_path(path).await.is_err());
                                    }
                                });
                            },
                        )),
                )
        }))
        .child(dynamic(move || {
            content.with(|content| match content {
                Some(content) => reader(
                    content.clone(),
                    row.with(|row| row.conversation_id.clone()),
                    actions,
                )
                .into_element(),
                None => view().hidden().into_element(),
            })
        }))
}
