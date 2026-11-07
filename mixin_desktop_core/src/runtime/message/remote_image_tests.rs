use super::*;
use crate::db::{app::AppDatabase, mixin::MixinDatabase};
use crate::network::NetworkService;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn remote_image_metadata_checks_actual_format_and_rejects_corrupt_or_excessive_pixels() {
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        1,
        image::Rgba([10, 20, 30, 255]),
    ))
    .write_to(&mut output, image::ImageFormat::Png)
    .unwrap();
    let mut png = output.into_inner();
    assert_eq!(
        remote_image_metadata(&png).unwrap(),
        ("image/png", "png", 2, 1)
    );
    assert!(remote_image_metadata(&png[..png.len() / 2]).is_err());
    // Keep a valid PNG header/checksum while declaring a decompression bomb.
    png[16..20].copy_from_slice(&8192_u32.to_be_bytes());
    png[20..24].copy_from_slice(&8192_u32.to_be_bytes());
    let checksum = crc32fast::hash(&png[12..29]);
    png[29..33].copy_from_slice(&checksum.to_be_bytes());
    assert!(remote_image_metadata(&png)
        .unwrap_err()
        .to_string()
        .contains("pixel limit"));
    assert!(remote_image_metadata(b"<svg width='1' height='1'/>").is_err());
}

#[tokio::test]
async fn oversized_remote_response_is_rejected_before_waiting_for_its_body() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppDatabase::connect_at(directory.path().join("app.db"))
        .await
        .unwrap();
    let network = NetworkService::new(app.setting_dao.clone()).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/image", listener.local_addr().unwrap());
    let (finish, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 2048];
        let count = stream.read(&mut request).await.unwrap();
        assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /image HTTP/1.1"));
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    MAX_REMOTE_IMAGE_BYTES + 1
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let _ = wait.await;
    });
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        download_remote_image(&network, &url, &CancellationToken::new()),
    )
    .await;
    let _ = finish.send(());
    server.await.unwrap();
    assert!(result
        .expect("oversized headers must not wait for a body")
        .is_err());
    drop(network);
    app.property_dao.0.close().await;
}

#[tokio::test]
async fn remote_download_waiting_for_body_stops_when_transfer_is_canceled() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppDatabase::connect_at(directory.path().join("app.db"))
        .await
        .unwrap();
    let network = NetworkService::new(app.setting_dao.clone()).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/image", listener.local_addr().unwrap());
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    let (finish, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 2048];
        assert!(stream.read(&mut request).await.unwrap() > 0);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\nGIF89a")
            .await
            .unwrap();
        cancel.cancel();
        let _ = wait.await;
    });
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        download_remote_image(&network, &url, &cancellation),
    )
    .await;
    let _ = finish.send(());
    server.await.unwrap();
    assert!(result
        .expect("cancellation must not wait for the rest of the body")
        .unwrap_err()
        .to_string()
        .contains("canceled"));
    drop(network);
    app.property_dao.0.close().await;
}

#[tokio::test]
async fn transfer_cancellation_changes_only_pending_messages_and_preserves_completed_jobs() {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    crate::db::query("INSERT INTO conversations (conversation_id, created_at, status, category) VALUES ('conversation', 0, 2, 'CONTACT')").execute(&database.message_dao.0).await.unwrap();
    for (id, status, media_status) in [
        ("sending", MessageStatus::Sending, MediaStatus::Pending),
        ("failed", MessageStatus::Failed, MediaStatus::Pending),
        ("received", MessageStatus::Delivered, MediaStatus::Pending),
        ("done", MessageStatus::Sending, MediaStatus::Pending),
        ("read", MessageStatus::Read, MediaStatus::Read),
    ] {
        database
            .message_dao
            .insert_message(&Message {
                message_id: id.into(),
                conversation_id: "conversation".into(),
                category: "SIGNAL_IMAGE".into(),
                status,
                media_status,
                ..Default::default()
            })
            .await
            .unwrap();
    }
    let job = Job::create_sending_job("done", "conversation", None, None, false, false, 0);
    assert!(database
        .message_dao
        .complete_pending_attachment("done", "done.png", &completed_update(), &job)
        .await
        .unwrap());
    for id in ["sending", "failed", "received"] {
        assert!(database
            .message_dao
            .cancel_pending_transfer(id)
            .await
            .unwrap());
        let row = database
            .message_dao
            .find_message_by_id(&id.to_owned())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.media_status, MediaStatus::Canceled);
    }
    for id in ["done", "read", "missing"] {
        assert!(!database
            .message_dao
            .cancel_pending_transfer(id)
            .await
            .unwrap());
    }
    let done = database
        .message_dao
        .find_message_by_id(&"done".to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.media_status, MediaStatus::Done);
    assert_eq!(done.content.as_deref(), Some("committed attachment"));
    let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
        .bind(&job.job_id)
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
    assert_eq!(jobs, 1);
    database.close().await;
}

