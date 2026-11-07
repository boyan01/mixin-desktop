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
        extensions::{history, starter::nodes, suggestion::mention_node},
        model::{Schema, SchemaSpec, Slice},
        state::{EditorState, EditorStateConfig, KeyEvent, PluginSpec, TextSelection, Transaction},
        view::{
            EditorSession, NodeViews, RichEditorChange, RichEditorClasses, RichEditorSelection,
            rich_editor,
        },
    },
    theme,
};
use nsh_core::prelude::*;
use regex::Regex;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::runtime;

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
        let (commands, mut receive) = mpsc::unbounded_channel();
        let picker_open = Arc::new(AtomicBool::new(false));
        let schema = schema();
        let initial = document(&draft.get_untracked(), &HashMap::new());
        let doc = schema
            .node_from_json(&initial)
            .expect("Composer document matches its schema");
        let positions = WireDocument::read(&initial);
        let (a, h) = positions.to_document(selection.get_untracked());
        let state = EditorState::create(
            EditorStateConfig::new(schema)
                .doc(doc.clone())
                .selection(TextSelection::create(&doc, a, Some(h)))
                .plugins(vec![
                    Arc::new(Behavior {
                        commands: commands.clone(),
                        picker_open: picker_open.clone(),
                    }),
                    history::history(Default::default()),
                ]),
        )
        .expect("Composer state matches its schema");
        let session = EditorSession::new(state, NodeViews::new());
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
        let write = stored(write);
        // Session callbacks may run outside the engine thread. Only this owner task
        // writes signals, and disposal cancels it together with the editor.
        spawn(async move {
            while let Some(command) = receive.recv().await {
                match command {
                    Command::Changed => {
                        let state = input.session.with(|session| session.state());
                        let wire = WireDocument::read(&state.doc().to_json());
                        batch(|| {
                            selection.set(
                                wire.to_wire((
                                    state.selection().anchor(),
                                    state.selection().head(),
                                )),
                            );
                            if wire.text != draft.get_untracked() {
                                write.with(|write| write(wire.text));
                            }
                        });
                        input.flush();
                    }
                    Command::Submit => {
                        if !input.composing() {
                            input.submit.with(|send| {
                                if let Some(send) = send {
                                    send();
                                }
                            });
                        }
                    }
                    Command::Picker(key) => input.keys.update(|(revision, current)| {
                        *revision = revision.wrapping_add(1);
                        *current = key;
                    }),
                }
            }
        });
        let names = signal(HashMap::<String, String>::new());
        let request = signal(0_u64);
        effect(move || {
            let text = draft.get();
            request.update(|id| *id = id.wrapping_add(1));
            let version = request.get_untracked();
            let account = account.clone();
            spawn(async move {
                let contents = text.clone();
                match runtime::run(
                    async move { account.user().mention_names(vec![contents]).await },
                )
                .await
                {
                    Ok(values)
                        if request.get_untracked() == version && draft.get_untracked() == text =>
                    {
                        names.set(values)
                    }
                    Ok(_) => {}
                    Err(error) => log::warn!("Could not resolve draft mentions: {error}"),
                }
            });
        });
        effect(move || {
            let raw = draft.get();
            let caret = selection.get();
            let labels = names.get();
            // Never replace a native composition with a display-name refresh.
            // Its commit changes the draft and runs this binding again.
            if input.composing() {
                return;
            }
            input.session.with(|session| {
                let state = session.state();
                let old = WireDocument::read(&state.doc().to_json());
                let desired = document(&raw, &labels);
                let mut tr = state.tr();
                if old.text != raw {
                    let doc = state
                        .schema()
                        .node_from_json(&desired)
                        .expect("Composer document matches its schema");
                    if let Err(error) =
                        tr.replace_with(0, state.doc().content().size(), doc.content().clone())
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
                let positions = WireDocument::read(&tr.doc().to_json());
                let (a, h) = positions.to_document(caret);
                if tr.doc_changed()
                    || state.selection().anchor() != a
                    || state.selection().head() != h
                {
                    tr.set_selection(TextSelection::create(tr.doc(), a, Some(h)));
                    if let Err(error) = session.dispatch(tr) {
                        log::warn!("Could not update composer selection: {error}");
                    }
                }
            });
        });
        input
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
                self.commands.with(|commands| {
                    let _ = commands.send(Command::Changed);
                });
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

fn schema() -> Schema {
    let mut spec = SchemaSpec::default();
    spec.nodes = vec![
        nodes::doc(),
        nodes::paragraph(),
        nodes::text(),
        nodes::hard_break(),
        mention_node("mention", "@"),
    ];
    Schema::new(spec).expect("Composer schema is valid")
}

fn document(raw: &str, names: &HashMap<String, String>) -> Value {
    static MENTION: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let regex =
        MENTION.get_or_init(|| Regex::new(r"@([0-9]{4,})").expect("Mention regex is valid"));
    let mut content = Vec::new();
    let mut from = 0;
    for found in regex.captures_iter(raw) {
        let token = found.get(0).expect("Mention has complete match");
        let id = &found[1];
        let Some(name) = names.get(id).filter(|name| !name.is_empty()) else {
            continue;
        };
        append_text(&mut content, &raw[from..token.start()]);
        content.push(
            json!({"type":"mention", "attrs":{"id":id,"label":name,"mentionSuggestionChar":"@"}}),
        );
        from = token.end();
    }
    append_text(&mut content, &raw[from..]);
    json!({"type":"doc","content":[{"type":"paragraph","content":content}]})
}

fn append_text(content: &mut Vec<Value>, value: &str) {
    for (at, line) in value.split('\n').enumerate() {
        if at != 0 {
            content.push(json!({"type":"hardBreak"}));
        }
        if !line.is_empty() {
            content.push(json!({"type":"text","text":line}));
        }
    }
}

fn mentions_differ(a: &Value, b: &Value) -> bool {
    fn collect(value: &Value, out: &mut Vec<Value>) {
        if value["type"] == "mention" {
            out.push(value["attrs"].clone());
        }
        if let Some(children) = value["content"].as_array() {
            for child in children {
                collect(child, out);
            }
        }
    }
    let (mut left, mut right) = (Vec::new(), Vec::new());
    collect(a, &mut left);
    collect(b, &mut right);
    left != right
}

fn refresh_mentions(
    state: &EditorState,
    old: &WireDocument,
    desired: &Value,
    tr: &mut Transaction,
) -> Result<(), String> {
    let new = WireDocument::read(desired);
    let mut replacements = BTreeMap::new();
    for (from, to, _) in &old.mentions {
        replacements.insert((*from, *to), None);
    }
    for (from, to, node) in &new.mentions {
        replacements.insert((*from, *to), Some(node));
    }
    for ((from, to), desired) in replacements.into_iter().rev() {
        let previous = old.mentions.iter().find(|(a, b, _)| *a == from && *b == to);
        if previous.map(|(_, _, node)| &node["attrs"]) == desired.map(|node| &node["attrs"]) {
            continue;
        }
        let (start, end) = old.to_document((from, to));
        let node = match desired {
            Some(node) => state
                .schema()
                .node_from_json(node)
                .map_err(|e| e.to_string())?,
            None => {
                let text = old
                    .text
                    .encode_utf16()
                    .skip(from as usize)
                    .take((to - from) as usize)
                    .collect::<Vec<_>>();
                state.schema().text(
                    &String::from_utf16(&text).map_err(|e| e.to_string())?,
                    vec![],
                )
            }
        };
        tr.replace_with_node(start, end, node)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Every boundary is valid Unicode and lies outside an atomic mention. Structural
/// paragraph boundaries consume two editor positions but only one wire newline.
struct WireDocument {
    text: String,
    boundaries: Vec<(u32, usize)>,
    mentions: Vec<(u32, u32, Value)>,
}
impl WireDocument {
    fn read(doc: &Value) -> Self {
        let mut value = Self {
            text: String::new(),
            boundaries: vec![(0, 1)],
            mentions: Vec::new(),
        };
        let mut position = 0;
        let mut units = 0;
        if let Some(blocks) = doc["content"].as_array() {
            for (at, block) in blocks.iter().enumerate() {
                if at != 0 {
                    value.text.push('\n');
                    units += 1;
                }
                position += 1;
                value.boundaries.push((units, position));
                if let Some(children) = block["content"].as_array() {
                    for child in children {
                        match child["type"].as_str() {
                            Some("mention") => {
                                let id = child["attrs"]["id"].as_str().unwrap_or_default();
                                let from = units;
                                value.text.push('@');
                                value.text.push_str(id);
                                position += 1;
                                units += 1 + id.encode_utf16().count() as u32;
                                value.mentions.push((from, units, child.clone()));
                                value.boundaries.push((units, position));
                            }
                            Some("hardBreak") => {
                                value.text.push('\n');
                                position += 1;
                                units += 1;
                                value.boundaries.push((units, position));
                            }
                            Some("text") => {
                                for ch in child["text"].as_str().unwrap_or_default().chars() {
                                    value.text.push(ch);
                                    position += ch.len_utf16();
                                    units += ch.len_utf16() as u32;
                                    value.boundaries.push((units, position));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                position += 1;
            }
        }
        value
    }
    fn to_document(&self, (a, h): (u32, u32)) -> (usize, usize) {
        (self.raw_position(a, a > h), self.raw_position(h, h > a))
    }
    fn raw_position(&self, raw: u32, round_up: bool) -> usize {
        if round_up {
            self.boundaries.iter().find(|(at, _)| *at >= raw)
        } else {
            self.boundaries.iter().rev().find(|(at, _)| *at <= raw)
        }
        .or_else(|| self.boundaries.last())
        .map(|(_, at)| *at)
        .unwrap_or(1)
    }
    fn to_wire(&self, (a, h): (usize, usize)) -> (u32, u32) {
        let position = |doc| {
            self.boundaries
                .iter()
                .rev()
                .find(|(_, at)| *at <= doc)
                .map(|(at, _)| *at)
                .unwrap_or(0)
        };
        (position(a), position(h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These pure boundary contracts protect message/draft data, including the
    // editor's atom positions. They cannot be covered in the NSH-free core.
    #[test]
    fn resolved_mentions_with_emoji_and_newlines_export_original_wire_text() {
        let raw = "😀 **literal**\n@12345 and @9999\n";
        let doc = document(raw, &HashMap::from([("12345".into(), "Alice😀".into())]));
        let schema = schema();
        let parsed = schema.node_from_json(&doc).unwrap();
        let wire = WireDocument::read(&parsed.to_json());
        assert_eq!(wire.text, raw);
        assert!(!wire.text.contains("Alice"));
        for &(raw, pos) in &wire.boundaries {
            let selection = TextSelection::create(&parsed, pos, Some(pos));
            assert_eq!(
                wire.to_wire((selection.anchor(), selection.head())),
                (raw, raw)
            );
            assert_eq!(wire.to_document((raw, raw)), (pos, pos));
        }
    }

    #[test]
    fn selection_crossing_mentions_and_surrogates_expands_to_complete_values() {
        let raw = "😀@12345x";
        let doc = document(raw, &HashMap::from([("12345".into(), "Alice".into())]));
        let wire = WireDocument::read(&doc);
        // A stale selection inside emoji and the mention may never split either.
        assert_eq!(wire.to_document((1, 4)), (1, 4));
        assert_eq!(wire.to_wire(wire.to_document((1, 4))), (0, 8));
        assert_eq!(wire.to_wire(wire.to_document((4, 1))), (8, 0));
        assert_eq!(wire.to_wire(wire.to_document((4, 4))), (2, 2));
    }

    #[test]
    fn pasted_paragraphs_map_newlines_and_keep_atomic_deletion_on_wire() {
        let doc = json!({"type":"doc", "content":[
            {"type":"paragraph","content":[{"type":"text","text":"😀"}]},
            {"type":"paragraph","content":[{"type":"mention","attrs":{"id":"12345","label":"Alice"}}, {"type":"text","text":" tail"}]},
            {"type":"paragraph"}
        ]});
        let schema = schema();
        let parsed = schema.node_from_json(&doc).unwrap();
        let wire = WireDocument::read(&parsed.to_json());
        assert_eq!(wire.text, "😀\n@12345 tail\n");
        assert_eq!(wire.to_document((3, 9)), (5, 6));
        let state = EditorState::create(EditorStateConfig::new(schema).doc(parsed)).unwrap();
        let mut tr = state.tr();
        tr.delete(5, 6).unwrap();
        let changed = state.apply(tr).unwrap();
        assert_eq!(
            WireDocument::read(&changed.doc().to_json()).text,
            "😀\n tail\n"
        );
    }

    #[test]
    fn resolving_a_mention_then_undoing_typing_preserves_other_message_content() {
        let schema = schema();
        let initial = document("😀 @12345", &HashMap::new());
        let doc = schema.node_from_json(&initial).unwrap();
        let mut state = EditorState::create(
            EditorStateConfig::new(schema)
                .doc(doc)
                .plugins(vec![history::history(Default::default())]),
        )
        .unwrap();
        let end = state.doc().content().size() - 1;
        let mut typed = state.tr();
        typed.insert_text(" tail", Some(end), Some(end));
        state = state.apply(typed).unwrap();
        let old = WireDocument::read(&state.doc().to_json());
        let desired = document(
            &old.text,
            &HashMap::from([("12345".into(), "Alice".into())]),
        );
        let mut resolved = state.tr();
        refresh_mentions(&state, &old, &desired, &mut resolved).unwrap();
        resolved.set_meta("addToHistory", json!(false));
        state = state.apply(resolved).unwrap();
        let mut undo = None;
        assert!(history::undo(&state, Some(&mut |tr| undo = Some(tr))));
        state = state.apply(undo.unwrap()).unwrap();
        assert_eq!(WireDocument::read(&state.doc().to_json()).text, "😀 @12345");
    }
}
