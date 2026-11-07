//! Portable transfer attachment identities and account-contained local sources.
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use sdk::message_category::MessageCategory as _;
use serde_json::{Map, Value};
use tokio::io::AsyncReadExt as _;
use uuid::Uuid;

use super::{message_from_record, DeviceTransferService};
use crate::core::attachment::{attachment_path, transcript_attachment_path};

impl DeviceTransferService {
    pub(super) fn normalize_attachment_url(
        &self,
        kind: &str,
        record: &mut Map<String, Value>,
    ) -> Result<()> {
        let message = message_from_record(record)?;
        let completed = record
            .get("media_status")
            .and_then(Value::as_str)
            .is_some_and(|status| matches!(status, "DONE" | "READ"));
        // Pending ordinary sends can hold a remote/upload source URL. Preserve
        // those commands rather than rewriting an unsent attachment's source.
        if message.category.is_attachment() && (kind == "transcript_message" || completed) {
            let path = if kind == "transcript_message" {
                transcript_attachment_path(&self.account_data_dir, &message)?
            } else {
                attachment_path(&self.account_data_dir, &message)?
            };
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("transcript attachment has no file name"))?;
            // The wire identifies files by message UUID, never a sender-local
            // absolute path. A receiver stores the same portable leaf name.
            record.insert("media_url".into(), Value::String(file_name.into()));
        }
        Ok(())
    }

    pub(super) async fn insert_attachment(&self, message_id: &str, source: &Path) -> Result<()> {
        Uuid::parse_str(message_id).context("invalid transferred attachment identity")?;
        let mut paths = Vec::with_capacity(2);
        if let Some(record) = self
            .record_by_message_id("transcript_messages", message_id)
            .await?
        {
            let message = message_from_record(&record)?;
            paths.push(transcript_attachment_path(
                &self.account_data_dir,
                &message,
            )?);
        }
        if let Some(record) = self.record_by_message_id("messages", message_id).await? {
            let message = message_from_record(&record)?;
            Uuid::parse_str(&message.conversation_id)
                .context("invalid transferred conversation identity")?;
            paths.push(attachment_path(&self.account_data_dir, &message)?);
        }
        for path in paths {
            publish_attachment(&self.account_data_dir, source, &path).await?;
        }
        Ok(())
    }

    pub(super) async fn attachment_for_record(
        &self,
        kind: &str,
        record: &Map<String, Value>,
    ) -> Result<Option<PathBuf>> {
        let message = message_from_record(record)?;
        if !message.category.is_attachment() {
            return Ok(None);
        }
        let media_status = record
            .get("media_status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let transferable = if kind == "transcript_message" {
            media_status == "DONE"
        } else {
            matches!(media_status, "DONE" | "READ")
        };
        if !transferable {
            return Ok(None);
        }
        let path = if kind == "transcript_message" {
            let root = record
                .get("transcript_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("transcript attachment has no root"))?;
            Uuid::parse_str(root).context("invalid transcript attachment root")?;
            let default = transcript_attachment_path(&self.account_data_dir, &message)?;
            let scoped = default
                .parent()
                .ok_or_else(|| anyhow!("transcript attachment has no parent"))?
                .join(root)
                .join(default.file_name().unwrap());
            // The serialized URL is portable. Consult the persisted source row
            // to find a forwarded copy owned by this transcript root.
            let source = self
                .database
                .transcript_message_dao
                .find(root, &message.message_id)
                .await?;
            match source
                .as_ref()
                .and_then(|row| row.media_url.as_deref())
                .map(Path::new)
                .filter(|path| path.is_absolute())
            {
                Some(path) if path == default || path == scoped => path.to_path_buf(),
                Some(_) => bail!("transcript attachment source is outside its root"),
                None => default,
            }
        } else {
            attachment_path(&self.account_data_dir, &message)?
        };
        if !tokio::fs::metadata(&path)
            .await
            .ok()
            .is_some_and(|metadata| metadata.is_file() && metadata.len() > 0)
        {
            return Ok(None);
        }
        let account = crate::fs::canonicalize(&self.account_data_dir).await?;
        let path = crate::fs::canonicalize(path).await?;
        if !path.starts_with(account) {
            bail!("transfer attachment source is outside the account");
        }
        Ok(Some(path))
    }
}

