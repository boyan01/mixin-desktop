//! A read-only editor exposes selection without a second message source of truth.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use mixin_desktop_api::MessageListView;
use nsh_base::rich_editor::{
    extensions::link::link_at,
    state::{
        ClickEvent, EditorState, EditorStateConfig, KeyEvent, PluginSpec, TextSelection,
        Transaction,
    },
    view::{EditorSession, NodeViews, RichEditorClasses, RichEditorSelection, rich_editor},
};
use nsh_core::{TouchEvent, TouchPhase, prelude::*};
use tokio::sync::mpsc;

use crate::{
    chat::{mentions::MentionNames, message_actions::Actions, message_links},
    runtime,
};

mod document;

#[derive(Clone)]
struct Current {
    root: Option<String>,
    id: String,
    session: EditorSession,
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
            let current = current.as_ref()?;
            if current.id != id || current.root.as_deref() != root {
                return None;
            }
            let state = current.session.state();
            let selection = state.selection();
            (!selection.empty())
                .then(|| {
                    state
                        .doc()
                        .text_between(selection.from(), selection.to(), Some("\n"), None)
                })
                .filter(|text| !text.is_empty())
        })
    }
}

/// The click hook records a candidate; release decides whether it was a click
/// or a text-selection drag. Opening on press-down would prevent link selection.
#[derive(Clone)]
struct Links {
    candidate: Arc<Mutex<Option<String>>>,
    open: mpsc::Sender<String>,
}

impl Links {
    fn href(state: &EditorState, pos: usize) -> Option<String> {
        let ty = state.schema().mark_type("link")?;
        let (mark, _, _) = link_at(state.doc(), pos, &ty)?;
        mark.attrs().get("href")?.as_str().map(str::to_owned)
    }
    fn take(&self) -> Option<String> {
        self.candidate
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
    }
}

impl PluginSpec for Links {
    fn handle_click(
        &self,
        state: &EditorState,
        pos: usize,
        event: &ClickEvent,
        _: &mut dyn FnMut(Transaction),
    ) -> bool {
        *self.candidate.lock().unwrap_or_else(|p| p.into_inner()) = (event.button == 0)
            .then(|| Self::href(state, pos))
            .flatten();
        false
    }
    fn handle_key_down(
        &self,
        state: &EditorState,
        event: &KeyEvent,
        _: &mut dyn FnMut(Transaction),
    ) -> bool {
        if event.key != "Enter" || event.alt || event.shift || !state.selection().empty() {
            return false;
        }
        let Some(href) = Self::href(state, state.selection().head()) else {
            return false;
        };
        let _ = self.open.try_send(href);
        true
    }
}

