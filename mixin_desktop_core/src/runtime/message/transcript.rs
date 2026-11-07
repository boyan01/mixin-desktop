use anyhow::{anyhow, ensure, Result};
use serde::Deserialize;

use super::{model, MessageAccess};

impl MessageAccess {
    /// Loads playable audio neighbors from the same persisted transcript root.
    pub async fn transcript_audio_messages_around(
        &self,
        transcript_id: String,
        anchor_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .transcript_message_dao
            .audio_items_around(&transcript_id, &anchor_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.transcript_message_list_view(item))
            .collect()
    }

    /// Reads the summary actually retained by a nested forwarded message.
    pub async fn transcript_summary(
        &self,
        transcript_id: String,
        child_message_id: String,
    ) -> Result<Vec<model::TranscriptSummaryItem>> {
        self.ensure_active()?;
        let content = self
            .database
            .transcript_message_dao
            .summary_content(&transcript_id, &child_message_id)
            .await?
            .ok_or_else(|| anyhow!("nested transcript summary is unavailable"))?;
        parse_summary(&content)
    }

    /// Loads a bounded image window within one forwarded transcript.
    pub async fn transcript_image_messages_around(
        &self,
        transcript_id: String,
        anchor_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .transcript_message_dao
            .image_items_around(&transcript_id, &anchor_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.transcript_message_list_view(item))
            .collect()
    }
}

// These are reader resource bounds, not protocol limits. Flutter's nested wire
// retains summary strings only; never recursively invent full message records.
const MAX_SUMMARY_BYTES: usize = 1024 * 1024;
const MAX_SUMMARY_ITEMS: usize = 200;

fn parse_summary(content: &str) -> Result<Vec<model::TranscriptSummaryItem>> {
    ensure!(
        content.len() <= MAX_SUMMARY_BYTES,
        "transcript summary is too large"
    );
    #[derive(Deserialize)]
    struct Summary {
        name: String,
        category: String,
        content: Option<String>,
    }
    let summaries: Vec<Summary> = serde_json::from_str(content)?;
    ensure!(!summaries.is_empty(), "transcript summary is empty");
    ensure!(
        summaries.len() <= MAX_SUMMARY_ITEMS,
        "transcript summary has too many entries"
    );
    Ok(summaries
        .into_iter()
        .map(|summary| model::TranscriptSummaryItem {
            sender_name: summary.name,
            category: summary.category,
            content: summary.content.unwrap_or_default(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flutter_nested_summary_preserves_sender_and_content_without_inventing_original_messages() {
        let items = parse_summary(r#"[{"name":"Alice","category":"SIGNAL_TEXT","content":"Hello"},{"name":"Bob","category":"SIGNAL_IMAGE","content":null}]"#).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].sender_name, "Alice");
        assert_eq!(items[0].category, "SIGNAL_TEXT");
        assert_eq!(items[0].content, "Hello");
        assert_eq!(items[1].sender_name, "Bob");
        assert!(items[1].content.is_empty());
    }

    #[test]
    fn empty_malformed_and_oversized_summaries_are_unavailable() {
        for content in ["[]", "{}", r#"[{"category":"SIGNAL_TEXT"}]"#] {
            assert!(parse_summary(content).is_err());
        }
        assert!(parse_summary(&" ".repeat(MAX_SUMMARY_BYTES + 1)).is_err());
        let row = serde_json::json!({"name":"Alice", "category":"SIGNAL_TEXT", "content":"hello"});
        assert!(
            parse_summary(&serde_json::to_string(&vec![row; MAX_SUMMARY_ITEMS + 1]).unwrap())
                .is_err()
        );
    }
}
