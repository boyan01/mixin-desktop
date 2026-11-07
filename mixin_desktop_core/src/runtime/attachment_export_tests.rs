use super::*;
use crate::db::mixin::{message::Message, transcript_message::TranscriptMessage};

async fn database(directory: &Path) -> MixinDatabase {
    let database = MixinDatabase::connect_at(directory.join("mixin.db"))
        .await
        .unwrap();
    crate::db::query("INSERT INTO conversations (conversation_id, created_at, status, category) VALUES ('conversation', ?, 2, 'CONTACT')")
        .bind(chrono::Utc::now()).execute(&database.message_dao.0).await.unwrap();
    database
}

fn png() -> Vec<u8> {
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        1,
        image::Rgba([10, 20, 30, 255]),
    ))
    .write_to(&mut encoded, image::ImageFormat::Png)
    .unwrap();
    encoded.into_inner()
}

#[tokio::test]
async fn downloaded_image_export_preserves_source_and_reads_current_persisted_status() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    let message = Message {
        message_id: "image".into(),
        conversation_id: "conversation".into(),
        category: "PLAIN_IMAGE".into(),
        media_status: MediaStatus::Done,
        media_mime_type: Some("image/png".into()),
        media_url: Some("image.png".into()),
        ..Default::default()
    };
    database.message_dao.insert_message(&message).await.unwrap();
    let path = attachment_path(directory.path(), &message).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = png();
    std::fs::write(&path, &bytes).unwrap();
    let (resolved, image) = source(&database, directory.path(), "image", None)
        .await
        .unwrap();
    assert!(image);
    let destination = directory.path().join("export.png");
    save(&resolved, &destination).await.unwrap();
    assert_eq!(std::fs::read(destination).unwrap(), bytes);
    assert!(save(&resolved, &resolved).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    let linked = directory.path().join("linked-export.png");
    std::fs::hard_link(&resolved, &linked).unwrap();
    save(&resolved, &linked).await.unwrap();
    std::fs::write(linked, b"new destination bytes").unwrap();
    assert_eq!(std::fs::read(&resolved).unwrap(), bytes);
    database
        .message_dao
        .update_media_status("image", MediaStatus::Canceled)
        .await
        .unwrap();
    assert!(source(&database, directory.path(), "image", None)
        .await
        .is_err());
    database.close().await;
}

#[tokio::test]
async fn transcript_export_accepts_legacy_and_root_scoped_copies_and_rejects_other_roots() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    database
        .message_dao
        .insert_message(&Message {
            message_id: "root".into(),
            conversation_id: "conversation".into(),
            category: "PLAIN_TRANSCRIPT".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let legacy = directory.path().join("Media/Transcripts/legacy.png");
    let scoped = directory.path().join("Media/Transcripts/root/scoped.png");
    for path in [&legacy, &scoped] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, png()).unwrap();
    }
    let rows: Vec<TranscriptMessage> = serde_json::from_value(serde_json::json!([
        {"transcript_id":"root", "message_id":"legacy", "category":"PLAIN_IMAGE", "created_at":now, "media_status":"DONE", "media_mime_type":"image/png", "media_url":"legacy.png"},
        {"transcript_id":"root", "message_id":"scoped", "category":"PLAIN_IMAGE", "created_at":now, "media_status":"DONE", "media_mime_type":"image/png", "media_url":scoped},
        {"transcript_id":"root", "message_id":"foreign", "category":"PLAIN_IMAGE", "created_at":now, "media_status":"DONE", "media_mime_type":"image/png", "media_url":legacy},
        {"transcript_id":"other", "message_id":"isolated", "category":"PLAIN_IMAGE", "created_at":now, "media_status":"DONE", "media_mime_type":"image/png", "media_url":"isolated.png"}
    ])).unwrap();
    database
        .transcript_message_dao
        .insert_all(&rows)
        .await
        .unwrap();
    for id in ["legacy", "scoped"] {
        let (resolved, _) = source(&database, directory.path(), id, Some("root"))
            .await
            .unwrap();
        let target = directory.path().join(format!("{id}-export.png"));
        save(&resolved, &target).await.unwrap();
        assert_eq!(std::fs::read(target).unwrap(), png());
    }
    for id in ["foreign", "isolated"] {
        assert!(source(&database, directory.path(), id, Some("root"))
            .await
            .is_err());
    }
    database.close().await;
}

#[cfg(unix)]
#[tokio::test]
async fn malicious_media_url_and_symlink_cannot_export_files_outside_account_media() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("private.png");
    std::fs::write(&secret, png()).unwrap();
    let database = database(directory.path()).await;
    let message = Message {
        message_id: "image".into(),
        conversation_id: "conversation".into(),
        category: "PLAIN_IMAGE".into(),
        media_status: MediaStatus::Done,
        media_mime_type: Some("image/png".into()),
        media_url: Some(secret.to_string_lossy().into_owned()),
        ..Default::default()
    };
    database.message_dao.insert_message(&message).await.unwrap();
    assert!(source(&database, directory.path(), "image", None)
        .await
        .is_err());
    let expected = attachment_path(directory.path(), &message).unwrap();
    std::fs::create_dir_all(expected.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&secret, expected).unwrap();
    assert!(source(&database, directory.path(), "image", None)
        .await
        .is_err());
    database.close().await;
}

#[test]
fn clipboard_export_contains_real_png_pixels_and_rejects_invalid_or_oversized_images() {
    let uri = image_data_uri(png()).unwrap();
    let encoded = uri.strip_prefix("data:image/png;base64,").unwrap();
    let bytes = Base64::decode_vec(encoded).unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (2, 1));
    assert_eq!(decoded.get_pixel(0, 0).0, [10, 20, 30, 255]);
    assert!(image_data_uri(b"not an image".to_vec()).is_err());
    let mut oversized = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::new(20_000, 1))
        .write_to(&mut oversized, image::ImageFormat::Png)
        .unwrap();
    assert!(image_data_uri(oversized.into_inner()).is_err());
}
