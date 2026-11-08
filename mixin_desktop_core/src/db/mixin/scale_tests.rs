//! Timeline and conversation-list queries over account-sized data: tens of
//! thousands of messages with long texts, and thousands of conversations.
//! Results must stay exact at that size, and the hot queries must stay on
//! indexes, so a jump or a page costs the same at row 20,000 as at row 20.

use std::time::Instant;

use super::MixinDatabase;

const MESSAGES: i64 = 20_000;
const UNREAD: i64 = 3_000;
const CONVERSATIONS: i64 = 10_000;
/// The window a timeline jump reads on each side of its target (`HALF` in the UI).
const HALF: i64 = 99;

async fn database() -> (tempfile::TempDir, MixinDatabase) {
    let directory = tempfile::tempdir().unwrap();
    let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
        .await
        .unwrap();
    (directory, database)
}

fn id(n: i64) -> String {
    format!("big-{n:05}")
}

/// `big` holds `MESSAGES` rows, oldest first: every seventh a ~10 KB text,
/// pairs sharing a timestamp (ordered by rowid), every 101st after the first
/// 5,000 quoting the row 5,000 older, and the newest `UNREAD` from others
/// still unread. `noise` interleaves rows that must never leak in.
async fn seed_messages(database: &MixinDatabase) {
    let pool = &database.message_dao.0;
    crate::db::query(
        "INSERT INTO conversations (conversation_id, category, created_at, status) \
         VALUES ('big', 'GROUP', 0, 2), ('noise', 'GROUP', 0, 2)",
    )
    .execute(pool)
    .await
    .unwrap();
    crate::db::query(
        "WITH RECURSIVE seq(n) AS (SELECT 0 UNION ALL SELECT n + 1 FROM seq WHERE n + 1 < ?1) \
         INSERT INTO messages (message_id, conversation_id, user_id, category, content, status, \
                               created_at, quote_message_id, quote_content) \
         SELECT printf('big-%05d', n), 'big', \
                CASE WHEN n % 3 = 0 THEN 'me' ELSE printf('user-%d', n % 17) END, \
                'PLAIN_TEXT', \
                CASE WHEN n % 7 = 0 \
                     THEN printf('%d ', n) || replace(hex(zeroblob(1000)), '00', 'long text ') \
                     ELSE printf('message %d', n) END, \
                CASE WHEN n >= ?1 - ?2 AND n % 3 != 0 THEN 'DELIVERED' ELSE 'READ' END, \
                1700000000000 + (n / 2) * 1000, \
                CASE WHEN n % 101 = 0 AND n >= 5000 THEN printf('big-%05d', n - 5000) END, \
                CASE WHEN n % 101 = 0 AND n >= 5000 \
                     THEN json_object('message_id', printf('big-%05d', n - 5000)) END \
         FROM seq",
    )
    .bind(MESSAGES)
    .bind(UNREAD)
    .execute(pool)
    .await
    .unwrap();
    crate::db::query(
        "WITH RECURSIVE seq(n) AS (SELECT 0 UNION ALL SELECT n + 1 FROM seq WHERE n + 1 < 5000) \
         INSERT INTO messages (message_id, conversation_id, user_id, category, content, status, \
                               created_at) \
         SELECT printf('noise-%05d', n), 'noise', 'user-1', 'PLAIN_TEXT', 'noise', 'DELIVERED', \
                1700000000000 + n * 4000 + 1 \
         FROM seq",
    )
    .execute(pool)
    .await
    .unwrap();
}

fn timed<T>(label: &str, started: Instant, value: T) -> T {
    // Reported for `--nocapture` runs; the assertions are on rows and plans.
    eprintln!("{label}: {:?}", started.elapsed());
    value
}

/// The steps of `sql`'s query plan that scan a table instead of searching an
/// index. A query's own bounded rows (`LIMIT` subqueries, `message_window`,
/// constants) may be scanned.
fn plan(sql: &str) -> crate::db::storage::Query<(i64, i64, i64, String)> {
    crate::db::query_as(sqlx::AssertSqlSafe(format!("EXPLAIN QUERY PLAN {sql}")))
}

