//! Native list updates that preserve the measured visible message.

use mixin_desktop_api::MessageListView;

pub(crate) fn splice_phase(
    previous: &[MessageListView],
    next: &[MessageListView],
    anchor: Option<&str>,
) -> Option<Vec<MessageListView>> {
    splice_by(previous, next, anchor, |row| row.message_id.as_str())
}

pub(crate) fn splice_by<T: Clone>(
    previous: &[T],
    next: &[T],
    anchor: Option<&str>,
    key: impl Fn(&T) -> &str,
) -> Option<Vec<T>> {
    let anchor = anchor?;
    let old_at = previous.iter().position(|row| key(row) == anchor)?;
    let new_at = next.iter().position(|row| key(row) == anchor)?;
    let prefix = previous
        .iter()
        .zip(next)
        .take_while(|(old, new)| key(old) == key(new))
        .count();
    let suffix = previous[prefix..]
        .iter()
        .rev()
        .zip(next[prefix..].iter().rev())
        .take_while(|(old, new)| key(old) == key(new))
        .count();
    if old_at < prefix || old_at >= previous.len() - suffix {
        return None;
    }
    // Change the smaller side first. Its intermediate length cannot exceed
    // max(previous.len(), next.len()), so the consumer's window bound holds throughout.
    let prefix_first = new_at + previous.len() - old_at;
    let suffix_first = old_at + next.len() - new_at;
    Some(if prefix_first <= suffix_first {
        next[..new_at]
            .iter()
            .chain(&previous[old_at..])
            .cloned()
            .collect()
    } else {
        previous[..=old_at]
            .iter()
            .chain(&next[new_at + 1..])
            .cloned()
            .collect()
    })
}
