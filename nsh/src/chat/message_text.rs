mod reader;
pub(super) use reader::{Selection, read};

use nsh_base::text_view::{Block, Document, Inline};
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

pub(super) const MENTION_LINK: &str = "mixin-mention:";

/// Ordinary messages keep literal wire text; only display names, links and
/// search emphasis are presentation. URLs and email addresses are consumed
/// before numeric identities so their contents never open a user profile.
pub(super) fn document(source: &str, names: &HashMap<String, String>, query: &str) -> Document {
    static TOKENS: OnceLock<Regex> = OnceLock::new();
    let tokens = TOKENS.get_or_init(|| {
        Regex::new(concat!(
            r#"(?P<url>(?:https?://|mixin://|mailto:)[^\s<>"，。！？）】]+)"#,
            r"|(?P<email>\b[\w.%+-]+@[\w.-]+\.[A-Za-z]{2,}\b)",
            r"|(?P<bot>7000[0-9]{6})|(?P<mention>@[0-9]{4,})"
        ))
        .expect("message token pattern is valid")
    });
    let mut content = Vec::new();
    for (index, line) in source.split('\n').enumerate() {
        if index > 0 {
            content.push(Inline::Break);
        }
        let mut emitted = 0;
        for token in tokens.captures_iter(line) {
            let matched = token.get(0).expect("a token has its full match");
            append_text(&line[emitted..matched.start()], &mut content);
            let raw = matched.as_str();
            if token.name("url").is_some() {
                let url = raw.trim_end_matches(['.', ',', '!', '?', ')', ']', ';', ':']);
                if url::Url::parse(url).is_ok() {
                    append_link(url, url, &mut content);
                    append_text(&raw[url.len()..], &mut content);
                } else {
                    append_text(raw, &mut content);
                }
            } else if token.name("email").is_some() {
                append_link(raw, &format!("mailto:{raw}"), &mut content);
            } else if token.name("bot").is_some() {
                let digit_before = line[..matched.start()]
                    .bytes()
                    .next_back()
                    .is_some_and(|byte| byte.is_ascii_digit() || byte == b'@');
                let digit_after = line[matched.end()..]
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_digit());
                if digit_before || digit_after {
                    append_text(raw, &mut content);
                } else {
                    append_link(raw, &format!("{MENTION_LINK}{raw}"), &mut content);
                }
            } else if let Some(name) = names.get(&raw[1..]) {
                append_link(
                    &format!("@{name}"),
                    &format!("{MENTION_LINK}{}", &raw[1..]),
                    &mut content,
                );
            } else {
                append_text(raw, &mut content);
            }
            emitted = matched.end();
        }
        append_text(&line[emitted..], &mut content);
    }
    if !query.is_empty() {
        // Match the complete displayed text before splitting spans, so a phrase
        // containing an email address or mention keeps both emphasis and links.
        let display = Document::new(vec![Block::Paragraph {
            content: content.clone(),
        }])
        .plain_text();
        let ranges = display
            .match_indices(query)
            .map(|(start, found)| start..start + found.len())
            .collect::<Vec<_>>();
        highlight(&mut content, &ranges, &mut 0);
    }
    Document::new(vec![Block::Paragraph { content }])
}

fn append_link(display: &str, url: &str, content: &mut Vec<Inline>) {
    content.push(Inline::Link {
        url: url.to_owned(),
        title: None,
        content: vec![Inline::text(display)],
    });
}

fn append_text(source: &str, content: &mut Vec<Inline>) {
    if !source.is_empty() {
        content.push(Inline::text(source));
    }
}

fn highlight(content: &mut Vec<Inline>, ranges: &[std::ops::Range<usize>], offset: &mut usize) {
    let mut highlighted = Vec::new();
    for mut node in std::mem::take(content) {
        match &mut node {
            Inline::Text { text } => {
                let end = *offset + text.len();
                let mut emitted = 0;
                for range in ranges
                    .iter()
                    .filter(|range| range.start < end && range.end > *offset)
                {
                    let start = range.start.saturating_sub(*offset);
                    let stop = range.end.min(end) - *offset;
                    append_text(&text[emitted..start], &mut highlighted);
                    highlighted.push(Inline::Highlight {
                        content: vec![Inline::text(&text[start..stop])],
                        color: None,
                    });
                    emitted = stop;
                }
                append_text(&text[emitted..], &mut highlighted);
                *offset = end;
            }
            Inline::Link { content, .. } => {
                highlight(content, ranges, offset);
                highlighted.push(node);
            }
            Inline::Break => {
                *offset += 1;
                highlighted.push(node);
            }
            _ => highlighted.push(node),
        }
    }
    *content = highlighted;
}
