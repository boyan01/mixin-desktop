//! The editor document is presentation state; drafts and sends always use wire text.
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use mixin_desktop_api::AccountClient;
use nsh_base::{
    rich_editor::{
        extensions::history,
        model::Slice,
        state::{EditorState, EditorStateConfig, KeyEvent, PluginSpec, TextSelection, Transaction},
        view::{
            EditorSession, NodeViews, RichEditorChange, RichEditorClasses, RichEditorSelection,
            rich_editor,
        },
    },
    theme,
};
use nsh_core::prelude::*;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::runtime;

mod document;
use document::{WireDocument, document, mention_tokens, mentions_differ, refresh_mentions, schema};

enum Command {
    Changed,
    Submit,
    Picker(String),
}

/// An owner-scoped input, shared only with the composer's existing commands.
#[derive(Clone, Copy)]
pub(crate) struct Input {
    session: Stored<EditorSession>,
    picker_open: Stored<Arc<AtomicBool>>,
    pub keys: Signal<(u64, String)>,
    submit: Stored<Option<Rc<dyn Fn()>>>,
    deferred: Stored<Vec<Rc<dyn Fn()>>>,
    commands: Stored<mpsc::UnboundedSender<Command>>,
}

impl Input {
    pub fn mount(
        account: Arc<AccountClient>,
        draft: Memo<String>,
        selection: Signal<(u32, u32)>,
        write: Rc<dyn Fn(String)>,
    ) -> Self {
        let (commands, receive) = mpsc::unbounded_channel();
        let picker_open = Arc::new(AtomicBool::new(false));
        let session = editor_session(
            &draft.get_untracked(),
            selection.get_untracked(),
            commands.clone(),
            picker_open.clone(),
        );
        let changed = commands.clone();
        let subscription = session.subscribe(Arc::new(move |_| {
            let _ = changed.send(Command::Changed);
        }));
        let dispose = session.clone();
        cleanup(move || dispose.unsubscribe(subscription));
        let input = Self {
            session: stored(session),
            picker_open: stored(picker_open),
            keys: signal((0, String::new())),
            submit: stored(None),
            deferred: stored(Vec::new()),
            commands: stored(commands),
        };
        input.run_commands(receive, draft, selection, write);
        input.follow_draft(draft, selection, resolved_mentions(account, draft));
        input
    }

    /// Session callbacks may run outside the engine thread. Only this owner task
    /// writes signals, and disposal cancels it together with the editor.
    fn run_commands(
        self,
        mut receive: mpsc::UnboundedReceiver<Command>,
        draft: Memo<String>,
        selection: Signal<(u32, u32)>,
        write: Rc<dyn Fn(String)>,
    ) {
        spawn(async move {
            while let Some(command) = receive.recv().await {
                match command {
                    Command::Changed => {
                        let state = self.session.with(|session| session.state());
                        let wire = WireDocument::read(&state.doc().to_json());
                        batch(|| {
                            selection.set(
                                wire.to_wire((
                                    state.selection().anchor(),
                                    state.selection().head(),
                                )),
                            );
                            if wire.text != draft.get_untracked() {
                                write(wire.text);
                            }
                        });
                        self.flush();
                    }
                    Command::Submit => {
                        if !self.composing() {
                            self.submit.with(|send| {
                                if let Some(send) = send {
                                    send();
                                }
                            });
                        }
                    }
                    Command::Picker(key) => self.keys.update(|(revision, current)| {
                        *revision = revision.wrapping_add(1);
                        *current = key;
                    }),
                }
            }
        });
    }

    /// Applies drafts, carets and resolved names that did not come from the editor.
    fn follow_draft(
        self,
        draft: Memo<String>,
        selection: Signal<(u32, u32)>,
        names: Signal<HashMap<String, String>>,
    ) {
        effect(move || {
            let raw = draft.get();
            let caret = selection.get();
            let labels = names.get();
            // Never replace a native composition with a display-name refresh.
            // Its commit changes the draft and runs this binding again.
            if self.composing() {
                return;
            }
            self.session
                .with(|session| show_draft(session, &raw, caret, &labels));
        });
    }

    pub fn composing(self) -> bool {
        self.session.with(|session| session.composing())
    }
    pub fn focus(self) {
        self.session.with(|session| session.focus());
    }
    pub fn blur(self) {
        self.session.with(|session| session.blur());
    }
    pub fn picker(self, open: bool) {
        self.picker_open
            .with(|flag| flag.store(open, Ordering::Relaxed));
    }
    pub fn after_composition(self, action: Rc<dyn Fn()>) {
        self.deferred.with(|queue| queue.push(action));
        // First publish the latest committed native document. A native IME
        // commit may precede its owner-task notification in this same frame.
        self.publish();
    }
    fn publish(self) {
        self.commands.with(|commands| {
            let _ = commands.send(Command::Changed);
        });
    }
    fn flush(self) {
        if self.composing() {
            return;
        }
        let queue = self.deferred.with(std::mem::take);
        for action in queue {
            action();
        }
    }

