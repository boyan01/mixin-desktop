//! Object creation flows following Flutter's contact/name selection order.
mod circle;
mod contacts;
mod lookup;

use crate::{
    i18n::{self, Key, tr},
    navigation::ChatNavigation,
    ui::parts,
    ui::{self, Typography as _},
};
use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Action {
    Search,
    Chat,
    Group,
    Circle,
}

impl Action {
    fn icon(self) -> ui::MenuIcon {
        match self {
            Self::Search => ui::MenuIcon::SearchUser,
            Self::Chat => ui::MenuIcon::CreateConversation,
            Self::Group => ui::MenuIcon::CreateGroup,
            Self::Circle => ui::MenuIcon::CircleOutline,
        }
    }

    fn title(self) -> Key {
        match self {
            Self::Search => Key::SearchContact,
            Self::Chat => Key::NewConversation,
            Self::Group => Key::NewGroup,
            Self::Circle => Key::NewCircle,
        }
    }
}

fn panel() -> View {
    ui::dialog_card(ui::DialogWidth::Regular).overflow_hidden()
}

fn modal(close: impl Fn() + 'static, card: View) -> nsh_core::element::Modal {
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    ui::held_dialog(presence, close, card)
}

pub(crate) fn trigger(action: Signal<Option<Action>>) -> impl IntoElement {
    let i18n = i18n::current();
    ui::menu()
        .offset(ui::space_xs())
        .trigger(
            ui::round_button()
                .size_8()
                .accessibility_label(move || Some(i18n.text(Key::NewConversation).into()))
                .child(ui::icon(ui::Icon::Add).size(ui::icon_lg())),
        )
        .children(
            [Action::Search, Action::Chat, Action::Group, Action::Circle]
                .into_iter()
                .map(move |value| {
                    ui::menu_row_icon(Some(value.icon()), move || i18n.text(value.title()))
                        .on_select(move || action.set(Some(value)))
                }),
        )
}

pub(crate) fn surface(
    account: Arc<AccountClient>,
    action: Signal<Option<Action>>,
    _selected: Signal<Option<ConversationListData>>,
) -> impl IntoElement {
    let account = stored(account);
    let owner = Owner::current();
    let failed = signal(false);
    let navigation = use_context::<ChatNavigation>().expect("Home provides navigation");
    on_key("mod+n", move |_| {
        if action.get_untracked().is_none() {
            action.set(Some(Action::Chat));
        }
    });
    on_key("shift+mod+n", move |_| {
        if action.get_untracked().is_none() {
            action.set(Some(Action::Group));
        }
    });
    effect(move || {
        if failed.get() {
            failed.set(false);
            // Clearing `failed` reruns this effect, which would cancel a task it owns.
            owner.run(|| {
                spawn(async move {
                    let _ = nsh_base::alert_dialog()
                        .title(tr("Couldn’t open conversation", "无法打开会话"))
                        .description(tr(
                            "Check your connection and try again.",
                            "请检查网络连接后重试。",
                        ))
                        .action("close", tr("Close", "关闭"))
                        .show()
                        .await;
                })
            });
        }
    });
    let presence = crate::motion::Presence::modal(move || action.get().is_some());
    let shown = signal(action.get_untracked());
    effect(move || {
        if let Some(kind) = action.get() {
            shown.set(Some(kind));
        }
    });
    when(
        move || presence.mounted(),
        move || {
            provide_context(presence);
            dynamic(move || {
                let kind = shown.get();
                untracked(|| match kind {
                    Some(Action::Search) => {
                        lookup::screen(account.with(|account| account.clone()), move || {
                            action.set(None)
                        })
                        .into_element()
                    }
                    Some(Action::Circle) => {
                        circle::screen(account.with(|account| account.clone()), move || {
                            action.set(None)
                        })
                        .into_element()
                    }
                    Some(kind @ (Action::Chat | Action::Group)) => contacts::screen(
                        account.with(|account| account.clone()),
                        kind,
                        move || action.set(None),
                        move |user| {
                            let account = account.with(|account| account.clone());
                            owner.run(|| navigation.open_user(account, user, failed));
                            action.set(None);
                        },
                    )
                    .into_element(),
                    None => view().hidden().into_element(),
                })
            })
        },
    )
}

#[cfg(debug_assertions)]
pub(crate) use {
    circle::preview as preview_circle, contacts::preview as preview_contacts,
    lookup::preview as preview_lookup,
};
