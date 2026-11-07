//! Exports existing accepted fixtures only. It never sends another message.
use super::*;

pub(super) async fn run(directory: &Path, evidence: &Path, account: &AccountRuntime) -> Result<()> {
    let evidence: Value = serde_json::from_slice(&std::fs::read(evidence)?)?;
    if std::fs::canonicalize(
        evidence["data_directory"]
            .as_str()
            .ok_or_else(|| anyhow!("missing isolated evidence directory"))?,
    )? != std::fs::canonicalize(directory)?
    {
        bail!("evidence belongs to another isolated directory");
    }
    let sources = evidence["source_files"]
        .as_array()
        .ok_or_else(|| anyhow!("missing accepted source files"))?;
    let source = |extension: &str| -> Result<Vec<u8>> {
        let path = sources
            .iter()
            .filter_map(|value| value["path"].as_str())
            .find(|path| {
                Path::new(path).extension().and_then(|value| value.to_str()) == Some(extension)
            })
            .ok_or_else(|| anyhow!("missing accepted {extension} fixture"))?;
        if !Path::new(path).starts_with(directory) {
            bail!("fixture is outside isolated acceptance directory");
        }
        Ok(std::fs::read(path)?)
    };
    let png = source("png")?;
    let ogg = source("ogg")?;
    let accepted = evidence["accounts"]
        .as_array()
        .ok_or_else(|| anyhow!("missing accepted accounts"))?
        .iter()
        .find(|row| row["identity_number"] == account.account().identity_number)
        .ok_or_else(|| anyhow!("accepted evidence does not cover restored account"))?;
    let ids: Vec<String> = accepted["messages"]
        .as_array()
        .ok_or_else(|| anyhow!("missing accepted messages"))?
        .iter()
        .filter(|row| {
            matches!(
                row["category"].as_str(),
                Some("SIGNAL_IMAGE" | "SIGNAL_AUDIO")
            )
        })
        .map(|row| {
            row["message_id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("missing accepted message ID"))
        })
        .collect::<Result<_>>()?;
    let output = tempfile::Builder::new()
        .prefix("nsh-export-check-")
        .tempdir_in(directory)?;
    let mut checked = 0;
    for row in account.message_access().message_items_by_ids(ids).await? {
        let expected = if row.category.ends_with("_IMAGE") {
            &png
        } else {
            &ogg
        };
        check(account, &row, None, expected, output.path(), &mut checked).await?;
    }
    let transcript: Value = serde_json::from_slice(&std::fs::read(
        directory.join("roundtrip-transcript-results.json"),
    )?)?;
    for field in ["combined_id", "forwarded_id"] {
        let root = transcript[field]
            .as_str()
            .ok_or_else(|| anyhow!("missing accepted transcript root"))?;
        for row in account
            .message_access()
            .transcript_messages(root.to_owned())
            .await?
        {
            let expected = if row.category.ends_with("_IMAGE") {
                &png
            } else if row.category.ends_with("_AUDIO") {
                &ogg
            } else {
                continue;
            };
            check(
                account,
                &row,
                Some(root),
                expected,
                output.path(),
                &mut checked,
            )
            .await?;
        }
    }
    if checked < 6 {
        bail!("expected original and both transcript fixture exports");
    }
    println!("PASS: {checked} persisted ordinary/transcript attachments exported byte-identically; PNG clipboard payload pixels match; temporary exports removed");
    Ok(())
}

async fn check(
    account: &AccountRuntime,
    row: &MessageListView,
    root: Option<&str>,
    expected: &[u8],
    output: &Path,
    count: &mut usize,
) -> Result<()> {
    let destination = output.join(format!(
        "{}-{}.bin",
        root.unwrap_or("ordinary"),
        row.message_id
    ));
    let access = account.attachment_access();
    if let Some(root) = root {
        access
            .save_transcript_attachment_as(
                root.to_owned(),
                row.message_id.clone(),
                destination.to_string_lossy().into_owned(),
            )
            .await?;
    } else {
        access
            .save_attachment_as(
                row.message_id.clone(),
                destination.to_string_lossy().into_owned(),
            )
            .await?;
    }
    if std::fs::read(destination)? != expected {
        bail!("export changed the accepted attachment bytes");
    }
    if row.category.ends_with("_IMAGE") {
        let uri = access
            .attachment_image_data_uri(row.message_id.clone(), root.map(str::to_owned))
            .await?;
        let encoded = uri
            .strip_prefix("data:image/png;base64,")
            .ok_or_else(|| anyhow!("image export is not a PNG clipboard payload"))?;
        let actual = image::load_from_memory(&Base64::decode_vec(encoded)?)?.to_rgba8();
        let original = image::load_from_memory(expected)?.to_rgba8();
        if actual != original {
            bail!("PNG clipboard payload changed original pixels");
        }
    }
    *count += 1;
    Ok(())
}