    pub fn view(
        self,
        placeholder: impl Fn() -> String + 'static,
        sending: Memo<bool>,
        focused: Signal<bool>,
        pasted: Rc<dyn Fn(&nsh_base::rich_editor::view::RichEditorPastedFiles)>,
        submit: Rc<dyn Fn()>,
    ) -> impl IntoElement {
        self.submit.with(|callback| *callback = Some(submit));
        let mut classes = RichEditorClasses::default();
        // Intrinsic grid layout gives the editor's internal scroll its full
        // document height. The bounded application scroll below is the only
        // region with a vertical range, as in the readonly message reader.
        classes.root = Some("grid".into());
        classes.content = Some("px-3 py-2 text-sm text-foreground".into());
        classes.caret = Some("bg-primary".into());
        classes.nodes = Some(BTreeMap::from([("mention".into(), "text-primary".into())]));
        let placeholder = stored(placeholder);
        let scroll_ref = element_ref();
        let viewport = stored(Size::<Points>::default());
        let scroll_top = stored(px(0.0));
        let reveal_queued = stored(false);
        let reveal = move || {
            if reveal_queued.replace(true) {
                return;
            }
            // Native document geometry arrives after the editor's change
            // event. Wait for its commit and display frame, not a timed retry.
            after_answer(move || {
                request_animation_frame(move |_| {
                    reveal_queued.replace(false);
                    self.session.with(|session| {
                        if !session.is_focused() || session.composing() {
                            return;
                        }
                        let state = session.state();
                        let Some(caret) = session.coords_at_pos(state.selection().head()) else {
                            return;
                        };
                        let top = scroll_top.get();
                        let height = viewport.with(|size| size.height);
                        if height <= px(0.0) {
                            return;
                        }
                        let bottom = caret.origin.y + caret.size.height;
                        let target = if caret.origin.y < top {
                            caret.origin.y
                        } else if bottom > top + height {
                            bottom - height
                        } else {
                            return;
                        };
                        scroll_ref.scroll_to(Point::new(px(0.0), target.max(px(0.0))), false);
                    });
                });
            });
        };
        let editor = rich_editor()
            .editor(self.session.with(|session| Some(session.clone())))
            .editable(move || !sending.get())
            .placeholder(move || Some(placeholder.with(|label| label()).into()))
            .accessibility_label(move || Some(placeholder.with(|label| label()).into()))
            .classes(classes)
            .flex_none()
            .w_full()
            .min_w_0()
            .on_change(move |_: &RichEditorChange| reveal())
            .on_selection_change(move |_: &RichEditorSelection| reveal())
            .on_focus(move || {
                focused.set(true);
                reveal();
            })
            .on_blur(move || {
                focused.set(false);
                self.publish();
            })
            .on_paste_files(
                move |event: &nsh_base::rich_editor::view::RichEditorPastedFiles| pasted(event),
            );
        view()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .bg(move || theme().muted)
            .rounded(move || px(theme().number("radius-composer").unwrap_or(4.0)))
            .child(
                scroll()
                    .element_ref(&scroll_ref)
                    .min_h_10()
                    .max_h_32()
                    .on_layout(move |bounds: &Bounds<Points>| {
                        if viewport.with(|size| *size != bounds.size) {
                            viewport.replace(bounds.size);
                            reveal();
                        }
                    })
                    .on_scroll(move |event: &ScrollEvent| {
                        scroll_top.replace(event.offset.y);
                        viewport.replace(event.size);
                    })
                    .child(editor),
            )
    }
}

fn editor_session(
    draft: &str,
    caret: (u32, u32),
    commands: mpsc::UnboundedSender<Command>,
    picker_open: Arc<AtomicBool>,
) -> EditorSession {
    let schema = schema();
    let initial = document(draft, &HashMap::new());
    let doc = schema
        .node_from_json(&initial)
        .expect("Composer document matches its schema");
    let (anchor, head) = WireDocument::read(&initial).to_document(caret);
    let state = EditorState::create(
        EditorStateConfig::new(schema)
            .doc(doc.clone())
            .selection(TextSelection::create(&doc, anchor, Some(head)))
            .plugins(vec![
                Arc::new(Behavior {
                    commands,
                    picker_open,
                }),
                history::history(Default::default()),
            ]),
    )
    .expect("Composer state matches its schema");
    EditorSession::new(state, NodeViews::new())
}

