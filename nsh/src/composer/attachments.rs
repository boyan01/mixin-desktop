use mixin_desktop_api::{AccountClient, MessageListView};
use nsh_base::{menu_item, theme};
use nsh_core::{file_dialog::OpenDialog, prelude::*};
use std::sync::Arc;

use super::{AttachmentSource, files, forms, tr};
use crate::{
    i18n::{self, Key},
    ui,
};

#[derive(Clone, PartialEq)]
enum Action {
    Contact,
    Post,
    Files(Vec<AttachmentSource>, files::Mode),
}

pub(crate) fn attachments(
    account: Arc<AccountClient>,
    id: String,
    failed: Signal<bool>,
    quote: Signal<Option<MessageListView>>,
    incoming: Signal<Vec<AttachmentSource>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let action = signal(None::<Action>);
    effect(move || {
        if incoming.with(Vec::is_empty) {
            return;
        }
        // An existing preview owns additions without replacing its editing state.
        if action.with_untracked(|action| matches!(action, Some(Action::Files(..)))) {
            return;
        }
        let mut paths = Vec::new();
        incoming.update(|items| paths = std::mem::take(items));
        if action.with_untracked(Option::is_some) {
            failed.set(true);
        } else {
            action.set(Some(Action::Files(paths, files::Mode::Auto)));
        }
    });
    let shown = signal(None::<Action>);
    effect(move || {
        if let Some(selected) = action.get() {
            shown.set(Some(selected));
        }
    });
    let presence = crate::motion::Presence::modal(move || action.with(Option::is_some));
    let account = stored(account);
    let id = stored(id);
    view()
        .child(
            ui::menu()
                .offset(px(8.0))
                .trigger(
                    ui::icon_button()
                        .size_10()
                        .px_1()
                        .py_1()
                        .accessibility_label(move || Some(i18n.text(Key::Attachment).into()))
                        .child(ui::icon(ui::Icon::Add).size_6()),
                )
                .child(
                    menu_item()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .hover(|s| s.bg(move || theme().accent))
                        .child(text(move || tr(i18n, "Contact…", "联系人…")).text_sm())
                        .on_select(move || action.set(Some(Action::Contact))),
                )
                .children([false, true].into_iter().map(move |media| {
                    menu_item()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .hover(|s| s.bg(move || theme().accent))
                        .child(
                            text(move || {
                                if media {
                                    tr(i18n, "Pictures and videos…", "图片和视频…")
                                } else {
                                    tr(i18n, "Files…", "文件…")
                                }
                            })
                            .text_sm(),
                        )
                        .on_select(move || {
                            spawn(async move {
                                match pick_paths(OpenDialog::files().multiple(true)).await {
                                    Ok(Some(paths)) if !paths.is_empty() => {
                                        action.set(Some(Action::Files(
                                            paths
                                                .into_iter()
                                                .map(|p| {
                                                    AttachmentSource::Path(
                                                        p.to_string_lossy().into_owned(),
                                                    )
                                                })
                                                .collect(),
                                            if media {
                                                files::Mode::Media
                                            } else {
                                                files::Mode::Files
                                            },
                                        )))
                                    }
                                    Ok(_) => {}
                                    Err(_) => failed.set(true),
                                }
                            });
                        })
                }))
                .child(
                    menu_item()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .hover(|s| s.bg(move || theme().accent))
                        .child(text(move || tr(i18n, "Post…", "文章…")).text_sm())
                        .on_select(move || action.set(Some(Action::Post))),
                ),
        )
        .child(when(
            move || presence.mounted(),
            move || {
                cleanup(move || {
                    let _ = shown.try_set(None);
                });
                dynamic(move || {
                    let selected = shown.get();
                    untracked(|| match selected {
                        Some(Action::Contact) => forms::contact(
                            account.with(|a| a.clone()),
                            id.with(|id| id.clone()),
                            quote,
                            move || action.set(None),
                            failed,
                            presence,
                        )
                        .into_element(),
                        Some(Action::Post) => forms::post(
                            account.with(|a| a.clone()),
                            id.with(|id| id.clone()),
                            move || action.set(None),
                            quote,
                            presence,
                        )
                        .into_element(),
                        Some(Action::Files(paths, mode)) => files::preview(
                            account.with(|a| a.clone()),
                            id.with(|id| id.clone()),
                            paths,
                            mode,
                            incoming,
                            quote,
                            move || action.set(None),
                            failed,
                            presence,
                        )
                        .into_element(),
                        None => view().hidden().into_element(),
                    })
                })
            },
        ))
}
