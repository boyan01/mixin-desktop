//! Live cancellation and same-message upload recovery; only the selected mixtest peers.
use super::*;
use futures::StreamExt as _;

pub(super) async fn run(
    directory: &Path,
    primary: &AccountRuntime,
    secondary: &AccountRuntime,
) -> Result<()> {
    let conversation = secondary
        .conversation_access()
        .open_user_conversation(primary.account_id().to_owned())
        .await?;
    let run = std::env::args()
        .skip_while(|arg| arg != "--cancel-run")
        .nth(1)
        .unwrap_or_default();
    if !run
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        bail!("invalid cancellation run label");
    }
    let caption = format!("NSH migration acceptance cancel/retry image {run}");
    let source = directory.join("NSH-migration-acceptance-cancel-retry.png");
    if !source.exists() {
        let mut random = 0x4d69_7869_6e4e_5348_u64;
        let pixels = image::RgbImage::from_fn(1024, 1024, |_, _| {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            image::Rgb([random as u8, (random >> 8) as u8, (random >> 16) as u8])
        });
        pixels.save(&source)?;
    }
    let expected = std::fs::read(&source)?;
    let report_path = directory.join(if run.is_empty() {
        "roundtrip-cancel-retry-results.json".into()
    } else {
        format!("roundtrip-cancel-retry-{run}.json")
    });
    let mut progress = Box::pin(secondary.attachment_progress_changes());
    let id = if report_path.exists() {
        let report: Value = serde_json::from_slice(&std::fs::read(&report_path)?)?;
        if report["conversation_id"] != conversation
            || report["caption"] != caption
            || report["primary"] != primary.account_id()
            || report["secondary"] != secondary.account_id()
        {
            bail!("existing cancellation report belongs to different selected peers");
        }
        if report["result"] == "passed" {
            bail!("cancellation check already passed; preserving its accepted message ID");
        }
        report["accepted_id"]
            .as_str()
            .ok_or_else(|| anyhow!("missing accepted cancellation ID"))?
            .to_owned()
    } else {
        let message = secondary.message_access();
        let send = message.send_attachment_receipt(
            conversation.clone(),
            source.to_string_lossy().into_owned(),
            "IMAGE".into(),
            "image/png".into(),
            Some("NSH migration acceptance cancel-retry.png".into()),
            Some(1024),
            Some(1024),
            None,
            None,
            Some(caption.clone()),
            None,
            false,
        );
        tokio::pin!(send);
        let id = tokio::time::timeout(DEADLINE, async {
            loop {
                tokio::select! {
                    biased;
                    snapshot = progress.next() => {
                        let snapshot = snapshot.ok_or_else(|| anyhow!("progress stream ended"))?;
                        if let Some(active) = snapshot.iter().find(|item| item.transcript_id.is_none()) {
                            let id = active.message_id.clone();
                            let row = message_matching(secondary, &id, |_| true).await?;
                            if row.caption.as_deref() != Some(caption.as_str()) || row.conversation_id != conversation {
                                bail!("unexpected concurrent transfer; refusing to cancel it");
                            }
                            write_transcript_report(&report_path, &json!({
                                "accepted_id": id, "conversation_id": conversation, "caption": caption,
                                "primary": primary.account_id(), "secondary": secondary.account_id(),
                                "source": source, "result": "accepted_before_cancel"
                            }))?;
                            secondary.attachment_access().cancel_attachment(id.clone()).await?;
                            let receipt = send.await?;
                            if receipt.message_id != id { bail!("cancellation changed accepted identity"); }
                            break Ok::<_, anyhow::Error>(id);
                        }
                    }
                    receipt = &mut send => {
                        let receipt = receipt?;
                        bail!("upload finished before cancellation could be observed: {}", receipt.message_id);
                    }
                }
            }
        }).await.context("initial upload cancellation deadline")??;
        let canceled =
            message_matching(secondary, &id, |row| row.media_status == "CANCELED").await?;
        if canceled.status != "SENDING" {
            bail!("canceling an upload changed delivery status");
        }
        if !primary
            .message_access()
            .message_items_by_ids(vec![id.clone()])
            .await?
            .is_empty()
        {
            bail!("canceled upload was delivered before explicit retry");
        }
        println!("PASS: active upload canceled before delivery; accepted identity and local source retained");
        id
    };
    let message = message_matching(secondary, &id, |_| true).await?;
    if message.caption.as_deref() != Some(caption.as_str()) {
        bail!("accepted cancellation fixture changed");
    }
    let mut positive_bytes = false;
    let mut progress_removed = false;
    if message.media_status == "CANCELED" {
        let access = secondary.attachment_access();
        let retry = access.resume_attachment(id.clone(), None);
        tokio::pin!(retry);
        tokio::time::timeout(DEADLINE, async {
            loop {
                tokio::select! {
                    biased;
                    snapshot = progress.next() => {
                        let snapshot = snapshot.ok_or_else(|| anyhow!("retry progress stream ended"))?;
                        positive_bytes |= snapshot.iter().any(|item| item.message_id == id && item.completed > 0 && item.total > 0);
                        progress_removed = snapshot.iter().all(|item| item.message_id != id);
                    }
                    result = &mut retry => break result,
                }
            }
        }).await.context("same-message upload retry deadline")??;
        if !positive_bytes {
            bail!("retry finished without observable real byte progress");
        }
        write_transcript_report(
            &report_path,
            &json!({
                "accepted_id": id, "conversation_id": conversation, "caption": caption,
                "primary": primary.account_id(), "secondary": secondary.account_id(),
                "source": source, "result": "retry_completed", "positive_bytes_observed": true
            }),
        )?;
        println!("PASS: same-message retry emitted real byte progress and finished upload");
    } else {
        bail!("resume requires canceled media; inspect accepted report before another run");
    }
    if !progress_removed {
        tokio::time::timeout(DEADLINE, async {
            loop {
                let snapshot = progress
                    .next()
                    .await
                    .ok_or_else(|| anyhow!("completion progress stream ended"))?;
                if snapshot.iter().all(|item| item.message_id != id) {
                    return Ok::<_, anyhow::Error>(());
                }
            }
        })
        .await
        .context("finished transfer progress removal deadline")??;
    }
    check_received_file(primary, &id, "SIGNAL_IMAGE", &expected).await?;
    let received = message_matching(primary, &id, |_| true).await?;
    if received.caption.as_deref() != Some(caption.as_str())
        || received.media_width != Some(1024)
        || received.media_height != Some(1024)
    {
        bail!("retried image changed presentation metadata");
    }
    primary
        .message_access()
        .mark_conversation_read(conversation.clone())
        .await?;
    message_matching(secondary, &id, |row| row.status == "READ").await?;
    if std::fs::read(&source)? != expected {
        bail!("retry changed original image bytes");
    }
    write_transcript_report(
        &report_path,
        &json!({
            "accepted_id": id, "conversation_id": conversation, "caption": caption,
            "primary": primary.account_id(), "secondary": secondary.account_id(), "source": source,
            "result": "passed", "checks": ["cancel_before_delivery", "same_ID_retry", "real_byte_progress",
                "completion_progress_removed", "Signal", "download_byte_identity", "caption_dimensions", "READ"]
        }),
    )?;
    println!("PASS: same-message retry emitted real byte progress, delivered identical PNG/caption/dimensions and READ; progress removed");
    Ok(())
}
