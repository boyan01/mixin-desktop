use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::task::JoinHandle;
use anyhow::Result;
use chrono::Utc;
use log::error;
use tokio::sync::Notify;

use crate::core::attachment::{message_media_files, remove_media_files};
use crate::core::conversation_change::ConversationChangeNotifier;
use crate::db::MixinDatabase;

pub struct ExpiredMessageService {
    handle: JoinHandle<()>,
    notify: Arc<Notify>,
}

impl ExpiredMessageService {
    pub fn new(
        database: Arc<MixinDatabase>,
        changes: Option<ConversationChangeNotifier>,
        account_data_dir: PathBuf,
    ) -> Self {
        let notify = Arc::new(Notify::new());
        let runner_notify = notify.clone();
        let handle = crate::task::spawn(async move {
            loop {
                match cleanup_expired_messages(&database, &account_data_dir).await {
                    Ok(true) => {
                        if let Some(changes) = &changes {
                            changes.notify_all();
                        }
                    }
                    Ok(false) => {}
                    Err(err) => error!("failed to clean up expired messages: {err}"),
                }

                let delay = match database
                    .expired_message_dao
                    .get_first_expired_message()
                    .await
                {
                    Ok(Some(message)) => message
                        .expire_at
                        .map(|expire_at| (expire_at - Utc::now().timestamp()).max(1) as u64)
                        .unwrap_or(60)
                        .min(60),
                    Ok(None) => 60,
                    Err(err) => {
                        error!("failed to schedule expired messages: {err}");
                        5
                    }
                };

                tokio::select! {
                    _ = crate::time::sleep(Duration::from_secs(delay)) => {}
                    _ = runner_notify.notified() => {}
                }
            }
        });
        Self { handle, notify }
    }

    pub fn wake(&self) {
        self.notify.notify_one();
    }

    pub(crate) fn notifier(&self) -> Arc<Notify> {
        self.notify.clone()
    }
}

impl Drop for ExpiredMessageService {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn cleanup_expired_messages(
    database: &MixinDatabase,
    account_data_dir: &Path,
) -> Result<bool> {
    let mut deleted = false;
    for expired in database
        .expired_message_dao
        .get_current_expired_messages()
        .await?
    {
        let Some(message) = database
            .message_dao
            .find_message_by_id(&expired.message_id)
            .await?
        else {
            database
                .expired_message_dao
                .delete_by_message_id(&expired.message_id)
                .await?;
            deleted = true;
            continue;
        };

        remove_media_files(message_media_files(database, account_data_dir, &message).await?)
            .await?;
        database
            .message_dao
            .delete_message(&message.conversation_id, &message.message_id)
            .await?;
        deleted = true;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sdk::message_category::PLAIN_TRANSCRIPT;
    use sdk::MessageStatus;

    use super::*;
    use crate::db::mixin::message::Message;
    use crate::db::mixin::transcript_message::TranscriptMessage;

    #[tokio::test]
    async fn cleanup_removes_media_stored_under_a_relative_path() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let file = directory
            .path()
            .join("Media/Images/conversation/expired.jpg");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"image").unwrap();
        crate::db::query(
            "INSERT INTO conversations (conversation_id, category, name, created_at, status, \
             mute_until) VALUES ('conversation', 'CONTACT', '', 0, 0, 0)",
        )
        .execute(&database.user_dao.0)
        .await
        .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "expired".into(),
                conversation_id: "conversation".into(),
                user_id: "peer".into(),
                category: "SIGNAL_IMAGE".into(),
                media_url: Some("conversation/expired.jpg".into()),
                media_mime_type: Some("image/jpeg".into()),
                status: MessageStatus::Read,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        database
            .expired_message_dao
            .insert("expired", 60, Some(Utc::now().timestamp() - 1))
            .await
            .unwrap();

        cleanup_expired_messages(&database, directory.path())
            .await
            .unwrap();

        assert!(!file.exists());
        database.close().await;
    }

    #[tokio::test]
    async fn cleanup_deletes_expired_message_and_tracking_row() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('conversation', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "expired".into(),
                conversation_id: "conversation".into(),
                user_id: "user".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("content".into()),
                status: MessageStatus::Read,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        database
            .expired_message_dao
            .insert("expired", 60, Some(Utc::now().timestamp() - 1))
            .await
            .unwrap();

        cleanup_expired_messages(&database, directory.path())
            .await
            .unwrap();

        assert!(database
            .message_dao
            .find_message_by_id(&"expired".to_string())
            .await
            .unwrap()
            .is_none());
        assert!(database
            .expired_message_dao
            .get_expired_message_by_id("expired")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn cleanup_deletes_parent_and_transcript_attachment_files() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('conversation', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let parent_path = directory.path().join("parent-attachment");
        let child_path = directory.path().join("transcript-attachment");
        crate::fs::write(&parent_path, b"parent").await.unwrap();
        crate::fs::write(&child_path, b"child").await.unwrap();

        database
            .message_dao
            .insert_message(&Message {
                message_id: "transcript".into(),
                conversation_id: "conversation".into(),
                user_id: "user".into(),
                category: PLAIN_TRANSCRIPT.into(),
                content: Some("[]".into()),
                media_url: Some(parent_path.to_string_lossy().into_owned()),
                status: MessageStatus::Read,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        let transcript: TranscriptMessage = serde_json::from_value(serde_json::json!({
            "transcript_id": "transcript",
            "message_id": "child",
            "category": "SIGNAL_DATA",
            "created_at": "2024-01-02T03:04:05Z",
            "media_url": child_path.to_string_lossy(),
        }))
        .unwrap();
        database
            .transcript_message_dao
            .insert_all(&[transcript])
            .await
            .unwrap();
        database
            .expired_message_dao
            .insert("transcript", 60, Some(Utc::now().timestamp() - 1))
            .await
            .unwrap();

        cleanup_expired_messages(&database, directory.path())
            .await
            .unwrap();

        assert!(!parent_path.exists());
        assert!(!child_path.exists());
        assert!(database
            .transcript_message_dao
            .find_by_transcript_id("transcript")
            .await
            .unwrap()
            .is_empty());
    }
}
