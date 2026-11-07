use mixin_desktop_api::{AccountClient, MessageListView};
use nsh_core::{
    file_dialog::{FileFilter, OpenDialog},
    prelude::*,
};
use std::sync::Arc;

use super::{AttachmentSource, files, forms, menu_row, tool_button, tr};
use crate::{
    i18n::{self, Key},
    motion::Presence,
    ui,
};

const MEDIA_EXTENSIONS: [&str; 13] = [
    "jpg", "jpeg", "png", "gif", "webp", "heic", "bmp", "mp4", "mov", "m4v", "webm", "mkv", "avi",
];

#[derive(Clone, PartialEq)]
enum Action {
    Contact,
    Post,
    Files(Vec<AttachmentSource>, files::Mode),
}

/// What every attachment dialog shares with the composer that opened it.
#[derive(Clone, Copy)]
pub(super) struct Host {
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    pub quote: Signal<Option<MessageListView>>,
    pub failed: Signal<bool>,
    pub presence: Presence,
    action: Signal<Option<Action>>,
}

impl Host {
    pub fn account(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    pub fn conversation(self) -> String {
        self.conversation.with(|id| id.clone())
    }

    pub fn close(self) {
        self.action.set(None);
    }
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
    let host = Host {
        account: stored(account),
        conversation: stored(id),
        quote,
        failed,
        presence: Presence::modal(move || action.with(Option::is_some)),
        action,
    };
    let pick = move |media: bool| {
        let dialog = OpenDialog::files().multiple(true);
        let dialog = if media {
            dialog.filter(FileFilter::new(
                tr(i18n, "Pictures and videos", "图片和视频"),
                MEDIA_EXTENSIONS,
            ))
        } else {
            dialog
        };
        spawn(async move {
            match pick_paths(dialog).await {
                Ok(Some(paths)) if !paths.is_empty() => {
                    let mode = if media {
                        files::Mode::Media
                    } else {
                        files::Mode::Files
                    };
                    action.set(Some(Action::Files(
                        paths
                            .into_iter()
                            .map(|path| AttachmentSource::Path(path.to_string_lossy().into_owned()))
                            .collect(),
                        mode,
                    )));
                }
                Ok(_) => {}
                Err(_) => failed.set(true),
            }
        });
    };
    view()
        .child(
            ui::menu()
                .offset(px(8.0))
                .trigger(
                    tool_button()
                        .accessibility_label(move || Some(i18n.text(Key::Attachment).into()))
                        .child(ui::icon(ui::Icon::Add).size_6()),
                )
                .child(menu_row(
                    move || tr(i18n, "Contact…", "联系人…"),
                    move || action.set(Some(Action::Contact)),
                ))
                .child(menu_row(
                    move || tr(i18n, "Files…", "文件…"),
                    move || pick(false),
                ))
                .child(menu_row(
                    move || tr(i18n, "Pictures and videos…", "图片和视频…"),
                    move || pick(true),
                ))
                .child(menu_row(
                    move || tr(i18n, "Post…", "文章…"),
                    move || action.set(Some(Action::Post)),
                )),
        )
        .child(when(
            move || host.presence.mounted(),
            move || {
                cleanup(move || {
                    let _ = shown.try_set(None);
                });
                dynamic(move || {
                    let selected = shown.get();
                    untracked(|| match selected {
                        Some(Action::Contact) => forms::contact(host).into_element(),
                        Some(Action::Post) => forms::post(host).into_element(),
                        Some(Action::Files(paths, mode)) => {
                            files::preview(host, paths, mode, incoming).into_element()
                        }
                        None => view().hidden().into_element(),
                    })
                })
            },
        ))
}
