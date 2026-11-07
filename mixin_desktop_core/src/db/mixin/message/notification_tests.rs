use super::*;

#[tokio::test]
async fn undecrypted_notification_rows_redact_ciphertext_and_do_not_trigger_agents() {
    let directory = tempfile::tempdir().unwrap();
    let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    let dao = &database.message_dao;
    crate::db::query("INSERT INTO conversations (conversation_id,category,created_at,status) VALUES ('chat','CONTACT',0,0)").execute(&dao.0).await.unwrap();
    for (id, status, content) in [
        ("unknown", MessageStatus::Unknown, "not-readable-ciphertext"),
        ("failed", MessageStatus::Failed, "failed-ciphertext"),
        ("decrypted", MessageStatus::Read, "readable message"),
    ] {
        dao.insert_message(&Message {
            message_id: id.into(),
            conversation_id: "chat".into(),
            user_id: "peer".into(),
            category: "SIGNAL_TEXT".into(),
            content: Some(content.into()),
            status,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        })
        .await
        .unwrap();
    }
    let rows = dao.notification_items_after("me", 0, 10).await.unwrap();
    assert_eq!(rows.len(), 3);
    for row in rows {
        let status = row.status;
        let trigger = crate::runtime::model::agent_trigger(&row, "me", "1234");
        let event =
            crate::runtime::model::NotificationEvent::from_message(row, "me", "1234").unwrap();
        assert_eq!(event.status, String::from(status));
        if status == MessageStatus::Read {
            assert_eq!(event.content, "readable message");
            assert_eq!(trigger, Some("direct"));
        } else {
            assert!(event.content.is_empty());
            assert_eq!(trigger, None);
        }
    }
    database.close().await;
}
