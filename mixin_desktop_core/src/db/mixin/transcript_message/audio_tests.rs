use crate::db::mixin::{message::Message, MixinDatabase};
use chrono::Utc;

#[tokio::test]
async fn transcript_audio_neighbors_are_playable_stable_bounded_and_scoped_to_the_root() {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    for (root, conversation, category) in [
        ("root-a", "chat-a", "SIGNAL_TRANSCRIPT"),
        ("root-b", "chat-b", "SIGNAL_TRANSCRIPT"),
        ("wrong-parent", "chat-a", "SIGNAL_TEXT"),
    ] {
        crate::db::query("INSERT OR IGNORE INTO conversations (conversation_id,created_at,status) VALUES (?,0,2)")
            .bind(conversation).execute(&database.message_dao.0).await.unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: root.into(),
                conversation_id: conversation.into(),
                category: category.into(),
                created_at: Utc::now().naive_utc(),
                ..Default::default()
            })
            .await
            .unwrap();
    }
    let dao = &database.transcript_message_dao;
    let ids = (0..205)
        .map(|index| format!("audio-{:03}", 300 - index))
        .collect::<Vec<_>>();
    for (index, id) in ids.iter().enumerate() {
        crate::db::query("INSERT INTO transcript_messages (transcript_id,message_id,category,created_at,media_status,media_url,media_duration) VALUES ('root-a',?,'SIGNAL_AUDIO',?,'DONE',?,'500')")
            .bind(id).bind(index as i64 / 2).bind(format!("{id}.ogg"))
            .execute(&dao.0).await.unwrap();
        crate::db::query("INSERT INTO transcript_messages (transcript_id,message_id,category,created_at,media_status,media_url) VALUES ('root-a',?,'SIGNAL_VIDEO',?,'DONE','video.mp4')")
            .bind(format!("video-{index}")).bind(index as i64 / 2)
            .execute(&dao.0).await.unwrap();
    }
    for (root, id, category, status, url) in [
        (
            "root-b",
            ids[102].as_str(),
            "PLAIN_AUDIO",
            "DONE",
            "other.ogg",
        ),
        (
            "root-a",
            "canceled",
            "PLAIN_AUDIO",
            "CANCELED",
            "canceled.ogg",
        ),
        ("root-a", "missing-file", "PLAIN_AUDIO", "DONE", ""),
        (
            "root-a",
            "read-audio",
            "ENCRYPTED_AUDIO",
            "READ",
            "read.ogg",
        ),
        (
            "wrong-parent",
            ids[102].as_str(),
            "SIGNAL_AUDIO",
            "DONE",
            "wrong.ogg",
        ),
    ] {
        crate::db::query("INSERT INTO transcript_messages (transcript_id,message_id,category,created_at,media_status,media_url) VALUES (?,?,?,2000,?,?)")
            .bind(root).bind(id).bind(category).bind(status).bind(url).execute(&dao.0).await.unwrap();
    }
    let neighbors = dao
        .audio_items_around("root-a", &ids[102], 1, 1)
        .await
        .unwrap();
    assert_eq!(
        neighbors
            .iter()
            .map(|row| row.message_id.as_str())
            .collect::<Vec<_>>(),
        ids[101..104].iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert!(neighbors.iter().all(|row| row.conversation_id == "chat-a"));
    let bounded = dao
        .audio_items_around("root-a", &ids[102], i64::MAX, i64::MAX)
        .await
        .unwrap();
    assert_eq!(bounded.len(), 201);
    assert_eq!(bounded.first().unwrap().message_id, ids[2]);
    assert_eq!(bounded.last().unwrap().message_id, ids[202]);
    assert_eq!(
        dao.audio_items_around("root-a", &ids[102], -10, -10)
            .await
            .unwrap()
            .len(),
        1
    );
    let other = dao
        .audio_items_around("root-b", &ids[102], 100, 100)
        .await
        .unwrap();
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].conversation_id, "chat-b");
    assert_eq!(other[0].media_url.as_deref(), Some("other.ogg"));
    dao.update_media_status("root-a", &ids[102], super::MediaStatus::Read)
        .await
        .unwrap();
    assert_eq!(
        dao.find("root-a", &ids[102])
            .await
            .unwrap()
            .unwrap()
            .media_status,
        Some(super::MediaStatus::Read)
    );
    assert_eq!(
        dao.find("root-b", &ids[102])
            .await
            .unwrap()
            .unwrap()
            .media_status,
        Some(super::MediaStatus::Done)
    );
    for (root, id) in [
        ("root-a", "canceled"),
        ("root-a", "missing-file"),
        ("root-a", "video-102"),
        ("missing", ids[102].as_str()),
        ("wrong-parent", ids[102].as_str()),
    ] {
        assert!(
            dao.audio_items_around(root, id, 100, 100)
                .await
                .unwrap()
                .is_empty(),
            "{root}/{id}"
        );
    }
    let read = dao
        .audio_items_around("root-a", "read-audio", 0, 0)
        .await
        .unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].media_status, Some(super::MediaStatus::Read));
    crate::db::query(
        "DELETE FROM transcript_messages WHERE transcript_id='root-a' AND message_id=?",
    )
    .bind(&ids[102])
    .execute(&dao.0)
    .await
    .unwrap();
    assert!(dao
        .audio_items_around("root-a", &ids[102], 100, 100)
        .await
        .unwrap()
        .is_empty());
    database.close().await;
}
