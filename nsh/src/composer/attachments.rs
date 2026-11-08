use mixin_desktop_api::{AccountClient, MessageListView};
use nsh_core::prelude::*;
use std::sync::Arc;

use super::{Tray, forms, tr};
use crate::{
    i18n::{self, Key},
    motion::Presence,
    ui,
};

#[derive(Clone, PartialEq)]
enum Action {
    Contact,
    Post,
}

/// What every attachment dialog shares with the composer that opened it.
#[derive(Clone, Copy)]
pub(super) struct Host {
    account: Stored<Arc<AccountClient>>,
    conversation: Stored<String>,
    pub quote: Signal<Option<MessageListView>>,
    pub presence: Presence,
    action: Signal<Option<Action>>,
    navigation: crate::navigation::ChatNavigation,
}

impl Host {
    /// Closes the dialog after a message was accepted and shows it.
    pub fn sent(self) {
        self.navigation.show_latest();
        self.close();
    }

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
    quote: Signal<Option<MessageListView>>,
    tray: Tray,
) -> impl IntoElement {
    let action = signal(None::<Action>);
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
        presence: Presence::modal(move || action.with(Option::is_some)),
        action,
        navigation: use_context::<crate::navigation::ChatNavigation>()
            .expect("Home provides chat navigation"),
    };
    view()
        .child(attachment_menu(
            move || tray.pick(true),
            move || tray.pick(false),
            move || action.set(Some(Action::Contact)),
            move || action.set(Some(Action::Post)),
        ))
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
                        None => view().hidden().into_element(),
                    })
                })
            },
        ))
}

/// The same menu in the live composer and in preview scenes.
pub(crate) fn attachment_menu(
    media: impl Fn() + 'static,
    files: impl Fn() + 'static,
    contact: impl Fn() + 'static,
    post: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let open = signal(false);
    ui::menu()
        .open(move || Some(open.get()))
        .on_open_change(move |event: &nsh_base::MenuEvent| {
            if let nsh_base::MenuEvent::OpenChange(value, _) = event {
                open.set(*value);
            }
        })
        .offset(ui::space_sm())
        .trigger(
            super::surface::menu_trigger()
                .test_id("composer-attachment-trigger")
                .tooltip(ui::hint(move || {
                    tr(i18n, "Add attachment", "添加附件").to_owned()
                }))
                .style_when(open, |s| {
                    s.bg(ui::state_selected).text_color(ui::fg_primary)
                })
                .optional_label(move || Some(i18n.text(Key::Attachment).into()))
                .child(ui::icon(ui::Icon::Attachment)),
        )
        .child(
            ui::menu_row_symbol(ui::Icon::Image, move || {
                tr(i18n, "Pictures and videos…", "图片和视频…")
            })
            .on_select(media),
        )
        .child(
            ui::menu_row_symbol(ui::Icon::File, move || tr(i18n, "Files…", "文件…"))
                .on_select(files),
        )
        .child(
            nsh_base::menu_separator()
                .h(ui::hairline_width())
                .my_1()
                .bg(ui::hairline),
        )
        .child(
            ui::menu_row_symbol(ui::Icon::Person, move || tr(i18n, "Contact…", "联系人…"))
                .on_select(contact),
        )
        .child(
            ui::menu_row_symbol(ui::Icon::Post, move || tr(i18n, "Post…", "文章…")).on_select(post),
        )
}
