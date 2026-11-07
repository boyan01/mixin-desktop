use super::*;
use crate::{core::attachment::attachment_path, db::mixin::message::Message};

#[tokio::test]
async fn completed_photo_retry_repairs_missing_or_corrupt_files_and_preserves_valid_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES ('chat', 'CONTACT', ?, 2)")
        .bind(chrono::Utc::now())
        .execute(&database.message_dao.0)
        .await
        .unwrap();
    let message = Message {
        message_id: "photo".into(),
        conversation_id: "chat".into(),
        user_id: "peer".into(),
        category: "SIGNAL_IMAGE".into(),
        media_status: MediaStatus::Done,
        media_url: Some("photo.png".into()),
        media_mime_type: Some("image/png".into()),
        ..Default::default()
    };
    database.message_dao.insert_message(&message).await.unwrap();
    let path = attachment_path(directory.path(), &message).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(2, 2)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let valid = encoded.into_inner();
    for (content, expected) in [
        (None, true),
        (Some(b"broken PNG".to_vec()), true),
        (Some(valid.clone()), false),
    ] {
        if let Some(content) = content.as_ref() {
            std::fs::write(&path, content).unwrap();
        }
        assert_eq!(
            download_required(
                &database,
                directory.path(),
                "photo",
                None,
                "SIGNAL_IMAGE",
                MediaStatus::Done
            )
            .await
            .unwrap(),
            expected
        );
        if let Some(content) = content {
            assert_eq!(std::fs::read(&path).unwrap(), content);
        }
    }
    assert_eq!(std::fs::read(&path).unwrap(), valid);
    database.close().await;
}
