//! Message text as native selectable text. The selection a message's menu copies
//! is the one the platform reports, not a second message source of truth.
use mixin_desktop_api::MessageListView;
use nsh_base::text_view::{Block, Document, Inline};
use nsh_core::{LineHeight, element::TextSelectionEvent, prelude::*};

use crate::{
    chat::{mentions::MentionNames, message_actions::Actions, message_links},
    runtime,
};

const LINK_CLASS: &str = "text-primary";
/// Looser than the font's own lines, so that long messages read easily.
const MESSAGE_LINE_HEIGHT: LineHeight = LineHeight::Multiple(1.4);
const HIGHLIGHT_CLASS: &str = "bg-highlight text-foreground";

struct Current {
    root: Option<String>,
    id: String,
    text: String,
}

/// One current presentation selection per action surface, never a per-row cache.
#[derive(Clone, Copy)]
pub(in crate::chat) struct Selection {
    owner: Owner,
    current: Stored<Option<Current>>,
}

impl Selection {
    pub(in crate::chat) fn new() -> Self {
        Self {
            owner: Owner::current(),
            current: stored(None),
        }
    }

    pub(in crate::chat) fn selected(self, id: &str, root: Option<&str>) -> Option<String> {
        if !self.owner.is_live() {
            return None;
        }
        self.current.with(|current| {
            current
                .as_ref()
                .filter(|current| current.id == id && current.root.as_deref() == root)
                .map(|current| current.text.clone())
        })
    }

    /// A message's selection changed: an empty one releases it, if it holds it.
    /// A surface being disposed has nothing left to release.
    fn report(self, id: &str, root: Option<&str>, text: &str) {
        self.current.try_with(|current| {
            if !text.is_empty() {
                *current = Some(Current {
                    root: root.map(str::to_owned),
                    id: id.to_owned(),
                    text: text.to_owned(),
                });
            } else if current
                .as_ref()
                .is_some_and(|current| current.id == id && current.root.as_deref() == root)
            {
                *current = None;
            }
        });
    }
}

/// `room` is the width the status at the end of the text takes, where it has one.
pub(in crate::chat) fn read(
    row: &MessageListView,
    source: &str,
    actions: Actions,
    root: Option<String>,
    room: Option<f32>,
) -> impl IntoElement {
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let navigation = use_context::<crate::navigation::ChatNavigation>();
    let names = use_context::<MentionNames>();
    let source = stored(source.to_owned());
    let conversation = stored(row.conversation_id.clone());
    let mapped = computed(move || {
        let query = navigation
            .map(|nav| conversation.with(|id| nav.search_term(id)))
            .unwrap_or_default();
        source.with(|source| match names {
            Some(names) => names.0.with(|names| super::document(source, names, &query)),
            None => super::document(source, &Default::default(), &query),
        })
    });
    let room = stored(room.map(|width| SharedString::from(format!("w-[{width}px] h-px"))));
    let selection = actions.text_selection;
    // A recycled or removed row takes its selection with it. Its own stored
    // values are gone by then, so the cleanup keeps plain copies.
    let (released_id, released_root) = (row.message_id.clone(), root.clone());
    cleanup(move || selection.report(&released_id, released_root.as_deref(), ""));
    let id = stored(row.message_id.clone());
    let root = stored(root);
    let open = move |url: String| open_link(actions, conversation, url);
    view()
        .min_w_0()
        .selectable(true)
        .on_text_selection_change(move |event: &TextSelectionEvent| {
            id.with(|id| root.with(|root| selection.report(id, root.as_deref(), &event.text)));
        })
        .child(
            text("")
                .flex_none()
                .min_w_0()
                .text_size(move || appearance.primary_size())
                .line_height(MESSAGE_LINE_HEIGHT)
                .text_color(move || nsh_base::theme().foreground)
                .child(dynamic(move || {
                    let mut out = Vec::new();
                    mapped.with(|document| spans(document, open, &mut out));
                    if out.is_empty() {
                        // An empty message keeps its line.
                        out.push(text("\u{200B}").into_element());
                    }
                    out.extend(
                        room.get()
                            .map(|class| view().app_class(class).into_element()),
                    );
                    out
                })),
        )
}

/// The spans of `document`'s paragraphs: links press, highlights stand out.
fn spans(document: &Document, open: impl Fn(String) + Copy + 'static, out: &mut Vec<AnyElement>) {
    for (index, block) in document.blocks.iter().enumerate() {
        if index > 0 {
            out.push(text("\n\n").into_element());
        }
        match block {
            Block::Paragraph { content } => inline(content, None, false, open, out),
            block => out.push(text(Document::new(vec![block.clone()]).plain_text()).into_element()),
        }
    }
}

fn inline(
    content: &[Inline],
    link: Option<&str>,
    highlighted: bool,
    open: impl Fn(String) + Copy + 'static,
    out: &mut Vec<AnyElement>,
) {
    for node in content {
        let piece = match node {
            Inline::Text { text } => text.clone(),
            Inline::Break => "\n".to_owned(),
            Inline::Link { url, content, .. } => {
                inline(content, Some(url), highlighted, open, out);
                continue;
            }
            Inline::Highlight { content, .. } => {
                inline(content, link, true, open, out);
                continue;
            }
            node => Document::new(vec![Block::Paragraph {
                content: vec![node.clone()],
            }])
            .plain_text(),
        };
        if piece.is_empty() {
            continue;
        }
        let classes = [
            link.map(|_| LINK_CLASS),
            highlighted.then_some(HIGHLIGHT_CLASS),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let span = text(piece);
        let span = if classes.is_empty() {
            span
        } else {
            span.app_class(SharedString::from(classes.join(" ")))
        };
        out.push(match link {
            Some(url) => {
                let url = url.to_owned();
                span.role(AccessibilityRole::Link)
                    .on_press(move || open(url.clone()))
                    .into_element()
            }
            None => span.into_element(),
        });
    }
}

fn open_link(actions: Actions, conversation: Stored<String>, url: String) {
    let Some(identity) = url.strip_prefix(super::MENTION_LINK) else {
        message_links::open(actions, url, conversation.with(|id| id.clone()));
        return;
    };
    let identity = identity.to_owned();
    let client = actions.account.with(|account| account.clone());
    actions.task(async move {
        match runtime::run(async move { client.user().user_profile(None, Some(identity)).await })
            .await
        {
            Ok(Some(user)) => actions.show_user(user.user_id),
            Ok(None) | Err(_) => actions.failed.set(true),
        }
    });
}
