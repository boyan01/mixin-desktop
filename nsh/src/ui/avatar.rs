//! Avatars and the frames they come in.

use nsh_base::{AvatarElement, avatar, theme};
use nsh_core::{element::ImageSrc, prelude::*};

use super::tokens::{TextRole, Typography, number};

#[derive(Clone, Copy)]
pub(crate) enum AvatarSize {
    /// A rail or menu identity.
    Small,
    /// A dense row: a mention, a message sender.
    Compact,
    /// The identity in a title bar.
    Bar,
    /// A row of a dialog or list.
    Medium,
    /// A conversation list cell.
    Large,
}

impl AvatarSize {
    fn points(self) -> Points {
        px(number(match self {
            Self::Small => "avatar-sm",
            Self::Compact => "avatar-compact",
            Self::Bar => "avatar-bar",
            Self::Medium => "avatar-md",
            Self::Large => "avatar-lg",
        }))
    }
}

pub(crate) fn avatar_view(
    name: impl Fn() -> SharedString + 'static,
    source: impl Fn() -> String + 'static,
    size: AvatarSize,
) -> AvatarElement {
    avatar()
        .name(move || Some(name()))
        .src(move || {
            let url = source();
            (!url.is_empty()).then(|| ImageSrc::from(url))
        })
        .size(size.points())
        .flex_shrink_0()
        .rounded_full()
        .overflow_hidden()
        .image_class("rounded-full object-cover")
        .fallback_style(|s| {
            s.items_center()
                .justify_center()
                .bg(move || theme().accent)
                .text_color(move || theme().accent_foreground)
                .text_role(TextRole::Body)
                .text_emphasis()
        })
}

/// Use the same one-to-four participant puzzle as the Flutter conversation list.
pub(crate) fn conversation_avatar(
    row: impl Fn() -> Option<mixin_desktop_api::ConversationListData> + 'static,
    size: AvatarSize,
) -> impl IntoElement {
    // A new message changes the row, not its picture; keep the loaded images.
    let identity = computed(move || {
        row().map(|row| {
            let puzzle = row.category == "GROUP" && !row.group_avatars.is_empty();
            let avatars = puzzle.then(|| row.group_avatars.into_iter().take(4).collect::<Vec<_>>());
            (row.name, row.avatar_url, avatars)
        })
    });
    view()
        .size(size.points())
        .flex_shrink_0()
        .rounded_full()
        .overflow_hidden()
        .child(dynamic(move || {
            let identity = identity.get();
            untracked(|| match identity {
                None => view().into_element(),
                Some((name, url, None)) => {
                    let name = stored(name);
                    let url = stored(url);
                    avatar_view(
                        move || name.with(|s| s.clone().into()),
                        move || url.with(|s| s.clone()),
                        size,
                    )
                    .size_full()
                    .into_element()
                }
                Some((_, _, Some(avatars))) => avatar_puzzle(
                    avatars
                        .into_iter()
                        .map(|avatar| (avatar.name, avatar.avatar_url))
                        .collect(),
                )
                .into_element(),
            })
        }))
}

/// One to four `(name, avatar URL)` tiles filling their parent, which clips them.
pub(crate) fn avatar_puzzle(avatars: Vec<(String, String)>) -> View {
    let tile = |at: usize| {
        let name = stored(avatars[at].0.clone());
        let url = stored(avatars[at].1.clone());
        view().flex_1().min_w_0().min_h_0().overflow_hidden().child(
            avatar_view(
                move || name.with(|s| s.clone().into()),
                move || url.with(|s| s.clone()),
                AvatarSize::Small,
            )
            .size_full()
            .rounded_none()
            .image_class("object-cover"),
        )
    };
    match avatars.len() {
        1 => tile(0),
        2 => view().flex_1().flex_row().child(tile(0)).child(tile(1)),
        3 => view()
            .flex_1()
            .flex_row()
            .child(tile(0))
            .child(view().flex_1().min_w_0().child(tile(1)).child(tile(2))),
        _ => view()
            .flex_1()
            .flex_row()
            .child(view().flex_1().min_w_0().child(tile(0)).child(tile(1)))
            .child(view().flex_1().min_w_0().child(tile(2)).child(tile(3))),
    }
}
