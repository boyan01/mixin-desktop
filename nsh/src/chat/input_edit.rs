//! Convert native UTF-16 selections to safe Rust string editing ranges.

pub(super) fn insert(text: &str, selection: (u32, u32), insertion: &str) -> (String, (u32, u32)) {
    let start = boundary(text, selection.0.min(selection.1), false);
    let end = boundary(text, selection.0.max(selection.1), true);
    let mut value = text.to_owned();
    value.replace_range(start..end, insertion);
    let caret = value[..start + insertion.len()].encode_utf16().count() as u32;
    (value, (caret, caret))
}

fn boundary(text: &str, position: u32, round_up: bool) -> usize {
    let mut units = 0_u32;
    for (byte, ch) in text.char_indices() {
        if position <= units {
            return byte;
        }
        let next = units.saturating_add(ch.len_utf16() as u32);
        if position < next {
            return if round_up { byte + ch.len_utf8() } else { byte };
        }
        units = next;
    }
    text.len()
}
