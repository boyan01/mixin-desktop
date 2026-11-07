use super::*;

#[tokio::test]
async fn quoted_sticker_sync_refreshes_every_reader_without_rewriting_quote_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    let dao = &database.message_dao;
    crate::db::query("INSERT INTO conversations (conversation_id,category,created_at,status) VALUES ('chat','CONTACT',0,0)").execute(&dao.0).await.unwrap();
    let snapshot = r#"{"type":"PLAIN_STICKER","sticker_id":"late-sticker","asset_url":"https://example.invalid/old.png","asset_type":"png","user_full_name":"Original author","message_id":"original"}"#;
    for (id, category, quote) in [
        ("reply", "PLAIN_TEXT", Some(snapshot)),
        ("invalid", "PLAIN_TEXT", Some("not-json")),
        ("parent", "SIGNAL_TRANSCRIPT", None),
    ] {
        dao.insert_message(&Message {
            message_id: id.into(),
            conversation_id: "chat".into(),
            user_id: "sender".into(),
            category: category.into(),
            content: Some("message".into()),
            quote_content: quote.map(str::to_owned),
            status: MessageStatus::Read,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        })
        .await
        .unwrap();
    }
    crate::db::query("INSERT INTO pin_messages (message_id,conversation_id,created_at) VALUES ('reply','chat',0)").execute(&dao.0).await.unwrap();
    crate::db::query("INSERT INTO transcript_messages (transcript_id,message_id,category,created_at,quote_id,quote_content) VALUES ('parent','child','PLAIN_TEXT',0,'original',?)")
        .bind(snapshot).execute(&dao.0).await.unwrap();
    let before = dao.list_items_by_ids(&["reply".into()]).await.unwrap();
    assert_eq!(before[0].quote_content.as_deref(), Some(snapshot));
    crate::db::query("INSERT INTO stickers (sticker_id,name,asset_url,asset_type,asset_width,asset_height,created_at) VALUES ('late-sticker','Current sticker','https://example.invalid/current.json','json',256,128,0)")
        .execute(&dao.0).await.unwrap();
    for rows in [
        dao.list_items("chat", None, None, 10).await.unwrap(),
        dao.list_items_by_ids(&["reply".into()]).await.unwrap(),
        dao.list_pinned_items_page("chat", 0, 10).await.unwrap(),
    ] {
        let reply = rows.iter().find(|row| row.message_id == "reply").unwrap();
        let quote: serde_json::Value =
            serde_json::from_str(reply.quote_content.as_deref().unwrap()).unwrap();
        assert_eq!(quote["asset_url"], "https://example.invalid/current.json");
        assert_eq!(quote["asset_type"], "json");
        assert_eq!(quote["asset_width"], 256);
        assert_eq!(quote["asset_height"], 128);
        assert_eq!(quote["asset_name"], "Current sticker");
        assert_eq!(quote["user_full_name"], "Original author");
        if let Some(invalid) = rows.iter().find(|row| row.message_id == "invalid") {
            assert_eq!(invalid.quote_content.as_deref(), Some("not-json"));
        }
    }
    let children = database
        .transcript_message_dao
        .list_items("parent")
        .await
        .unwrap();
    let quote: serde_json::Value =
        serde_json::from_str(children[0].quote_content.as_deref().unwrap()).unwrap();
    assert_eq!(quote["asset_url"], "https://example.invalid/current.json");
    assert_eq!(quote["asset_type"], "json");
    assert_eq!(
        dao.find_message_by_id(&"reply".into())
            .await
            .unwrap()
            .unwrap()
            .quote_content
            .as_deref(),
        Some(snapshot)
    );
    assert_eq!(
        database
            .transcript_message_dao
            .find("parent", "child")
            .await
            .unwrap()
            .unwrap()
            .quote_content
            .as_deref(),
        Some(snapshot)
    );
    database.close().await;
}
