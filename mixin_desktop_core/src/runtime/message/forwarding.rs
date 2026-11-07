//! Account-owned individual and combined message forwarding.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use log::warn;
use sdk::{
    message_category::MessageCategory as _, AttachmentMessage, ContactMessage, LiveMessage,
    MessageStatus, StickerMessage,
};
use uuid::Uuid;

use crate::core::{
    attachment::{attachment_file_name, attachment_path, transcript_attachment_path},
    model::{job::sanitize_transcript_app_card, AttachmentExtra},
};
use crate::db::{
    mixin::{
        job::Job,
        message::{MediaStatus, Message},
        transcript_message::TranscriptMessage,
    },
    path::account_data_directory,
};
use crate::runtime::{
    combine_transcript_category, forward_category, forward_transcript_category,
    validate_combine_forward_source, AttachmentAccess,
};

use super::{validate_forward_source, MessageAccess};

struct ForwardPlan {
    message: Message,
    job: Job,
    transcripts: Option<Vec<TranscriptMessage>>,
}

impl MessageAccess {
    pub async fn forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        let target_conversation_id = target_conversation_id.as_str();
        let source_message_ids = source_message_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.ensure_can_send(target_conversation_id).await?;
        let account_dir = account_data_directory(&self.profile.borrow().identity_number)?;
        if !(1..100).contains(&source_message_ids.len()) {
            return Err(anyhow!("forward requires 1 to 99 messages"));
        }
        if source_message_ids.iter().collect::<HashSet<_>>().len() != source_message_ids.len() {
            return Err(anyhow!("forward message ids contain duplicates"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(target_conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {target_conversation_id}"))?;
        let target_text_category = self.text_category(conversation.owner_id.as_deref()).await?;
        let target_prefix = target_text_category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid target message category: {target_text_category}"))?;

        let sources = self.messages_by_ids(source_message_ids).await?;
        for (source, source_message_id) in sources.iter().zip(source_message_ids) {
            validate_forward_source(source, target_prefix)
                .with_context(|| format!("message cannot be forwarded: {source_message_id}"))?;
        }

        let mut copied_paths = Vec::new();
        let prepared: Result<Vec<ForwardPlan>> = async {
            let mut plans = Vec::with_capacity(sources.len());
            for source in sources {
                let message_id = Uuid::new_v4().to_string();
                let category = forward_category(&source.category, target_prefix)?;
                let mut content = source.content.clone().unwrap_or_default();
                let mut media_size = None;
                let mut media_status = crate::db::mixin::message::MediaStatus::Done;
                let mut media_url = None;
                let mut media_mime_type = None;
                let mut media_duration = String::new();
                let mut media_width = None;
                let mut media_height = None;
                let mut thumb_image = None;
                let mut thumb_url = None;
                let mut media_key = None;
                let mut media_digest = None;
                let mut media_waveform = None;
                let mut caption = None;
                let mut forwarded_transcripts = None;

                if source.category.is_attachment() {
                    let extra = serde_json::from_str::<AttachmentExtra>(
                        source.content.as_deref().unwrap_or_default(),
                    )?;
                    let stored_path = source.media_url.as_deref().unwrap_or_default();
                    let source_path = if Path::new(stored_path).is_absolute() {
                        Path::new(stored_path).to_path_buf()
                    } else {
                        attachment_path(&account_dir, &source)?
                    };
                    let target_file_message = Message {
                        message_id: message_id.clone(),
                        conversation_id: target_conversation_id.to_string(),
                        category: category.clone(),
                        media_mime_type: source.media_mime_type.clone(),
                        name: source.name.clone(),
                        ..Message::default()
                    };
                    let (target_path, actual_size) = self
                        .app_service
                        .attachment
                        .copy_for_forward(&source_path, &target_file_message)
                        .await?;
                    copied_paths.push(target_path.clone());

                    let same_prefix = source.category.starts_with(&format!("{target_prefix}_"));
                    let reusable_age = extra.created_at.and_then(|created_at| {
                        let age = Utc::now().signed_duration_since(created_at);
                        (age >= chrono::Duration::zero() && age <= chrono::Duration::hours(24))
                            .then_some(age)
                    });
                    let reusable_material = target_prefix == "PLAIN"
                        || (source.media_key.is_some() && source.media_digest.is_some());
                    let reusable = same_prefix && reusable_age.is_some() && reusable_material;
                    let (attachment_id, attachment_created_at, key, digest) = if reusable {
                        (
                            extra.attachment_id,
                            extra.created_at,
                            source.media_key.clone(),
                            source.media_digest.clone(),
                        )
                    } else {
                        media_status = MediaStatus::Canceled;
                        (String::new(), None, None, None)
                    };
                    let attachment = AttachmentMessage {
                        key: key.clone(),
                        digest: digest.clone(),
                        attachment_id,
                        mime_type: source
                            .media_mime_type
                            .clone()
                            .unwrap_or_else(|| "application/octet-stream".to_string()),
                        size: actual_size,
                        name: source.name.clone(),
                        width: source.media_width,
                        height: source.media_height,
                        thumbnail: source.thumb_image.clone(),
                        duration: source.media_duration.parse().ok(),
                        waveform: source
                            .media_waveform
                            .as_deref()
                            .map(Base64::decode_vec)
                            .transpose()?,
                        caption: source.caption.clone(),
                        created_at: attachment_created_at,
                        shareable: Some(true),
                    };
                    content = Base64::encode_string(serde_json::to_string(&attachment)?.as_bytes());
                    media_url = Some(attachment_file_name(&target_path)?.to_string());
                    media_mime_type = source.media_mime_type.clone();
                    media_size = Some(actual_size);
                    media_duration = source.media_duration.clone();
                    media_width = source.media_width;
                    media_height = source.media_height;
                    thumb_image = source.thumb_image.clone();
                    media_key = key;
                    media_digest = digest;
                    media_waveform = source.media_waveform.clone();
                    caption = source.caption.clone();
                } else if source.category.is_live() {
                    let live = LiveMessage {
                        width: source
                            .media_width
                            .ok_or_else(|| anyhow!("forward live message has no width"))?,
                        height: source
                            .media_height
                            .ok_or_else(|| anyhow!("forward live message has no height"))?,
                        thumb_url: source.thumb_url.clone().unwrap_or_default(),
                        url: source
                            .media_url
                            .clone()
                            .filter(|url| !url.is_empty())
                            .ok_or_else(|| anyhow!("forward live message has no URL"))?,
                        shareable: true,
                    };
                    let live_content = serde_json::to_string(&live)?;
                    content = live_content;
                    media_url = Some(live.url);
                    thumb_url = Some(live.thumb_url);
                    media_width = Some(live.width);
                    media_height = Some(live.height);
                } else if source.category.is_sticker() {
                    let sticker = StickerMessage {
                        sticker_id: source.sticker_id.clone().ok_or_else(|| {
                            anyhow!("forward sticker has no sticker id: {}", source.message_id)
                        })?,
                        album_id: None,
                        name: None,
                    };
                    content = Base64::encode_string(serde_json::to_string(&sticker)?.as_bytes());
                } else if source.category.is_contact() {
                    let contact = ContactMessage {
                        user_id: source.shared_user_id.clone().ok_or_else(|| {
                            anyhow!(
                                "forward contact has no shared user id: {}",
                                source.message_id
                            )
                        })?,
                    };
                    content = Base64::encode_string(serde_json::to_string(&contact)?.as_bytes());
                } else if source.category.is_transcript() {
                    let transcripts = self
                        .database
                        .transcript_message_dao
                        .find_by_transcript_id(&source.message_id)
                        .await?;
                    if transcripts.is_empty() {
                        return Err(anyhow!("transcript is empty: {}", source.message_id));
                    }
                    let mut transcript_entries =
                        prepare_forwarded_transcripts(transcripts, &message_id, target_prefix)?;
                    copied_paths.extend(
                        copy_transcript_attachments(&mut transcript_entries, &account_dir, &message_id)
                            .await?,
                    );
                    let summaries = transcript_entries.iter().map(|transcript| {
                        Ok(serde_json::json!({
                            "name": transcript.user_full_name.as_deref().unwrap_or_default(),
                            "category": forward_transcript_category(&transcript.category, target_prefix)?,
                            "content": transcript.content,
                        }))
                    }).collect::<Result<Vec<_>>>()?;
                    content = serde_json::to_string(&summaries)?;
                    media_size = Some(0);
                    media_status = if transcript_entries
                        .iter()
                        .any(|entry| entry.category.is_attachment())
                    {
                        MediaStatus::Canceled
                    } else {
                        MediaStatus::Done
                    };
                    forwarded_transcripts = Some(transcript_entries);
                } else if source.content.is_none() {
                    return Err(anyhow!(
                        "forward message has no content: {}",
                        source.message_id
                    ));
                }

                let message = Message {
                    message_id: message_id.clone(),
                    conversation_id: target_conversation_id.to_string(),
                    user_id: self.account_id.clone(),
                    category,
                    content: Some(content.clone()),
                    media_url,
                    media_mime_type,
                    media_size,
                    media_duration,
                    media_width,
                    media_height,
                    thumb_image,
                    media_key,
                    media_digest,
                    media_status,
                    name: source.name,
                    album_id: source.album_id,
                    sticker_id: source.sticker_id,
                    shared_user_id: source.shared_user_id,
                    media_waveform,
                    thumb_url,
                    caption,
                    status: MessageStatus::Sending,
                    created_at: Utc::now().naive_utc(),
                    ..Message::default()
                };
                let job = Job::create_sending_job(
                    &message_id,
                    target_conversation_id,
                    None,
                    None,
                    false,
                    false,
                    conversation.expire_in,
                );
                plans.push(ForwardPlan {
                    message,
                    job,
                    transcripts: forwarded_transcripts,
                });
            }
            Ok(plans)
        }
        .await;
        let plans = accept_forward_batch(&self.database, prepared, &copied_paths).await?;
        // Every requested message is now accepted. Failures below must retain these IDs and files.
        self.notify_conversation_changed(target_conversation_id);
        let attachments = AttachmentAccess::new(self.state.clone());
        let mut forwarded_ids = Vec::with_capacity(plans.len());
        for plan in plans {
            let message = plan.message;
            let message_id = message.message_id;
            if message.media_status != MediaStatus::Done {
                let upload = if message.category.is_transcript() {
                    attachments
                        .retry_transcript_attachment_locked(&message_id)
                        .await
                } else {
                    attachments.retry_attachment_locked(&message_id).await
                };
                if let Err(error) = upload {
                    warn!(
                        "accepted forwarded message {message_id} needs attachment retry: {error}"
                    );
                }
            }
            if message.category.is_text()
                || message.category.is_post()
                || message.category.is_transcript()
            {
                if let Err(error) = self
                    .database
                    .message_fts_dao
                    .upsert(
                        &message_id,
                        target_conversation_id,
                        message.content.as_deref().unwrap_or_default(),
                    )
                    .await
                {
                    warn!("failed to index forwarded message {message_id}: {error}");
                }
            }
            forwarded_ids.push(message_id);
        }
        if let Err(error) = self.app_service.job.wake(sdk::SENDING_MESSAGE) {
            warn!("accepted forwarded messages need delivery retry: {error}");
        }
        self.notify_conversation_changed(target_conversation_id);
        Ok(forwarded_ids)
    }

    pub async fn combine_forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<String> {
        let target_conversation_id = target_conversation_id.as_str();
        let source_message_ids = source_message_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.ensure_can_send(target_conversation_id).await?;
        let account_dir = account_data_directory(&self.profile.borrow().identity_number)?;
        if !(2..100).contains(&source_message_ids.len()) {
            return Err(anyhow!("combine forward requires 2 to 99 messages"));
        }
        let unique_ids = source_message_ids.iter().collect::<HashSet<_>>();
        if unique_ids.len() != source_message_ids.len() {
            return Err(anyhow!("combine forward message ids contain duplicates"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(target_conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {target_conversation_id}"))?;
        let target_text_category = self.text_category(conversation.owner_id.as_deref()).await?;
        let target_prefix = target_text_category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid target message category: {target_text_category}"))?;

        let mut sources = self.messages_by_ids(source_message_ids).await?;
        for source in &sources {
            validate_combine_forward_source(source)?;
        }
        sources.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.message_id.cmp(&right.message_id))
        });
        let user_ids = sources
            .iter()
            .flat_map(|source| {
                std::iter::once(source.user_id.clone()).chain(source.shared_user_id.iter().cloned())
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let user_names = self
            .database
            .user_dao
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|user| (user.user_id, user.full_name))
            .collect::<HashMap<_, _>>();

        let transcript_id = Uuid::new_v4().to_string();
        let mut transcripts = Vec::with_capacity(sources.len());
        let mut summaries = Vec::with_capacity(sources.len());
        let mut fts_parts = Vec::new();
        let mut has_attachments = false;
        for source in sources {
            let sender_name = user_names.get(&source.user_id).cloned().unwrap_or_default();
            let category = combine_transcript_category(&source.category, target_prefix)?;
            let is_attachment = source.category.is_attachment();
            has_attachments |= is_attachment;
            let media_created_at = source
                .content
                .as_deref()
                .and_then(|content| serde_json::from_str::<AttachmentExtra>(content).ok())
                .and_then(|extra| extra.created_at);
            // Attachment files still belong to the source conversation until upload completes.
            let media_url = if is_attachment {
                Some(
                    forward_source_attachment_path(&source, &account_dir)?
                        .to_string_lossy()
                        .into_owned(),
                )
            } else {
                source.media_url.clone()
            };
            let mut transcript = TranscriptMessage {
                transcript_id: transcript_id.clone(),
                message_id: source.message_id,
                user_id: Some(source.user_id),
                user_full_name: Some(sender_name.clone()),
                // Keep the source encryption category until the uploader replaces its credentials.
                category: if is_attachment {
                    source.category.clone()
                } else {
                    category.clone()
                },
                created_at: source.created_at.and_utc(),
                content: source.content,
                media_url,
                media_name: source.name,
                media_size: source.media_size,
                media_width: source.media_width,
                media_height: source.media_height,
                media_mime_type: source.media_mime_type,
                media_duration: Some(source.media_duration),
                media_status: Some(if is_attachment {
                    MediaStatus::Canceled
                } else {
                    source.media_status
                }),
                media_waveform: source.media_waveform,
                thumb_image: source.thumb_image,
                thumb_url: source.thumb_url,
                media_key: source.media_key.map(|key| Base64::encode_string(&key)),
                media_digest: source
                    .media_digest
                    .map(|digest| Base64::encode_string(&digest)),
                media_created_at,
                sticker_id: source.sticker_id,
                shared_user_id: source.shared_user_id,
                mentions: None,
                quote_id: source.quote_message_id,
                quote_content: source.quote_content,
                caption: source.caption,
            };
            sanitize_transcript_app_card(&mut transcript);
            if category.is_text() || category.is_post() {
                if let Some(content) = transcript.content.as_deref() {
                    fts_parts.push(content.to_string());
                }
            } else if category.is_data() {
                if let Some(name) = transcript.media_name.as_deref() {
                    fts_parts.push(name.to_string());
                }
            } else if category.is_contact() {
                if let Some(user_id) = transcript.shared_user_id.as_deref() {
                    if let Some(full_name) = user_names.get(user_id) {
                        fts_parts.push(full_name.clone());
                    }
                }
            }
            summaries.push(serde_json::json!({
                "name": sender_name,
                "category": category,
                "content": transcript.content,
            }));
            transcripts.push(transcript);
        }

        let content = serde_json::to_string(&summaries)?;
        let copied_transcript_paths =
            copy_transcript_attachments(&mut transcripts, &account_dir, &transcript_id).await?;
        let message = Message {
            message_id: transcript_id.clone(),
            conversation_id: target_conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category: format!("{target_prefix}_TRANSCRIPT"),
            content: Some(content.clone()),
            media_size: Some(0),
            media_status: if has_attachments {
                MediaStatus::Canceled
            } else {
                MediaStatus::Done
            },
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        let job = Job::create_sending_job(
            &transcript_id,
            target_conversation_id,
            None,
            None,
            false,
            false,
            conversation.expire_in,
        );
        if let Err(error) =
            persist_outgoing_transcript(&self.database, &message, &job, &transcripts, async {
                if has_attachments {
                    AttachmentAccess::new(self.state.clone())
                        .retry_transcript_attachment_locked(&transcript_id)
                        .await?;
                } else {
                    self.app_service.job.wake(&job.action)?;
                }
                Ok(())
            })
            .await
        {
            remove_copied_transcript_files(&copied_transcript_paths).await;
            return Err(error);
        }
        let fts_content = fts_parts.join(" ");
        if !fts_content.is_empty() {
            if let Err(error) = self
                .database
                .message_fts_dao
                .upsert(&transcript_id, target_conversation_id, &fts_content)
                .await
            {
                warn!("failed to index combined transcript {transcript_id}: {error}");
            }
        }
        self.notify_conversation_changed(target_conversation_id);
        Ok(transcript_id)
    }
}

async fn accept_forward_batch(
    database: &crate::db::MixinDatabase,
    prepared: Result<Vec<ForwardPlan>>,
    copied_paths: &[std::path::PathBuf],
) -> Result<Vec<ForwardPlan>> {
    let result = async {
        let plans = prepared?;
        let rows = plans
            .iter()
            .map(|plan| (&plan.message, &plan.job, plan.transcripts.as_deref()))
            .collect::<Vec<_>>();
        database
            .message_dao
            .insert_outgoing_batch_with_transcripts(&rows)
            .await
            .map_err(crate::CoreError::from)?;
        Ok(plans)
    }
    .await;
    if result.is_err() {
        remove_copied_transcript_files(copied_paths).await;
    }
    result
}

async fn persist_outgoing_transcript(
    database: &crate::db::MixinDatabase,
    message: &Message,
    job: &Job,
    transcripts: &[TranscriptMessage],
    upload: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    database
        .message_dao
        .insert_outgoing_message_with_transcripts(message, job, transcripts)
        .await
        .map_err(crate::CoreError::from)?;
    // Acceptance is durable. Retrying the command after an upload error would enqueue a duplicate.
    // The existing message exposes CANCELED media and its attachment retry command resumes this ID.
    if let Err(error) = upload.await {
        warn!(
            "accepted transcript {} needs delivery retry: {error}",
            message.message_id
        );
    }
    Ok(())
}

fn forward_source_attachment_path(
    source: &Message,
    account_dir: &Path,
) -> Result<std::path::PathBuf> {
    let stored = source
        .media_url
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| anyhow!("attachment has no local file: {}", source.message_id))?;
    if Path::new(stored).is_absolute() {
        Ok(Path::new(stored).to_path_buf())
    } else {
        attachment_path(account_dir, source)
    }
}

async fn remove_copied_transcript_files(paths: &[std::path::PathBuf]) {
    for path in paths {
        let _ = crate::fs::remove_file(path).await;
    }
}

async fn copy_transcript_attachments(
    transcripts: &mut [TranscriptMessage],
    account_dir: &Path,
    target_id: &str,
) -> Result<Vec<std::path::PathBuf>> {
    let mut copied = Vec::new();
    let result: Result<()> = async {
        for transcript in transcripts
            .iter_mut()
            .filter(|row| row.category.is_attachment())
        {
            let message = Message {
                message_id: transcript.message_id.clone(),
                category: transcript.category.clone(),
                media_mime_type: transcript.media_mime_type.clone(),
                name: transcript.media_name.clone(),
                ..Default::default()
            };
            let default_path = transcript_attachment_path(account_dir, &message)?;
            let stored = transcript
                .media_url
                .as_deref()
                .filter(|path| !path.trim().is_empty())
                .ok_or_else(|| {
                    anyhow!(
                        "transcript attachment has no local file: {}",
                        transcript.message_id
                    )
                })?;
            let source = if Path::new(stored).is_absolute() {
                Path::new(stored).to_path_buf()
            } else {
                default_path.clone()
            };
            let target = default_path
                .parent()
                .ok_or_else(|| anyhow!("transcript path has no parent"))?
                .join(target_id)
                .join(attachment_file_name(&default_path)?);
            crate::fs::create_dir_all(target.parent().unwrap()).await?;
            copied.push(target.clone());
            crate::fs::copy(&source, &target)
                .await
                .with_context(|| format!("copy transcript attachment {}", transcript.message_id))?;
            transcript.media_url = Some(target.to_string_lossy().into_owned());
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        remove_copied_transcript_files(&copied).await;
        return Err(error);
    }
    Ok(copied)
}

fn prepare_forwarded_transcripts(
    transcripts: Vec<TranscriptMessage>,
    target_id: &str,
    target_prefix: &str,
) -> Result<Vec<TranscriptMessage>> {
    transcripts
        .into_iter()
        .map(|mut transcript| {
            let category = forward_transcript_category(&transcript.category, target_prefix)?;
            if transcript.category.is_attachment() {
                if !matches!(
                    transcript.media_status,
                    Some(MediaStatus::Done | MediaStatus::Read)
                ) {
                    return Err(anyhow!(
                        "transcript attachment is not downloaded: {}",
                        transcript.message_id
                    ));
                }
                if transcript
                    .media_url
                    .as_deref()
                    .is_none_or(|path| path.trim().is_empty())
                {
                    return Err(anyhow!(
                        "transcript attachment has no local file: {}",
                        transcript.message_id
                    ));
                }
                transcript.media_status = Some(MediaStatus::Canceled);
            } else {
                transcript.category = category;
            }
            transcript.transcript_id = target_id.to_string();
            sanitize_transcript_app_card(&mut transcript);
            Ok(transcript)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MixinDatabase;

    fn downloaded_transcript() -> TranscriptMessage {
        serde_json::from_value(serde_json::json!({
            "transcript_id":"original", "message_id":"photo", "user_id":"sender",
            "user_full_name":"Sender", "category":"SIGNAL_IMAGE",
            "created_at":"2024-01-02T03:04:05Z", "content":"old-attachment",
            "media_url":"photo.png", "media_status":"DONE", "media_size":4,
            "media_key":Base64::encode_string(&[1;64]), "media_digest":Base64::encode_string(&[2;32])
        })).unwrap()
    }

    #[test]
    fn forwarding_downloaded_transcript_preserves_source_credentials_until_upload() {
        let original = downloaded_transcript();
        let forwarded = prepare_forwarded_transcripts(vec![original.clone()], "forwarded", "PLAIN")
            .unwrap()
            .remove(0);
        assert_eq!(forwarded.transcript_id, "forwarded");
        assert_eq!(forwarded.message_id, original.message_id);
        assert_eq!(forwarded.created_at, original.created_at);
        assert_eq!(forwarded.category, "SIGNAL_IMAGE");
        assert_eq!(forwarded.media_key, original.media_key);
        assert_eq!(forwarded.media_status, Some(MediaStatus::Canceled));
        for status in [
            None,
            Some(MediaStatus::Pending),
            Some(MediaStatus::Canceled),
        ] {
            let mut unavailable = original.clone();
            unavailable.media_status = status;
            assert!(prepare_forwarded_transcripts(vec![unavailable], "new", "PLAIN").is_err());
        }
        let mut missing_path = original;
        missing_path.media_url = None;
        assert!(prepare_forwarded_transcripts(vec![missing_path], "new", "PLAIN").is_err());
    }

    #[test]
    fn combining_attachment_resolves_original_conversation_file_and_preserves_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let source = Message {
            message_id: "source".into(),
            conversation_id: "original".into(),
            category: "SIGNAL_IMAGE".into(),
            media_url: Some("source.png".into()),
            media_mime_type: Some("image/png".into()),
            ..Default::default()
        };
        let path = attachment_path(directory.path(), &source).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"attachment bytes").unwrap();
        let resolved = forward_source_attachment_path(&source, directory.path()).unwrap();
        assert_eq!(resolved, path);
        assert_eq!(std::fs::read(resolved).unwrap(), b"attachment bytes");
    }

    #[tokio::test]
    async fn transcript_copy_is_independent_and_failed_batch_removes_partial_copies() {
        let directory = tempfile::tempdir().unwrap();
        let mut source = downloaded_transcript();
        let message = Message {
            message_id: source.message_id.clone(),
            category: source.category.clone(),
            ..Default::default()
        };
        let source_path = transcript_attachment_path(directory.path(), &message).unwrap();
        std::fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        std::fs::write(&source_path, b"original bytes").unwrap();
        source.media_url = Some(source_path.to_string_lossy().into_owned());
        let mut forwarded =
            prepare_forwarded_transcripts(vec![source.clone()], "forwarded", "PLAIN").unwrap();
        let copied = copy_transcript_attachments(&mut forwarded, directory.path(), "forwarded")
            .await
            .unwrap();
        assert_eq!(copied.len(), 1);
        assert_ne!(copied[0], source_path);
        assert_eq!(std::fs::read(&copied[0]).unwrap(), b"original bytes");
        assert_eq!(forwarded[0].media_url.as_deref(), copied[0].to_str());
        remove_copied_transcript_files(&copied).await;
        assert!(source_path.exists());
        assert!(!copied[0].exists());
        let mut missing = source.clone();
        missing.message_id = "missing".into();
        missing.media_url = Some(
            directory
                .path()
                .join("missing.png")
                .to_string_lossy()
                .into_owned(),
        );
        assert!(
            copy_transcript_attachments(&mut [source, missing], directory.path(), "failed")
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read_dir(source_path.parent().unwrap().join("failed"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(std::fs::read(source_path).unwrap(), b"original bytes");
    }

    #[tokio::test]
    async fn copy_or_transaction_failure_rejects_batch_and_removes_all_new_files() {
        for reject_job in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap();
            if reject_job {
                crate::db::query("CREATE TRIGGER reject_job BEFORE INSERT ON jobs BEGIN SELECT RAISE(ABORT, 'rejected'); END")
                    .execute(&database.message_dao.0).await.unwrap();
            }
            let source_path = directory.path().join("source.png");
            crate::fs::write(&source_path, b"original bytes")
                .await
                .unwrap();
            let source = downloaded_transcript();
            let mut copied_paths = Vec::new();
            let prepared: Result<Vec<ForwardPlan>> = async {
                let mut first = vec![TranscriptMessage {
                    transcript_id: "first".into(),
                    media_url: Some(source_path.to_string_lossy().into_owned()),
                    ..source.clone()
                }];
                copied_paths.extend(
                    copy_transcript_attachments(&mut first, directory.path(), "first").await?,
                );
                let message = Message {
                    message_id: "first".into(),
                    conversation_id: "conversation".into(),
                    user_id: "me".into(),
                    category: "PLAIN_TRANSCRIPT".into(),
                    media_status: MediaStatus::Canceled,
                    status: MessageStatus::Sending,
                    ..Default::default()
                };
                let plan = ForwardPlan {
                    message,
                    job: Job::create_sending_job(
                        "first",
                        "conversation",
                        None,
                        None,
                        false,
                        false,
                        0,
                    ),
                    transcripts: Some(first),
                };
                if reject_job {
                    return Ok(vec![plan]);
                }
                let mut second = vec![TranscriptMessage {
                    message_id: "missing".into(),
                    media_url: Some(
                        directory
                            .path()
                            .join("missing.png")
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    ..source
                }];
                copied_paths.extend(
                    copy_transcript_attachments(&mut second, directory.path(), "second").await?,
                );
                anyhow::bail!("missing file unexpectedly existed")
            }
            .await;
            assert_eq!(copied_paths.len(), 1);
            assert!(copied_paths[0].exists());
            assert!(accept_forward_batch(&database, prepared, &copied_paths)
                .await
                .is_err());
            assert!(!copied_paths[0].exists());
            assert_eq!(
                crate::fs::read(&source_path).await.unwrap(),
                b"original bytes"
            );
            for (table, sql) in [
                ("messages", "SELECT COUNT(*) FROM messages"),
                ("jobs", "SELECT COUNT(*) FROM jobs"),
                (
                    "transcript_messages",
                    "SELECT COUNT(*) FROM transcript_messages",
                ),
            ] {
                let count: i64 = crate::db::query_scalar(sql)
                    .fetch_one(&database.message_dao.0)
                    .await
                    .unwrap();
                assert_eq!(count, 0, "{table} must remain empty before acceptance");
            }
        }
    }

    #[tokio::test]
    async fn transcript_upload_failure_keeps_acceptance_and_one_retryable_durable_message() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query("INSERT INTO conversations (conversation_id, created_at, status, category) VALUES ('conversation', ?, 2, 'CONTACT')")
            .bind(Utc::now()).execute(&database.message_dao.0).await.unwrap();
        let transcripts =
            prepare_forwarded_transcripts(vec![downloaded_transcript()], "accepted", "PLAIN")
                .unwrap();
        let message = Message {
            message_id: "accepted".into(),
            conversation_id: "conversation".into(),
            category: "PLAIN_TRANSCRIPT".into(),
            status: MessageStatus::Sending,
            media_status: MediaStatus::Canceled,
            content: Some("[]".into()),
            ..Default::default()
        };
        let job = Job::create_sending_job("accepted", "conversation", None, None, false, false, 0);
        let missing_upload_file = directory.path().join("missing-upload-source.png");
        let accepted =
            persist_outgoing_transcript(&database, &message, &job, &transcripts, async {
                // A real local I/O failure occurs after the outgoing transaction has committed.
                crate::fs::read(&missing_upload_file).await?;
                anyhow::bail!("missing source unexpectedly existed")
            })
            .await;
        assert!(
            accepted.is_ok(),
            "a committed message must remain accepted after its upload fails"
        );
        let stored = database
            .message_dao
            .find_message_by_id(&message.message_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.media_status, MediaStatus::Canceled);
        assert_eq!(stored.status, MessageStatus::Sending);
        assert_eq!(
            database
                .transcript_message_dao
                .find_by_transcript_id("accepted")
                .await
                .unwrap()
                .len(),
            1
        );
        let jobs = database.job_dao.sending_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, job.job_id);
        // The existing attachment retry completes the accepted rows rather than enqueueing a new message.
        database
            .transcript_message_dao
            .complete_attachment_upload(
                "accepted",
                "photo",
                "PLAIN_IMAGE",
                "uploaded-id",
                None,
                None,
                Utc::now(),
            )
            .await
            .unwrap();
        database
            .message_dao
            .update_media_status("accepted", MediaStatus::Done)
            .await
            .unwrap();
        assert_eq!(
            database
                .message_dao
                .find_sending_message("accepted")
                .await
                .unwrap()
                .unwrap()
                .media_status,
            MediaStatus::Done
        );
        assert_eq!(
            database.job_dao.sending_jobs().await.unwrap()[0].job_id,
            job.job_id
        );
    }

    #[tokio::test]
    async fn transcript_forward_transaction_failure_rolls_back_then_replay_persists_one_job() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query(
            "INSERT INTO conversations (conversation_id, created_at, status, category) VALUES (?, ?, ?, 'CONTACT')",
        )
        .bind("conversation")
        .bind(Utc::now())
        .bind(2)
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let transcripts =
            prepare_forwarded_transcripts(vec![downloaded_transcript()], "new", "PLAIN").unwrap();
        let parent = Message {
            message_id: "new".into(),
            conversation_id: "conversation".into(),
            user_id: "me".into(),
            category: "PLAIN_TRANSCRIPT".into(),
            content: Some("[]".into()),
            media_status: MediaStatus::Canceled,
            status: MessageStatus::Sending,
            ..Default::default()
        };
        let job = Job::create_sending_job("new", "conversation", None, None, false, false, 0);
        crate::db::query("CREATE TRIGGER reject_job BEFORE INSERT ON jobs BEGIN SELECT RAISE(ABORT, 'rejected'); END")
            .execute(&database.message_dao.0).await.unwrap();
        let upload_marker = directory.path().join("upload-started");
        assert!(
            persist_outgoing_transcript(&database, &parent, &job, &transcripts, async {
                crate::fs::write(&upload_marker, b"started").await?;
                Ok(())
            })
            .await
            .is_err()
        );
        assert!(
            !upload_marker.exists(),
            "an upload must not start before the transaction commits"
        );
        assert!(database
            .message_dao
            .find_message_by_id(&"new".to_string())
            .await
            .unwrap()
            .is_none());
        assert!(database
            .transcript_message_dao
            .find_by_transcript_id("new")
            .await
            .unwrap()
            .is_empty());
        crate::db::query("DROP TRIGGER reject_job")
            .execute(&database.message_dao.0)
            .await
            .unwrap();
        database
            .message_dao
            .insert_outgoing_message_with_transcripts(&parent, &job, &transcripts)
            .await
            .unwrap();
        let stored = database
            .transcript_message_dao
            .find_by_transcript_id("new")
            .await
            .unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].category, "SIGNAL_IMAGE");
        assert_eq!(stored[0].media_status, Some(MediaStatus::Canceled));
        let jobs = database.job_dao.sending_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, job.job_id);
    }
}