fn scans(steps: Vec<(i64, i64, i64, String)>) -> Vec<String> {
    steps
        .into_iter()
        .map(|(_, _, _, detail)| detail)
        .filter(|detail| {
            detail.starts_with("SCAN ")
                && !detail.starts_with("SCAN (")
                && !detail.starts_with("SCAN message_window")
                && detail != "SCAN CONSTANT ROW"
        })
        .collect()
}

#[tokio::test]
async fn far_jump_reads_a_contiguous_window_around_a_message_twenty_thousand_rows_back() {
    let (_directory, database) = database().await;
    seed_messages(&database).await;
    let dao = &database.message_dao;

    for target in [137, MESSAGES / 2, MESSAGES - 2] {
        let started = Instant::now();
        let rows = timed(
            &format!("around {}", id(target)),
            started,
            dao.list_items_around("big", &id(target), HALF, HALF)
                .await
                .unwrap(),
        );
        let expected = (target - HALF).max(0)..=(target + HALF).min(MESSAGES - 1);
        assert_eq!(
            rows.iter()
                .map(|row| row.message_id.clone())
                .collect::<Vec<_>>(),
            expected.map(id).collect::<Vec<_>>(),
            "the window around {target} is its neighbors, oldest first"
        );
    }
    // A quote jumps 5,000 rows back to the quoted message.
    let quoting = dao
        .list_items_around("big", &id(19_998), 0, 0)
        .await
        .unwrap();
    assert_eq!(
        quoting[0].quote_message_id.as_deref(),
        Some(id(14_998).as_str())
    );
    let quoted = dao
        .list_items_around("big", &id(14_998), HALF, HALF)
        .await
        .unwrap();
    assert_eq!(quoted.len() as i64, HALF * 2 + 1);
    assert_eq!(quoted[HALF as usize].message_id, id(14_998));
    // Long rows are read whole.
    let long = &dao
        .list_items_around("big", &id(7_000), 0, 0)
        .await
        .unwrap()[0];
    assert!(long.content.as_deref().unwrap().len() > 10_000);
}

#[tokio::test]
async fn paging_older_from_the_latest_rows_visits_every_message_once_in_order() {
    let (_directory, database) = database().await;
    seed_messages(&database).await;
    let dao = &database.message_dao;

    let started = Instant::now();
    let mut ids = dao
        .list_items("big", None, None, 100)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.message_id)
        .collect::<Vec<_>>();
    let mut pages = 0;
    loop {
        let anchor = ids.last().unwrap().clone();
        let page = dao
            .list_items_relative("big", &anchor, true, 100)
            .await
            .unwrap();
        if page.is_empty() {
            break;
        }
        pages += 1;
        ids.extend(page.into_iter().map(|row| row.message_id));
    }
    timed(&format!("{pages} older pages"), started, ());
    assert_eq!(ids, (0..MESSAGES).rev().map(id).collect::<Vec<_>>());

    // And back toward the latest rows from the oldest.
    let mut newer = vec![id(0)];
    for _ in 0..20 {
        let page = dao
            .list_items_relative("big", &newer[0], false, 100)
            .await
            .unwrap();
        newer = page
            .into_iter()
            .map(|row| row.message_id)
            .chain(newer)
            .collect();
    }
    assert_eq!(newer, (0..=2_000).rev().map(id).collect::<Vec<_>>());
}

#[tokio::test]
async fn first_unread_message_is_the_oldest_unread_row_from_others() {
    let (_directory, database) = database().await;
    seed_messages(&database).await;

    let started = Instant::now();
    let first = timed(
        "first unread",
        started,
        database
            .message_dao
            .first_unread_message_id("big", "me")
            .await
            .unwrap(),
    );
    let expected = (MESSAGES - UNREAD..MESSAGES).find(|n| n % 3 != 0).unwrap();
    assert_eq!(first, Some(id(expected)));
}

