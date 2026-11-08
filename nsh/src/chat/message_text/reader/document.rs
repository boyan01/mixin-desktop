//! Lossless display-document mapping for the read-only message selection surface.
use nsh_base::{
    rich_editor::{
        extensions::starter::{marks, nodes},
        model::{MarkSpec, Schema, SchemaSpec},
    },
    text_view::{Block, Document, Inline},
};
use nsh_core::prelude::*;
use serde_json::{Value, json};

pub(super) const LINK_CLASS: &str = "text-primary underline";
pub(super) const HIGHLIGHT_CLASS: &str = "bg-highlight text-foreground";

/// The editor's paragraph spans for `value`, without the editor: its text
/// nodes with their marks' classes, then the status room's widget.
pub(super) fn spans(value: &Value, room: Option<&str>) -> Vec<AnyElement> {
    let nodes = value["content"][0]["content"].as_array();
    let mut spans: Vec<AnyElement> = nodes
        .into_iter()
        .flatten()
        .filter_map(|node| {
            let text = node["text"].as_str()?.to_owned();
            let classes = node["marks"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|mark| match mark["type"].as_str()? {
                    "link" => Some(LINK_CLASS),
                    "highlight" => Some(HIGHLIGHT_CLASS),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let span = nsh_core::prelude::text(text);
            Some(if classes.is_empty() {
                span.into_element()
            } else {
                span.app_class(SharedString::from(classes.join(" ")))
                    .into_element()
            })
        })
        .collect();
    if spans.is_empty() {
        // An empty paragraph keeps a line, as the editor's filler does.
        spans.push(nsh_core::prelude::text("\u{200B}").into_element());
    }
    spans.extend(room.map(|class| {
        view()
            .app_class(SharedString::from(class.to_owned()))
            .into_element()
    }));
    spans
}

pub(super) fn schema() -> Schema {
    let mut spec = SchemaSpec::default();
    spec.nodes = vec![nodes::doc(), nodes::paragraph(), nodes::text()];
    spec.marks = vec![marks::link(), ("highlight".into(), MarkSpec::default())];
    Schema::new(spec).expect("Read-only message schema is valid")
}

pub(super) fn value(document: &Document) -> Value {
    let mut content = Vec::new();
    for (index, block) in document.blocks.iter().enumerate() {
        if index != 0 {
            append("\n\n", &[], &mut content);
        }
        match block {
            Block::Paragraph { content: inlines } => inline(inlines, &[], &mut content),
            // The message mapper currently emits only one literal paragraph.
            // Preserve text if a future mapper adds another block kind.
            block => append(
                &Document::new(vec![block.clone()]).plain_text(),
                &[],
                &mut content,
            ),
        }
    }
    json!({"type":"doc", "content":[{"type":"paragraph", "content":content}]})
}

fn inline(inlines: &[Inline], marks: &[Value], out: &mut Vec<Value>) {
    for node in inlines {
        match node {
            Inline::Text { text } => append(text, marks, out),
            Inline::Break => append("\n", marks, out),
            Inline::Link {
                url,
                title,
                content,
            } => {
                let mut nested = marks.to_vec();
                nested.push(json!({"type":"link", "attrs":{"href":url,"title":title}}));
                inline(content, &nested, out);
            }
            Inline::Highlight { content, .. } => {
                let mut nested = marks.to_vec();
                nested.push(json!({"type":"highlight"}));
                inline(content, &nested, out);
            }
            node => append(
                &Document::new(vec![Block::Paragraph {
                    content: vec![node.clone()],
                }])
                .plain_text(),
                marks,
                out,
            ),
        }
    }
}

fn append(text: &str, marks: &[Value], out: &mut Vec<Value>) {
    if text.is_empty() {
        return;
    }
    let mut node = json!({"type":"text", "text":text});
    if !marks.is_empty() {
        node["marks"] = Value::Array(marks.to_vec());
    }
    out.push(node);
}