/// Resolves names only when the set of mention tokens changes. Ordinary typing
/// reads nothing, and a rerun cancels a lookup for tokens no longer present.
fn resolved_mentions(
    account: Arc<AccountClient>,
    draft: Memo<String>,
) -> Signal<HashMap<String, String>> {
    let names = signal(HashMap::new());
    let tokens = computed(move || mention_tokens(&draft.get()));
    effect(move || {
        let tokens = tokens.get();
        if tokens.is_empty() {
            return;
        }
        let account = account.clone();
        spawn(async move {
            match runtime::run(async move { account.user().mention_names(vec![tokens]).await })
                .await
            {
                Ok(values) => names.set(values),
                Err(error) => log::warn!("Could not resolve draft mentions: {error}"),
            }
        });
    });
    names
}

fn show_draft(
    session: &EditorSession,
    raw: &str,
    caret: (u32, u32),
    names: &HashMap<String, String>,
) {
    let state = session.state();
    let old = WireDocument::read(&state.doc().to_json());
    let desired = document(raw, names);
    let mut tr = state.tr();
    if old.text != raw {
        let doc = state
            .schema()
            .node_from_json(&desired)
            .expect("Composer document matches its schema");
        if let Err(error) = tr.replace_with(0, state.doc().content().size(), doc.content().clone())
        {
            log::warn!("Could not update composer display: {error}");
            return;
        }
    } else if mentions_differ(&state.doc().to_json(), &desired) {
        if let Err(error) = refresh_mentions(&state, &old, &desired, &mut tr) {
            log::warn!("Could not refresh composer mentions: {error}");
            return;
        }
        // Only mention spans are replaced, so history mappings for
        // unrelated typing survive asynchronous name resolution.
        tr.set_meta("addToHistory", json!(false));
    }
    let (anchor, head) = WireDocument::read(&tr.doc().to_json()).to_document(caret);
    if tr.doc_changed() || state.selection().anchor() != anchor || state.selection().head() != head
    {
        tr.set_selection(TextSelection::create(tr.doc(), anchor, Some(head)));
        if let Err(error) = session.dispatch(tr) {
            log::warn!("Could not update composer selection: {error}");
        }
    }
}

struct Behavior {
    commands: mpsc::UnboundedSender<Command>,
    picker_open: Arc<AtomicBool>,
}
impl PluginSpec for Behavior {
    fn handle_key_down(
        &self,
        state: &EditorState,
        key: &KeyEvent,
        dispatch: &mut dyn FnMut(Transaction),
    ) -> bool {
        // Also releases queued insertions after an IME cancellation, which may
        // change no document and therefore emit no session transaction.
        let _ = self.commands.send(Command::Changed);
        let plain = !(key.shift || key.alt || key.ctrl || key.meta || key.mod_key);
        if plain
            && self.picker_open.load(Ordering::Relaxed)
            && matches!(
                key.key.as_str(),
                "Enter" | "Tab" | "Escape" | "ArrowUp" | "ArrowDown"
            )
        {
            let _ = self.commands.send(Command::Picker(key.key.clone()));
            return true;
        }
        if key.key == "Enter" && plain {
            let _ = self.commands.send(Command::Submit);
            return true;
        }
        if key.key == "Enter" && key.shift && !(key.alt || key.ctrl || key.meta || key.mod_key) {
            let node = state
                .schema()
                .node_type("hardBreak")
                .expect("Composer has line breaks")
                .create(None, Default::default(), vec![])
                .expect("Line break fits its schema");
            let mut tr = state.tr();
            tr.replace_selection_with(node, false);
            tr.scroll_into_view();
            dispatch(tr);
            return true;
        }
        if key.mod_key && key.key.eq_ignore_ascii_case("z") {
            return if key.shift {
                history::redo(state, Some(dispatch))
            } else {
                history::undo(state, Some(dispatch))
            };
        }
        false
    }
    // Preserve literal Markdown and plain clipboard text. HTML must not create
    // formatting or impersonate an authenticated mention node.
    fn transform_pasted_html(&self, _: &EditorState, _: String) -> String {
        String::new()
    }
    fn clipboard_text_serializer(&self, _: &EditorState, slice: &Slice) -> Option<String> {
        fn text(node: &Value) -> String {
            match node["type"].as_str() {
                Some("mention") => format!("@{}", node["attrs"]["id"].as_str().unwrap_or_default()),
                Some("hardBreak") => "\n".into(),
                Some("text") => node["text"].as_str().unwrap_or_default().to_owned(),
                _ => node["content"]
                    .as_array()
                    .map(|children| children.iter().map(text).collect())
                    .unwrap_or_default(),
            }
        }
        let value = slice.content.to_json();
        let children = value.as_array()?;
        let separator = if children
            .first()
            .is_some_and(|node| node["type"] == "paragraph")
        {
            "\n"
        } else {
            ""
        };
        Some(
            children
                .iter()
                .map(text)
                .collect::<Vec<_>>()
                .join(separator),
        )
    }
}
