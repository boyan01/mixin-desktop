use std::collections::HashSet;

#[tokio::test]
async fn pages_beyond_five_hundred_keep_position_filter_and_tie_order_consistent() {
    let directory = tempfile::tempdir().unwrap();
    let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    let dao = &database.conversation_dao;
    crate::db::query("INSERT INTO users (user_id,identity_number,full_name,relationship,app_id) VALUES ('friend','1234','Friend','FRIEND',NULL),('stranger','1235','Stranger','STRANGER',NULL),('bot','1236','Robot','FRIEND','bot-app')")
        .execute(&dao.0).await.unwrap();
    for index in 0..660 {
        let (category, owner) = match index % 4 {
            0 => ("GROUP", "friend"),
            1 => ("CONTACT", "friend"),
            2 => ("CONTACT", "stranger"),
            _ => ("CONTACT", "bot"),
        };
        crate::db::query("INSERT INTO conversations (conversation_id,category,owner_id,name,created_at,status,last_message_created_at,unseen_message_count,draft,pin_time) VALUES (?,?,?,?,10,2,20,?,'',?)")
            .bind(format!("conversation-{index:03}")).bind(category).bind(owner).bind(format!("Group {index}"))
            .bind(i64::from(index % 3 == 0)).bind((index % 31 == 0).then_some(100_i64)).execute(&dao.0).await.unwrap();
    }
    let mut ids = Vec::new();
    for offset in (0..700).step_by(100) {
        let page = dao
            .list_items("chats", None, "", false, 100, offset)
            .await
            .unwrap();
        for (at, row) in page.iter().enumerate() {
            assert_eq!(
                dao.item_position("chats", None, "", false, &row.conversation_id)
                    .await
                    .unwrap(),
                Some(offset + at as i64)
            );
        }
        ids.extend(page.into_iter().map(|row| row.conversation_id));
    }
    assert_eq!(ids.len(), 660);
    assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 660);
    assert_eq!(
        dao.list_items("chats", None, "", false, 5000, 0)
            .await
            .unwrap()
            .len(),
        500
    );
    assert_eq!(
        dao.count_items("chats", None, "", false).await.unwrap(),
        660
    );
    for category in ["contacts", "groups", "strangers", "bots"] {
        let rows = dao
            .list_items(category, None, "", false, 500, 0)
            .await
            .unwrap();
        assert_eq!(rows.len(), 165);
        assert_eq!(
            dao.count_items(category, None, "", false).await.unwrap(),
            165
        );
        for (at, row) in rows.iter().enumerate() {
            assert_eq!(
                dao.item_position(category, None, "", false, &row.conversation_id)
                    .await
                    .unwrap(),
                Some(at as i64)
            );
        }
        let unrelated = ids
            .iter()
            .find(|id| !rows.iter().any(|row| &row.conversation_id == *id))
            .unwrap();
        assert_eq!(
            dao.item_position(category, None, "", false, unrelated)
                .await
                .unwrap(),
            None
        );
    }
    let unread = dao
        .list_items("chats", None, "", true, 500, 0)
        .await
        .unwrap();
    assert_eq!(unread.len(), 220);
    assert_eq!(dao.count_items("chats", None, "", true).await.unwrap(), 220);
    assert!(unread.iter().all(|row| row.unseen_count > 0));
    let keyword = dao
        .list_items("groups", None, "Group 65", false, 100, 0)
        .await
        .unwrap();
    assert!(!keyword.is_empty());
    for (at, row) in keyword.iter().enumerate() {
        assert_eq!(
            dao.item_position("groups", None, "Group 65", false, &row.conversation_id)
                .await
                .unwrap(),
            Some(at as i64)
        );
    }
    database.close().await;
}

#[tokio::test]
async fn filtered_anchor_positions_follow_pin_draft_and_membership_changes() {
    let directory = tempfile::tempdir().unwrap();
    let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    let dao = &database.conversation_dao;
    for id in ["alpha", "beta", "gamma"] {
        crate::db::query("INSERT INTO conversations (conversation_id,category,name,created_at,status,last_message_created_at,draft) VALUES (?,'GROUP',?,10,2,20,'')")
            .bind(id).bind(id).execute(&dao.0).await.unwrap();
        crate::db::query("INSERT INTO circle_conversations (conversation_id,circle_id,user_id,created_at) VALUES (?,'circle','',0)").bind(id).execute(&dao.0).await.unwrap();
    }
    let ordered = dao
        .list_items("circle", Some("circle"), "", false, 100, 0)
        .await
        .unwrap();
    assert_eq!(
        ordered
            .iter()
            .map(|row| row.conversation_id.as_str())
            .collect::<Vec<_>>(),
        ["gamma", "beta", "alpha"]
    );
    crate::db::query("UPDATE conversations SET draft='draft' WHERE conversation_id='alpha'")
        .execute(&dao.0)
        .await
        .unwrap();
    assert_eq!(
        dao.item_position("circle", Some("circle"), "", false, "alpha")
            .await
            .unwrap(),
        Some(0)
    );
    dao.set_pinned("beta", true).await.unwrap();
    assert_eq!(
        dao.item_position("circle", Some("circle"), "", false, "beta")
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        dao.item_position("circle", Some("circle"), "", false, "alpha")
            .await
            .unwrap(),
        Some(1)
    );
    crate::db::query("DELETE FROM circle_conversations WHERE conversation_id='alpha'")
        .execute(&dao.0)
        .await
        .unwrap();
    assert_eq!(
        dao.item_position("circle", Some("circle"), "", false, "alpha")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        dao.count_items("circle", Some("circle"), "", false)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        dao.item_position("circle", Some("missing"), "", false, "beta")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        dao.item_position("chats", None, "", false, "missing")
            .await
            .unwrap(),
        None
    );
    database.close().await;
}