#[tokio::test]
async fn timeline_queries_search_indexes_at_scale() {
    let (_directory, database) = database().await;
    seed_messages(&database).await;
    crate::db::query("ANALYZE")
        .execute(&database.message_dao.0)
        .await
        .unwrap();

    let pool = &database.message_dao.0;
    let around = plan(super::message::LIST_ITEMS_AROUND_QUERY)
        .bind("big")
        .bind(id(137))
        .bind(1_700_000_068_000_i64)
        .bind(138_i64)
        .bind(HALF)
        .bind(HALF)
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(scans(around), Vec::<String>::new());
    let unread = plan(super::message::FIRST_UNREAD_MESSAGE_QUERY)
        .bind("big")
        .bind("me")
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(scans(unread), Vec::<String>::new());
    // The oracle sees a scan where there is one.
    let unindexed = plan("SELECT message_id FROM messages WHERE content = ?")
        .bind("x")
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(scans(unindexed), ["SCAN messages"]);
}

/// `CONVERSATIONS` chats, every fifth unread and every 97th pinned, each with
/// a last message, newest first by `last_message_created_at`.
async fn seed_conversations(database: &MixinDatabase) {
    let pool = &database.conversation_dao.0;
    crate::db::query(
        "WITH RECURSIVE seq(n) AS (SELECT 0 UNION ALL SELECT n + 1 FROM seq WHERE n + 1 < ?1) \
         INSERT INTO conversations (conversation_id, category, name, created_at, status, \
                                    last_message_id, last_message_created_at, \
                                    unseen_message_count, draft, pin_time) \
         SELECT printf('chat-%05d', n), 'GROUP', printf('Group %d', n), 0, 2, \
                printf('last-%05d', n), 1700000000000 + n * 1000, \
                CASE WHEN n % 5 = 0 THEN n % 50 + 1 ELSE 0 END, '', \
                CASE WHEN n % 97 = 0 THEN 1800000000000 + n END \
         FROM seq",
    )
    .bind(CONVERSATIONS)
    .execute(pool)
    .await
    .unwrap();
    crate::db::query(
        "INSERT INTO messages (message_id, conversation_id, user_id, category, content, status, \
                               created_at) \
         SELECT last_message_id, conversation_id, 'user-1', 'PLAIN_TEXT', \
                'a last message long enough to be cut ' || name, 'READ', last_message_created_at \
         FROM conversations",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn conversation_pages_and_positions_stay_exact_across_ten_thousand_chats() {
    let (_directory, database) = database().await;
    seed_conversations(&database).await;
    let dao = &database.conversation_dao;

    let pinned = (0..CONVERSATIONS).rev().filter(|n| n % 97 == 0);
    let rest = (0..CONVERSATIONS).rev().filter(|n| n % 97 != 0);
    let expected = pinned
        .chain(rest)
        .map(|n| format!("chat-{n:05}"))
        .collect::<Vec<_>>();
    for offset in [0, 5_000, CONVERSATIONS - 100] {
        let started = Instant::now();
        let page = timed(
            &format!("conversations at {offset}"),
            started,
            dao.list_items("chats", None, "", false, 100, offset)
                .await
                .unwrap(),
        );
        assert_eq!(
            page.iter()
                .map(|row| row.conversation_id.clone())
                .collect::<Vec<_>>(),
            expected[offset as usize..offset as usize + 100]
        );
    }
    let deep = &expected[9_876];
    let started = Instant::now();
    let position = timed(
        "conversation position",
        started,
        dao.item_position("chats", None, "", false, deep)
            .await
            .unwrap(),
    );
    assert_eq!(position, Some(9_876));
    let started = Instant::now();
    let unread = timed(
        "unread conversations",
        started,
        dao.count_items("chats", None, "", true).await.unwrap(),
    );
    assert_eq!(unread, CONVERSATIONS / 5);
}
