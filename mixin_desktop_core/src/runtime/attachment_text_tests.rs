use super::*;
use crate::core::attachment::attachment_path;
use crate::db::mixin::{
    message::{MediaStatus, Message},
    transcript_message::TranscriptMessage,
};

async fn database(directory: &Path) -> MixinDatabase {
    let database = MixinDatabase::connect_at(directory.join("mixin.db"))
        .await
        .unwrap();
    crate::db::query("INSERT INTO conversations (conversation_id,created_at,status,category) VALUES ('chat',0,2,'CONTACT')")
        .execute(&database.message_dao.0).await.unwrap();
    database
}

async fn markdown(database: &MixinDatabase, directory: &Path, bytes: &[u8]) -> std::path::PathBuf {
    let message = Message {
        message_id: "markdown".into(),
        conversation_id: "chat".into(),
        category: "SIGNAL_DATA".into(),
        name: Some("notes.MD".into()),
        media_status: MediaStatus::Done,
        ..Default::default()
    };
    database.message_dao.insert_message(&message).await.unwrap();
    let path = attachment_path(directory, &message).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    path
}

#[tokio::test]
async fn downloaded_markdown_reads_utf8_and_bom_from_persisted_path_without_changing_source() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    let bytes = "\u{feff}# Notes\n中文 *markdown* 🦀\n".as_bytes();
    let path = markdown(&database, directory.path(), bytes).await;
    assert_eq!(
        load_content(&database, directory.path(), "markdown", None)
            .await
            .unwrap(),
        "# Notes\n中文 *markdown* 🦀\n"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    database
        .message_dao
        .update_media_status("markdown", MediaStatus::Canceled)
        .await
        .unwrap();
    assert!(load_content(&database, directory.path(), "markdown", None)
        .await
        .is_err());
    database.close().await;
}

#[tokio::test]
async fn reader_accepts_the_exact_byte_limit_and_rejects_oversized_or_invalid_utf8_without_truncating(
) {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    let path = markdown(&database, directory.path(), &vec![b'a'; MAX_MARKDOWN_BYTES]).await;
    assert_eq!(
        load_content(&database, directory.path(), "markdown", None)
            .await
            .unwrap()
            .len(),
        MAX_MARKDOWN_BYTES
    );
    std::fs::write(&path, vec![b'a'; MAX_MARKDOWN_BYTES + 1]).unwrap();
    assert!(load_content(&database, directory.path(), "markdown", None)
        .await
        .unwrap_err()
        .to_string()
        .contains("reader limit"));
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(load_content(&database, directory.path(), "markdown", None)
        .await
        .unwrap_err()
        .to_string()
        .contains("UTF-8"));
    database.close().await;
}

#[tokio::test]
async fn transcript_markdown_reads_only_the_requested_root_and_rejects_wrong_root_or_non_markdown_rows(
) {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    for root in ["root", "other"] {
        database
            .message_dao
            .insert_message(&Message {
                message_id: root.into(),
                conversation_id: "chat".into(),
                category: "SIGNAL_TRANSCRIPT".into(),
                ..Default::default()
            })
            .await
            .unwrap();
    }
    let path = directory.path().join("Media/Transcripts/root/child.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "# Root only").unwrap();
    let now = chrono::Utc::now();
    let rows: Vec<TranscriptMessage> = serde_json::from_value(serde_json::json!([
        {"transcript_id":"root","message_id":"child","category":"SIGNAL_DATA","created_at":now,"media_name":"notes.md","media_status":"DONE","media_url":path},
        {"transcript_id":"other","message_id":"child","category":"SIGNAL_DATA","created_at":now,"media_name":"notes.md","media_status":"DONE","media_url":path},
        {"transcript_id":"root","message_id":"plain","category":"SIGNAL_DATA","created_at":now,"media_name":"notes.txt","media_status":"DONE","media_url":path}
    ])).unwrap();
    database
        .transcript_message_dao
        .insert_all(&rows)
        .await
        .unwrap();
    assert_eq!(
        load_content(&database, directory.path(), "child", Some("root"))
            .await
            .unwrap(),
        "# Root only"
    );
    assert!(
        load_content(&database, directory.path(), "child", Some("other"))
            .await
            .is_err()
    );
    assert!(
        load_content(&database, directory.path(), "child", Some("missing"))
            .await
            .is_err()
    );
    assert!(
        load_content(&database, directory.path(), "plain", Some("root"))
            .await
            .is_err()
    );
    database.close().await;
}