#[tokio::test]
async fn transcript_cancellation_keeps_completed_child_and_other_roots_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    let rows: Vec<crate::db::mixin::transcript_message::TranscriptMessage> = serde_json::from_value(serde_json::json!([
        {"transcript_id":"root", "message_id":"pending", "category":"SIGNAL_IMAGE", "created_at":Utc::now(), "media_status":"PENDING"},
        {"transcript_id":"root", "message_id":"done", "category":"SIGNAL_IMAGE", "created_at":Utc::now(), "media_status":"DONE"},
        {"transcript_id":"other", "message_id":"pending", "category":"SIGNAL_IMAGE", "created_at":Utc::now(), "media_status":"PENDING"}
    ])).unwrap();
    database
        .transcript_message_dao
        .insert_all(&rows)
        .await
        .unwrap();
    assert!(database
        .transcript_message_dao
        .cancel_pending_transfer("root", "pending")
        .await
        .unwrap());
    assert!(!database
        .transcript_message_dao
        .cancel_pending_transfer("root", "done")
        .await
        .unwrap());
    assert_eq!(
        database
            .transcript_message_dao
            .find("root", "pending")
            .await
            .unwrap()
            .unwrap()
            .media_status,
        Some(MediaStatus::Canceled)
    );
    assert_eq!(
        database
            .transcript_message_dao
            .find("root", "done")
            .await
            .unwrap()
            .unwrap()
            .media_status,
        Some(MediaStatus::Done)
    );
    assert_eq!(
        database
            .transcript_message_dao
            .find("other", "pending")
            .await
            .unwrap()
            .unwrap()
            .media_status,
        Some(MediaStatus::Pending)
    );
    assert!(!database
        .transcript_message_dao
        .complete_attachment_upload_if_pending(
            "root",
            "pending",
            "SIGNAL_IMAGE",
            "late-upload",
            None,
            None,
            Utc::now()
        )
        .await
        .unwrap());
    assert!(database
        .transcript_message_dao
        .complete_attachment_upload_if_pending(
            "other",
            "pending",
            "SIGNAL_IMAGE",
            "uploaded",
            None,
            None,
            Utc::now()
        )
        .await
        .unwrap());
    assert_eq!(
        database
            .transcript_message_dao
            .find("root", "pending")
            .await
            .unwrap()
            .unwrap()
            .media_status,
        Some(MediaStatus::Canceled)
    );
    database.close().await;
}

#[tokio::test]
async fn failed_retry_job_insert_rolls_back_completion_and_same_id_remains_retryable() {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    crate::db::query("INSERT INTO conversations (conversation_id, created_at, status, category) VALUES ('conversation', 0, 2, 'CONTACT')").execute(&database.message_dao.0).await.unwrap();
    database
        .message_dao
        .insert_message(&Message {
            message_id: "retry".into(),
            conversation_id: "conversation".into(),
            category: "SIGNAL_IMAGE".into(),
            status: MessageStatus::Failed,
            content: Some("old attachment".into()),
            media_status: MediaStatus::Pending,
            ..Default::default()
        })
        .await
        .unwrap();
    crate::db::query("CREATE TRIGGER fail_retry_job BEFORE INSERT ON jobs BEGIN SELECT RAISE(ABORT, 'test job failure'); END").execute(&database.message_dao.0).await.unwrap();
    let job = Job::create_sending_job("retry", "conversation", None, None, false, false, 0);
    assert!(database
        .message_dao
        .complete_pending_attachment("retry", "retry.png", &completed_update(), &job)
        .await
        .is_err());
    let row = database
        .message_dao
        .find_message_by_id(&"retry".to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.media_status, MediaStatus::Pending);
    assert_eq!(row.status, MessageStatus::Failed);
    assert_eq!(row.content.as_deref(), Some("old attachment"));
    let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
    assert_eq!(jobs, 0);
    assert!(database
        .message_dao
        .cancel_pending_transfer("retry")
        .await
        .unwrap());
    crate::db::query("DROP TRIGGER fail_retry_job")
        .execute(&database.message_dao.0)
        .await
        .unwrap();
    database
        .message_dao
        .update_media_status("retry", MediaStatus::Pending)
        .await
        .unwrap();
    assert!(database
        .message_dao
        .complete_pending_attachment("retry", "retry.png", &completed_update(), &job)
        .await
        .unwrap());
    let row = database
        .message_dao
        .find_message_by_id(&"retry".to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.media_status, MediaStatus::Done);
    assert_eq!(row.status, MessageStatus::Sending);
    assert_eq!(row.content.as_deref(), Some("committed attachment"));
    let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
        .bind(&job.job_id)
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
    assert_eq!(jobs, 1);
    database.close().await;
}

fn completed_update() -> AttachmentMessageUpdate {
    AttachmentMessageUpdate {
        status: MessageStatus::Sending,
        content: "committed attachment".into(),
        media_mime_type: "image/png".into(),
        media_size: 24,
        media_status: MediaStatus::Done,
        media_width: Some(2),
        media_height: Some(1),
        media_digest: None,
        media_key: None,
        media_waveform: None,
        caption: None,
        name: None,
        thumb_image: None,
        media_duration: None,
    }
}
