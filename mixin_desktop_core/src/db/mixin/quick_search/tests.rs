use super::*;

async fn database() -> (tempfile::TempDir, MixinDatabase) {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    (directory, database)
}

async fn user(
    database: &MixinDatabase,
    id: &str,
    name: &str,
    identity: &str,
    relationship: &str,
    bot: bool,
) {
    crate::db::query("INSERT INTO users (user_id,full_name,identity_number,relationship,avatar_url,is_verified,app_id) VALUES (?,?,?,?, '',FALSE,?)")
        .bind(id).bind(name).bind(identity).bind(relationship).bind(bot.then_some(id)).execute(&database.user_dao.0).await.unwrap();
}

async fn conversation(
    database: &MixinDatabase,
    id: &str,
    name: &str,
    owner: &str,
    category: &str,
    pin: i64,
) {
    crate::db::query("INSERT INTO conversations (conversation_id,name,owner_id,category,created_at,status,pin_time) VALUES (?,?,?,?,0,2,?)")
        .bind(id).bind(name).bind(owner).bind(category).bind(pin).execute(&database.user_dao.0).await.unwrap();
}

#[tokio::test]
async fn exact_prefix_and_contains_compete_across_users_and_chats_before_paging() {
    let (_directory, database) = database().await;
    user(&database, "exact-user", "Alice", "7001", "FRIEND", false).await;
    user(
        &database,
        "prefix-user",
        "Alice suffix",
        "7002",
        "FRIEND",
        false,
    )
    .await;
    user(&database, "bot", "xxAlice", "7003", "FRIEND", true).await;
    user(&database, "stranger", "Alice", "7004", "STRANGER", false).await;
    user(&database, "owner", "Group owner", "7005", "STRANGER", false).await;
    conversation(&database, "exact-chat", "Alice", "owner", "GROUP", 0).await;
    conversation(&database, "friend-chat", "", "exact-user", "CONTACT", 999).await;
    conversation(&database, "stranger-chat", "", "stranger", "CONTACT", 5).await;
    conversation(&database, "prefix-chat", "Alice test", "owner", "GROUP", 0).await;
    conversation(
        &database,
        "zero-score-chat",
        "Unrelated",
        "owner",
        "GROUP",
        0,
    )
    .await;
    let expected = vec![
        (0, "exact-user".into()),
        (1, "stranger-chat".into()),
        (1, "exact-chat".into()),
        (1, "prefix-chat".into()),
        (0, "prefix-user".into()),
        (0, "bot".into()),
        (1, "zero-score-chat".into()),
    ];
    assert_eq!(
        ranked_ids(&database, "Alice", 0, 200).await.unwrap(),
        expected
    );
    let mut pages = Vec::new();
    for offset in [0, 2, 4, 6] {
        pages.extend(ranked_ids(&database, "Alice", offset, 2).await.unwrap());
    }
    assert_eq!(pages, expected);
    assert!(ranked_ids(&database, "  ", 0, 20).await.unwrap().is_empty());
    assert!(ranked_ids(&database, "Alice", 0, 0)
        .await
        .unwrap()
        .is_empty());
    database.close().await;
}

#[tokio::test]
async fn identity_exact_and_prefix_scores_rank_between_name_matches() {
    let (_directory, database) = database().await;
    user(&database, "name-exact", "7000", "1000", "FRIEND", false).await;
    user(&database, "id-exact", "Somebody", "7000", "FRIEND", false).await;
    user(
        &database,
        "name-prefix",
        "7000 test",
        "1001",
        "FRIEND",
        false,
    )
    .await;
    user(&database, "id-prefix", "Other", "7000123", "FRIEND", false).await;
    user(
        &database,
        "name-contains",
        "xx7000xx",
        "1002",
        "FRIEND",
        false,
    )
    .await;
    user(
        &database,
        "id-contains",
        "Third",
        "99700099",
        "FRIEND",
        false,
    )
    .await;
    let ids: Vec<_> = ranked_ids(&database, "7000", 0, 20)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    assert_eq!(
        ids,
        [
            "name-exact",
            "id-exact",
            "name-prefix",
            "id-prefix",
            "name-contains",
            "id-contains"
        ]
    );
    database.close().await;
}

#[tokio::test]
async fn three_keyword_scores_reward_sequence_spacing_and_prefix_without_requiring_all_words() {
    let (_directory, database) = database().await;
    user(&database, "owner", "Owner", "1000", "STRANGER", false).await;
    for (id, name) in [
        ("prefix", "alpha beta gamma"),
        ("embedded", "X alpha beta gamma"),
        ("sequential", "alpha X beta gamma"),
        ("reversed", "gamma beta alpha"),
        ("one-word", "alpha"),
        ("zero", "unrelated"),
    ] {
        conversation(&database, id, name, "owner", "GROUP", 0).await;
    }
    let ids = ranked_ids(&database, " alpha  beta gamma ignored ", 0, 50)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, id)| id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "prefix",
            "embedded",
            "sequential",
            "one-word",
            "reversed",
            "zero"
        ]
    );
    database.close().await;
}

#[tokio::test]
async fn escaped_wildcards_and_bound_quotes_keep_reference_like_semantics() {
    let (_directory, database) = database().await;
    for (id, name) in [
        ("percent", "100%"),
        ("underscore", "a_b"),
        ("quote", "O'Reilly"),
        ("ordinary", "abc"),
    ] {
        user(&database, id, name, id, "FRIEND", false).await;
    }
    assert_eq!(
        ranked_ids(&database, r"100\%", 0, 20).await.unwrap(),
        [(0, "percent".into())]
    );
    assert_eq!(
        ranked_ids(&database, r"a\_b", 0, 20).await.unwrap(),
        [(0, "underscore".into())]
    );
    assert_eq!(
        ranked_ids(&database, "O'Reilly", 0, 20).await.unwrap(),
        [(0, "quote".into())]
    );
    assert!(ranked_ids(&database, "' OR 1=1 --", 0, 20)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(ranked_ids(&database, "%", 0, 20).await.unwrap().len(), 4);
    database.close().await;
}

#[tokio::test]
async fn equal_scores_use_stable_ids_and_page_size_is_bounded() {
    let (_directory, database) = database().await;
    for index in (0..205).rev() {
        user(
            &database,
            &format!("user-{index:03}"),
            "same",
            &format!("{index:05}"),
            "FRIEND",
            false,
        )
        .await;
    }
    let first = ranked_ids(&database, "same", -10, 999).await.unwrap();
    assert_eq!(first.len(), 200);
    assert_eq!(first.first().unwrap().1, "user-000");
    assert_eq!(first.last().unwrap().1, "user-199");
    assert_eq!(
        ranked_ids(&database, "same", 200, 10).await.unwrap().len(),
        5
    );
    database.close().await;
}
