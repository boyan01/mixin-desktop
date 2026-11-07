//! Maps wire text to the editor document. Wire text stays the only draft value:
//! mention nodes are its display form and always export their original token.
use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};

use nsh_base::rich_editor::{
    extensions::{starter::nodes, suggestion::mention_node},
    model::{Schema, SchemaSpec},
    state::{EditorState, Transaction},
};
use regex::Regex;
use serde_json::{Value, json};

fn mention_regex() -> &'static Regex {
    static MENTION: OnceLock<Regex> = OnceLock::new();
    MENTION.get_or_init(|| Regex::new(r"@([0-9]{4,})").expect("Mention regex is valid"))
}

/// The mention tokens of `raw`, canonical so ordinary typing leaves them equal.
pub(super) fn mention_tokens(raw: &str) -> String {
    let mut tokens = mention_regex()
        .find_iter(raw)
        .map(|token| token.as_str())
        .collect::<Vec<_>>();
    tokens.sort_unstable();
    tokens.dedup();
    tokens.join(" ")
}

pub(super) fn schema() -> Schema {
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

pub(super) fn document(raw: &str, names: &HashMap<String, String>) -> Value {
    let mut content = Vec::new();
    let mut from = 0;
    for found in mention_regex().captures_iter(raw) {
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

pub(super) fn mentions_differ(a: &Value, b: &Value) -> bool {
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

pub(super) fn refresh_mentions(
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
pub(super) struct WireDocument {
    pub(super) text: String,
    boundaries: Vec<(u32, usize)>,
    mentions: Vec<(u32, u32, Value)>,
}
impl WireDocument {
    pub(super) fn read(doc: &Value) -> Self {
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
    pub(super) fn to_document(&self, (a, h): (u32, u32)) -> (usize, usize) {
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
    pub(super) fn to_wire(&self, (a, h): (usize, usize)) -> (u32, u32) {
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
    use nsh_base::rich_editor::{
        extensions::history,
        state::{EditorStateConfig, TextSelection},
    };

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