/// The v3 file packet has only a message UUID, shared by ordinary and transcript
/// rows. A conflicting local copy cannot be represented by that wire identity.
pub(super) async fn same_bytes(first: &Path, second: &Path) -> Result<bool> {
    if first == second {
        return Ok(true);
    }
    let mut first = tokio::fs::File::open(first).await?;
    let mut second = tokio::fs::File::open(second).await?;
    let length = first.metadata().await?.len();
    if length != second.metadata().await?.len() {
        return Ok(false);
    }
    let mut remaining = length;
    let mut a = vec![0_u8; 64 * 1024];
    let mut b = vec![0_u8; 64 * 1024];
    while remaining > 0 {
        let count = remaining.min(a.len() as u64) as usize;
        first.read_exact(&mut a[..count]).await?;
        second.read_exact(&mut b[..count]).await?;
        if a[..count] != b[..count] {
            return Ok(false);
        }
        remaining -= count as u64;
    }
    Ok(true)
}

/// Publish an authenticated file without overwriting an existing attachment.
/// A same-directory hard link supplies atomic create-if-absent on native filesystems.
async fn publish_attachment(account: &Path, source: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("attachment has no parent"))?;
    let relative = parent
        .strip_prefix(account)
        .context("attachment destination is outside the account")?;
    if !relative.starts_with("Media")
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        bail!("invalid attachment destination components");
    }
    let canonical_account = crate::fs::canonicalize(account).await?;
    let media_root = canonical_account.join("Media");
    let mut canonical_parent = canonical_account;
    // Inspect each existing ancestor before creating its next child. A symlink
    // cannot redirect create_dir_all into another account or an external tree.
    for component in relative.components() {
        canonical_parent.push(component);
        match tokio::fs::create_dir(&canonical_parent).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        canonical_parent = crate::fs::canonicalize(&canonical_parent).await?;
        if !canonical_parent.starts_with(&media_root) {
            bail!("attachment destination is outside account media");
        }
    }
    match tokio::fs::symlink_metadata(destination).await {
        Ok(metadata) => {
            if !metadata.file_type().is_file() || !same_bytes(source, destination).await? {
                bail!("existing attachment conflicts with transferred bytes");
            }
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let (staged, mut output) = super::protocol::StagedFile::create(&canonical_parent)?;
    let mut input = tokio::fs::File::open(source).await?;
    tokio::io::copy(&mut input, &mut output).await?;
    tokio::io::AsyncWriteExt::flush(&mut output).await?;
    drop(output);
    match tokio::fs::hard_link(staged.path(), destination).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = tokio::fs::symlink_metadata(destination).await?;
            if metadata.file_type().is_file() && same_bytes(source, destination).await? {
                Ok(())
            } else {
                bail!("existing attachment conflicts with transferred bytes")
            }
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn authenticated_attachment_reuses_identical_existing_file_and_rejects_conflicts() {
        let account = tempfile::tempdir().unwrap();
        let source = account.path().join("source");
        let target = account.path().join("Media/Images/conversation/image.png");
        std::fs::write(&source, b"verified original").unwrap();
        publish_attachment(account.path(), &source, &target)
            .await
            .unwrap();
        publish_attachment(account.path(), &source, &target)
            .await
            .unwrap();
        std::fs::write(&source, b"conflicting copy").unwrap();
        assert!(publish_attachment(account.path(), &source, &target)
            .await
            .is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"verified original");
        assert_eq!(
            std::fs::read_dir(target.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn media_directory_symlink_is_rejected_before_creating_external_children() {
        let fixture = tempfile::tempdir().unwrap();
        let account = fixture.path().join("account");
        let external = fixture.path().join("external");
        std::fs::create_dir(&account).unwrap();
        std::fs::create_dir(&external).unwrap();
        std::os::unix::fs::symlink(&external, account.join("Media")).unwrap();
        let source = fixture.path().join("source");
        std::fs::write(&source, b"verified original").unwrap();
        assert!(publish_attachment(
            &account,
            &source,
            &account.join("Media/Images/conversation/image.png")
        )
        .await
        .is_err());
        assert_eq!(std::fs::read_dir(external).unwrap().count(), 0);
    }
}