pub(in crate::chat) fn read(
    row: &MessageListView,
    source: &str,
    actions: Actions,
    root: Option<String>,
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
    let schema = document::schema();
    let initial = document::value(&mapped.get_untracked());
    let initial = schema
        .node_from_json(&initial)
        .expect("Message display fits its schema");
    let (open, mut receive) = mpsc::channel(4);
    let links = Links {
        candidate: Arc::new(Mutex::new(None)),
        open,
    };
    let state = EditorState::create(
        EditorStateConfig::new(schema.clone())
            .doc(initial)
            .plugins(vec![Arc::new(links.clone())]),
    )
    .expect("Read-only message state is valid");
    let session = EditorSession::new(state, NodeViews::new());
    session.set_editable(false);
    let session = stored(session);
    let current = stored(Current {
        root,
        id: row.message_id.clone(),
        session: session.with(|s| s.clone()),
    });
    let register = move || {
        actions
            .text_selection
            .current
            .replace(Some(current.with(|value| value.clone())))
    };
    let cleanup_session = session.get();
    cleanup(move || {
        if actions.text_selection.owner.is_live() {
            actions.text_selection.current.with(|active| {
                if active
                    .as_ref()
                    .is_some_and(|active| active.session == cleanup_session)
                {
                    *active = None;
                }
            });
        }
    });
    let update_links = links.clone();
    effect(move || {
        let value = document::value(&mapped.get());
        session.with(|session| {
            let old = session.state();
            if old.doc().to_json() == value {
                return;
            }
            let doc = schema
                .node_from_json(&value)
                .expect("Message display fits its schema");
            // Highlight-only changes keep the selected characters. Resolved
            // display-name changes clear it rather than retarget old offsets.
            let mut config = EditorStateConfig::new(schema.clone())
                .doc(doc.clone())
                .plugins(vec![Arc::new(update_links.clone())]);
            if old.doc().text_content() == doc.text_content() {
                let last = doc.content().size().saturating_sub(1).max(1);
                config = config.selection(TextSelection::create(
                    &doc,
                    old.selection().anchor().clamp(1, last),
                    Some(old.selection().head().clamp(1, last)),
                ));
            }
            let state = EditorState::create(config).expect("Read-only message state is valid");
            session.commit(state, &[]);
        });
    });
    spawn(async move {
        while let Some(url) = receive.recv().await {
            if let Some(identity) = url.strip_prefix(super::MENTION_LINK) {
                let identity = identity.to_owned();
                let client = actions.account.with(|account| account.clone());
                match runtime::run(
                    async move { client.user().user_profile(None, Some(identity)).await },
                )
                .await
                {
                    Ok(Some(user)) => actions.show_user(user.user_id),
                    Ok(None) | Err(_) => actions.failed.set(true),
                }
            } else {
                message_links::open(actions, url, conversation.with(|id| id.clone()));
            }
        }
    });
    let pointer = stored(None::<(u32, Point<Points>)>);
    let links = stored(links);
    let mut classes = RichEditorClasses::default();
    // The editor's internal flex-1 scroll otherwise collapses an unconstrained
    // axis. One grid cell keeps both intrinsic text width and full text height,
    // so the timeline remains the only region with a vertical scroll range.
    classes.root = Some("grid".into());
    classes.marks = Some(BTreeMap::from([
        ("link".into(), "text-primary underline".into()),
        (
            "highlight".into(),
            "bg-accent text-accent-foreground".into(),
        ),
    ]));
    let editor = rich_editor()
        .editor(session.with(|session| Some(session.clone())))
        .editable(false)
        .classes(classes)
        .flex_none()
        .min_w_0()
        .text_size(move || appearance.primary_size())
        .on_focus(move || {
            register();
        })
        .on_selection_change(move |_: &RichEditorSelection| {
            // A background display-name/highlight refresh must not take the
            // current selection away from a different message's view.
            if session.with(|session| session.is_focused())
                || actions.text_selection.current.with(|active| {
                    active
                        .as_ref()
                        .is_some_and(|active| session.with(|session| active.session == *session))
                })
            {
                register();
            }
        });
    view()
        .min_w_0()
        .on_touch(move |event: &TouchEvent| {
            let point = event.points.iter().find(|point| point.changed);
            match event.phase {
                TouchPhase::Start => {
                    links.with(|links| {
                        links.take();
                    });
                    pointer.replace(
                        point
                            .filter(|_| event.points.len() == 1)
                            .map(|point| (point.id, point.page)),
                    );
                }
                TouchPhase::Move => {
                    if let (Some((id, start)), Some(point)) = (pointer.get(), point)
                        && (point.id != id
                            || (point.page.x.0 - start.x.0).hypot(point.page.y.0 - start.y.0) > 4.0)
                    {
                        pointer.replace(None);
                        links.with(|links| {
                            links.take();
                        });
                    }
                }
                TouchPhase::End => {
                    let candidate = links.with(|links| links.take());
                    if pointer.replace(None).is_some()
                        && session.with(|session| session.state().selection().empty())
                        && let Some(url) = candidate
                    {
                        links.with(|links| {
                            let _ = links.open.try_send(url);
                        });
                    }
                }
                TouchPhase::Cancel => {
                    pointer.replace(None);
                    links.with(|links| {
                        links.take();
                    });
                }
            }
        })
        .child(editor)
}
