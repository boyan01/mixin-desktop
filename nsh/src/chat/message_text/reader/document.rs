//! Lossless display-document mapping for the read-only message selection surface.
use nsh_base::{
    rich_editor::{
        extensions::starter::{marks, nodes},
        model::{MarkSpec, Schema, SchemaSpec},
    },
    text_view::{Block, Document, Inline},
};
use serde_json::{Value, json};

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
