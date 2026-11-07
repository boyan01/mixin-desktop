//! Bounded Markdown reads selected by persisted account message identity.
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use sdk::message_category::MessageCategory as _;
use tokio::io::AsyncReadExt as _;

use crate::db::mixin::MixinDatabase;

use super::{export, AttachmentAccess};

const MAX_MARKDOWN_BYTES: usize = 2 * 1024 * 1024;

impl AttachmentAccess {
    /// Reads a downloaded Markdown attachment; the caller never supplies a path.
    pub async fn markdown_content(
        &self,
        message_id: String,
        transcript_id: Option<String>,
    ) -> Result<String> {
        self.ensure_active()?;
        let directory =
            crate::db::path::account_data_directory(&self.profile.borrow().identity_number)?;
        let mut shutdown = self.state.shutdown.clone();
        tokio::select! {
            biased;
            _ = shutdown.wait_for(|stopping| *stopping) => {
                Err(anyhow!("account runtime is shutting down"))
            }
            result = load_content(&self.database, &directory, &message_id, transcript_id.as_deref()) => result,
        }
    }
}

async fn load_content(
    database: &MixinDatabase,
    directory: &Path,
    message_id: &str,
    transcript_id: Option<&str>,
) -> Result<String> {
    let (category, name) = if let Some(root) = transcript_id {
        let child = database
            .transcript_message_dao
            .find(root, message_id)
            .await?
            .ok_or_else(|| anyhow!("transcript attachment not found"))?;
        (child.category, child.media_name)
    } else {
        let message = database
            .message_dao
            .find_message_by_id(&message_id.to_owned())
            .await?
            .ok_or_else(|| anyhow!("attachment not found"))?;
        (message.category, message.name)
    };
    if !category.is_data()
        || name
            .as_deref()
            .and_then(|name| Path::new(name).extension())
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("md"))
    {
        bail!("attachment is not a Markdown file");
    }
    // The shared resolver checks current download status, account containment,
    // transcript membership, scoped paths, and symlinks before opening anything.
    let (source, _) = export::source(database, directory, message_id, transcript_id).await?;
    let file = crate::fs::File::open(source).await?;
    let mut bytes = Vec::new();
    file.take(MAX_MARKDOWN_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > MAX_MARKDOWN_BYTES {
        bail!("Markdown file exceeds the 2 MiB reader limit");
    }
    let content = String::from_utf8(bytes).context("Markdown file is not valid UTF-8")?;
    // Dart's default UTF-8 decoder consumes an optional byte order mark.
    Ok(content
        .strip_prefix('\u{feff}')
        .unwrap_or(&content)
        .to_owned())
}

#[cfg(test)]
#[path = "attachment_text_tests.rs"]
mod tests;
