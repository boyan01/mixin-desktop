use super::{MediaStatus, Message};
use crate::db::mixin::MixinDatabase;
use chrono::Utc;
use sdk::MessageStatus;

#[tokio::test]
async fn pinned_media_neighbors_filter_before_limits_and_reject_unpinned_or_wrong_scope_anchors() {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    let dao = &database.message_dao;
    crate::db::query(
        "INSERT INTO conversations (conversation_id,created_at,status,category) \
         VALUES ('conversation',0,0,'CONTACT'),('other',0,0,'CONTACT')",
    )
    .execute(&dao.0)
    .await
    .unwrap();
    let created_at = Utc::now().naive_utc();

    // Reverse lexical IDs and interleave unpinned media at identical timestamps.
    // The owning timeline contract orders these ties by insertion rowid.
    for kind in ["IMAGE", "AUDIO"] {
        let id = |index: usize| format!("{kind}-pin-{:03}", 250 - index);
        let unpinned = |index: usize| format!("{kind}-unpinned-{:03}", 250 - index);
        for index in 0..=250 {
            for (message_id, pinned) in [(id(index), true), (unpinned(index), false)] {
                dao.insert_message(&Message {
                    message_id: message_id.clone(),
                    conversation_id: "conversation".into(),
                    user_id: "sender".into(),
                    category: format!("PLAIN_{kind}"),
                    content: Some(r#"{"attachment_id":"file"}"#.into()),
                    media_url: Some(format!("/{message_id}")),
                    media_status: MediaStatus::Done,
                    status: MessageStatus::Read,
                    created_at,
                    ..Message::default()
                })
                .await
                .unwrap();
                if pinned {
                    crate::db::query(
                        "INSERT INTO pin_messages (message_id,conversation_id,created_at) \
                         VALUES (?,'conversation',0)",
                    )
                    .bind(&message_id)
                    .execute(&dao.0)
                    .await
                    .unwrap();
                }
            }
        }
        let neighbors = if kind == "IMAGE" {
            dao.list_pinned_image_items_around("conversation", &id(125), 1, 1)
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>()
        } else {
            dao.list_pinned_audio_items_around("conversation", &id(125), 1, 1)
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>()
        };
        assert_eq!(neighbors, [id(124), id(125), id(126)]);
        let ordinary = if kind == "IMAGE" {
            dao.list_image_items_around("conversation", &id(125), 1, 1)
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>()
        } else {
            dao.list_audio_items_around("conversation", &id(125), 1, 1)
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ordinary, [unpinned(124), id(125), unpinned(125)]);

        for (conversation, anchor, before, after, expected) in [
            (
                "conversation",
                id(125),
                i64::MAX,
                i64::MAX,
                (25..=225).map(id).collect(),
            ),
            ("conversation", id(125), -1, -100, vec![id(125)]),
            ("other", id(125), 1, 1, vec![]),
            ("conversation", unpinned(125), 1, 1, vec![]),
            ("conversation", "absent".into(), 1, 1, vec![]),
        ] {
            let actual = if kind == "IMAGE" {
                dao.list_pinned_image_items_around(conversation, &anchor, before, after)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|item| item.message_id)
                    .collect::<Vec<_>>()
            } else {
                dao.list_pinned_audio_items_around(conversation, &anchor, before, after)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|item| item.message_id)
                    .collect::<Vec<_>>()
            };
            assert_eq!(actual, expected, "{kind} anchor {anchor}");
        }
        crate::db::query("DELETE FROM pin_messages WHERE message_id = ?")
            .bind(id(125))
            .execute(&dao.0)
            .await
            .unwrap();
        assert!(dao
            .list_pinned_image_items_around("conversation", &id(125), 1, 1)
            .await
            .unwrap()
            .is_empty());
        assert!(dao
            .list_pinned_audio_items_around("conversation", &id(125), 1, 1)
            .await
            .unwrap()
            .is_empty());
    }
    database.close().await;
}
