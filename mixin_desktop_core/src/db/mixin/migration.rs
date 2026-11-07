use futures::FutureExt;
use uuid::Uuid;

use crate::db::migration::{Migration, MigrationFuture, Migrator};

/// The single source of truth for migration order.
const MIGRATIONS: &[Migration] = &[
    Migration::action(3, "rebuild query indexes", migrate_to_v3),
    Migration::action(
        4,
        "recreate addresses and add message fields",
        migrate_to_v4,
    ),
    Migration::action(5, "add transcript messages", migrate_to_v5),
    Migration::action(6, "add pinned messages", migrate_to_v6),
    Migration::sql(
        7,
        "add fiat rates",
        "CREATE TABLE fiats (code TEXT NOT NULL, rate REAL NOT NULL, PRIMARY KEY(code))",
    ),
    Migration::sql(
        8,
        "remove the legacy update trigger",
        "DROP TRIGGER IF EXISTS conversation_last_message_update",
    ),
    Migration::sql(
        9,
        "add message status lookup index",
        "CREATE INDEX index_message_conversation_id_status_user_id \
         ON messages(conversation_id, status, user_id)",
    ),
    Migration::action(10, "extend sticker albums", migrate_to_v10),
    Migration::sql(
        11,
        "add favorite apps",
        "CREATE TABLE favorite_apps (app_id TEXT NOT NULL, user_id TEXT NOT NULL, \
         created_at INTEGER NOT NULL, PRIMARY KEY(app_id, user_id))",
    ),
    Migration::action(12, "add snapshot trace ids", migrate_to_v12),
    Migration::action(13, "add verified sticker albums", migrate_to_v13),
    Migration::action(14, "add expiring messages", migrate_to_v14),
    Migration::sql(
        15,
        "add message category index",
        "CREATE INDEX index_messages_conversation_id_category_created_at \
         ON messages(conversation_id, category, created_at DESC)",
    ),
    Migration::sql(
        16,
        "add user identity index",
        "CREATE INDEX index_users_identity_number ON users(identity_number)",
    ),
    Migration::action(17, "add user code fields", migrate_to_v17),
    Migration::sql(
        18,
        "add quoted message index",
        "CREATE INDEX index_messages_conversation_id_quote_message_id \
         ON messages(conversation_id, quote_message_id)",
    ),
    Migration::sql(
        19,
        "add chains",
        "CREATE TABLE chains (chain_id TEXT NOT NULL, name TEXT NOT NULL, symbol TEXT NOT NULL, \
         icon_url TEXT NOT NULL, threshold INTEGER NOT NULL, PRIMARY KEY(chain_id))",
    ),
    Migration::sql(
        20,
        "remove the legacy delete trigger",
        "DROP TRIGGER IF EXISTS conversation_last_message_delete",
    ),
    Migration::action(21, "schedule the FTS background migration", migrate_to_v21),
    Migration::action(22, "add snapshot balance fields", migrate_to_v22),
    Migration::sql(
        23,
        "add properties",
        "CREATE TABLE properties (\"key\" TEXT NOT NULL, \"group\" TEXT NOT NULL, \
         \"value\" TEXT NOT NULL, PRIMARY KEY(\"key\", \"group\"))",
    ),
    Migration::action(24, "add deactivated users", migrate_to_v24),
    Migration::action(25, "add safe snapshots and tokens", migrate_to_v25),
    Migration::action(26, "add inscriptions", migrate_to_v26),
    Migration::action(27, "add memberships", migrate_to_v27),
    Migration::action(28, "add token precision", migrate_to_v28),
    Migration::sql(
        29,
        "index scheduled message expiration",
        "CREATE INDEX IF NOT EXISTS index_expired_messages_expire_at \
         ON expired_messages (expire_at) WHERE expire_at IS NOT NULL",
    ),
    Migration::action(30, "repair binary audio waveforms", migrate_to_v30),
];

pub(crate) const SCHEMA_VERSION: i64 = 30;
pub(crate) const MIGRATOR: Migrator = Migrator::new(
    "mixin",
    SCHEMA_VERSION,
    include_str!("schema.sql"),
    MIGRATIONS,
);

fn migrate_to_v30(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        use base64ct::{Base64, Encoding};
        let mut cursor = None::<i64>;
        loop {
            let rows = crate::db::query_as::<(i64, Vec<u8>)>(
                "SELECT rowid, media_waveform FROM messages WHERE typeof(media_waveform) = 'blob' \
                 AND (? IS NULL OR rowid > ?) ORDER BY rowid ASC LIMIT 200",
            ).bind(cursor).bind(cursor).fetch_all(&mut *connection).await?;
            if rows.is_empty() { break; }
            for (row_id, waveform) in rows {
                crate::db::query("UPDATE messages SET media_waveform = ? WHERE rowid = ? AND typeof(media_waveform) = 'blob'")
                    .bind(Base64::encode_string(&waveform)).bind(row_id).execute(&mut *connection).await?;
                cursor = Some(row_id);
            }
        }
        Ok(())
    }.boxed()
}

fn migrate_to_v21(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::query(
            "INSERT INTO jobs (job_id, action, created_at, priority, run_count) \
             VALUES (?, 'LOCAL_MIGRATE_FTS', ?, 5, 0)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v3(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::raw_sql(
            r#"
DROP INDEX IF EXISTS index_conversations_category_status_pin_time_created_at;
DROP INDEX IF EXISTS index_participants_conversation_id;
DROP INDEX IF EXISTS index_participants_created_at;
DROP INDEX IF EXISTS index_users_full_name;
DROP INDEX IF EXISTS index_snapshots_asset_id;
DROP INDEX IF EXISTS index_messages_conversation_id_user_id_status_created_at;
DROP INDEX IF EXISTS index_messages_conversation_id_status_user_id;
DROP INDEX IF EXISTS index_conversations_pin_time_last_message_created_at;
DROP INDEX IF EXISTS index_messages_conversation_id_category;
DROP INDEX IF EXISTS index_messages_conversation_id_quote_message_id;
DROP INDEX IF EXISTS index_messages_conversation_id_status_user_id_created_at;
DROP INDEX IF EXISTS index_messages_conversation_id_created_at;
DROP INDEX IF EXISTS index_message_mentions_conversation_id;
DROP INDEX IF EXISTS index_users_relationship_full_name;
DROP INDEX IF EXISTS index_messages_conversation_id;
CREATE INDEX index_conversations_category_status ON conversations(category, status);
CREATE INDEX index_conversations_mute_until ON conversations(mute_until);
CREATE INDEX index_flood_messages_created_at ON flood_messages(created_at);
CREATE INDEX index_message_mentions_conversation_id_has_read ON message_mentions(conversation_id, has_read);
CREATE INDEX index_messages_conversation_id_created_at ON messages(conversation_id, created_at DESC);
CREATE INDEX index_participants_conversation_id_created_at ON participants(conversation_id, created_at);
CREATE INDEX index_sticker_albums_category_created_at ON sticker_albums(category, created_at DESC);
"#,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v4(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::raw_sql(
            r#"
DROP TABLE addresses;
CREATE TABLE addresses (
    address_id TEXT NOT NULL, type TEXT NOT NULL, asset_id TEXT NOT NULL,
    destination TEXT NOT NULL, label TEXT NOT NULL, updated_at INTEGER NOT NULL,
    reserve TEXT NOT NULL, fee TEXT NOT NULL, tag TEXT, dust TEXT,
    PRIMARY KEY(address_id)
);
"#,
        )
        .execute(&mut *connection)
        .await?;
        add_column(connection, "assets", "reserve", "TEXT").await?;
        add_column(connection, "messages", "caption", "TEXT").await
    }
    .boxed()
}

fn migrate_to_v5(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::raw_sql(
            r#"CREATE TABLE transcript_messages (
    transcript_id TEXT NOT NULL, message_id TEXT NOT NULL, user_id TEXT,
    user_full_name TEXT, category TEXT NOT NULL, created_at INTEGER NOT NULL,
    content TEXT, media_url TEXT, media_name TEXT, media_size INTEGER,
    media_width INTEGER, media_height INTEGER, media_mime_type TEXT,
    media_duration TEXT, media_status TEXT, media_waveform TEXT,
    thumb_image TEXT, thumb_url TEXT, media_key TEXT, media_digest TEXT,
    media_created_at INTEGER, sticker_id TEXT, shared_user_id TEXT, mentions TEXT,
    quote_id TEXT, quote_content TEXT, caption TEXT,
    PRIMARY KEY(transcript_id, message_id)
);"#,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v6(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::raw_sql(
            r#"CREATE TABLE pin_messages (
    message_id TEXT NOT NULL, conversation_id TEXT NOT NULL,
    created_at INTEGER NOT NULL, PRIMARY KEY(message_id)
);
CREATE INDEX index_pin_messages_conversation_id ON pin_messages(conversation_id);"#,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v10(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        add_column(
            connection,
            "sticker_albums",
            "ordered_at",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        add_column(connection, "sticker_albums", "banner", "TEXT").await?;
        add_column(
            connection,
            "sticker_albums",
            "added",
            "BOOLEAN DEFAULT FALSE",
        )
        .await?;
        crate::db::query("UPDATE sticker_albums SET added = TRUE")
            .execute(connection)
            .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v12(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    add_column_boxed(connection, "snapshots", "trace_id", "TEXT")
}

fn migrate_to_v13(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    add_column_boxed(
        connection,
        "sticker_albums",
        "is_verified",
        "BOOLEAN NOT NULL DEFAULT FALSE",
    )
}

fn migrate_to_v14(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "conversations", "expire_in", "INTEGER").await?;
        crate::db::query(
            "CREATE TABLE expired_messages (message_id TEXT NOT NULL, expire_in INTEGER NOT NULL, \
             expire_at INTEGER, PRIMARY KEY(message_id))",
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v17(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "users", "code_url", "TEXT").await?;
        add_column(connection, "users", "code_id", "TEXT").await
    }
    .boxed()
}

fn migrate_to_v22(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "snapshots", "snapshot_hash", "TEXT").await?;
        add_column(connection, "snapshots", "opening_balance", "TEXT").await?;
        add_column(connection, "snapshots", "closing_balance", "TEXT").await
    }
    .boxed()
}

fn migrate_to_v24(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    add_column_boxed(connection, "users", "is_deactivated", "BOOLEAN")
}

fn migrate_to_v25(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::raw_sql(
            r#"CREATE TABLE safe_snapshots (
    snapshot_id TEXT NOT NULL, type TEXT NOT NULL, asset_id TEXT NOT NULL,
    amount TEXT NOT NULL, user_id TEXT NOT NULL, opponent_id TEXT NOT NULL,
    memo TEXT NOT NULL, transaction_hash TEXT NOT NULL, created_at TEXT NOT NULL,
    trace_id TEXT, confirmations INTEGER, opening_balance TEXT, closing_balance TEXT,
    withdrawal TEXT, deposit TEXT, PRIMARY KEY(snapshot_id)
);
CREATE TABLE tokens (
    asset_id TEXT NOT NULL, kernel_asset_id TEXT NOT NULL, symbol TEXT NOT NULL,
    name TEXT NOT NULL, icon_url TEXT NOT NULL, price_btc TEXT NOT NULL,
    price_usd TEXT NOT NULL, chain_id TEXT NOT NULL, change_usd TEXT NOT NULL,
    change_btc TEXT NOT NULL, confirmations INTEGER NOT NULL, asset_key TEXT NOT NULL,
    dust TEXT NOT NULL, PRIMARY KEY(asset_id)
);
CREATE INDEX index_tokens_kernel_asset_id ON tokens(kernel_asset_id);"#,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v26(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "safe_snapshots", "inscription_hash", "TEXT").await?;
        add_column(connection, "tokens", "collection_hash", "TEXT").await?;
        crate::db::raw_sql(
            r#"CREATE INDEX index_tokens_collection_hash ON tokens(collection_hash);
CREATE TABLE inscription_collections (
    collection_hash TEXT NOT NULL, supply TEXT NOT NULL, unit TEXT NOT NULL,
    symbol TEXT NOT NULL, name TEXT NOT NULL, icon_url TEXT NOT NULL,
    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
    PRIMARY KEY(collection_hash)
);
CREATE TABLE inscription_items (
    inscription_hash TEXT NOT NULL, collection_hash TEXT NOT NULL,
    sequence INTEGER NOT NULL, content_type TEXT NOT NULL, content_url TEXT NOT NULL,
    occupied_by TEXT, occupied_at TEXT, created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL, PRIMARY KEY(inscription_hash)
);"#,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v27(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    add_column_boxed(connection, "users", "membership", "TEXT")
}

fn migrate_to_v28(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    add_column_boxed(
        connection,
        "tokens",
        "precision",
        "INTEGER NOT NULL DEFAULT -1",
    )
}

fn add_column_boxed<'a>(
    connection: &'a mut crate::db::Conn,
    table: &'static str,
    column: &'static str,
    definition: &'static str,
) -> MigrationFuture<'a> {
    async move { add_column(connection, table, column, definition).await }.boxed()
}

async fn add_column(
    connection: &mut crate::db::Conn,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let exists: bool =
        crate::db::query_scalar("SELECT EXISTS(SELECT 1 FROM pragma_table_info(?) WHERE name = ?)")
            .bind(table)
            .bind(column)
            .fetch_one(&mut *connection)
            .await?;
    if !exists {
        let statement = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
        crate::db::query(sqlx::AssertSqlSafe(statement))
            .execute(connection)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::db::mixin::MixinDatabase;

    #[tokio::test]
    async fn reopening_a_legacy_database_repairs_binary_waveforms_without_changing_text_or_jobs() {
        use base64ct::{Base64, Encoding};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixin.db");
        let database = super::super::MixinDatabase::connect_at(&path)
            .await
            .unwrap();
        let pool = &database.message_dao.0;
        crate::db::query("INSERT INTO conversations (conversation_id, created_at, status) VALUES ('conversation', 0, 0)").execute(pool).await.unwrap();
        for id in ["binary", "text"] {
            crate::db::query("INSERT INTO messages (message_id, conversation_id, user_id, category, status, created_at, media_waveform) VALUES (?, 'conversation', 'sender', 'SIGNAL_AUDIO', 'SENDING', 0, ?)")
                .bind(id).bind("unchanged").execute(pool).await.unwrap();
        }
        let waveform = [0_u8, 128, 255];
        crate::db::query("UPDATE messages SET media_waveform = ? WHERE message_id = 'binary'")
            .bind(waveform.to_vec())
            .execute(pool)
            .await
            .unwrap();
        let job = crate::db::mixin::job::Job::create_sending_job(
            "binary",
            "conversation",
            None,
            None,
            false,
            false,
            0,
        );
        database.job_dao.insert_job(&job).await.unwrap();
        crate::db::query("PRAGMA user_version = 29")
            .execute(pool)
            .await
            .unwrap();
        pool.close().await;
        drop(database);
        let reopened = super::super::MixinDatabase::connect_at(&path)
            .await
            .unwrap();
        let row = reopened
            .message_dao
            .find_message_by_id(&"binary".to_owned())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            Base64::decode_vec(row.media_waveform.as_deref().unwrap()).unwrap(),
            waveform
        );
        assert_eq!(
            reopened
                .message_dao
                .find_message_by_id(&"text".to_owned())
                .await
                .unwrap()
                .unwrap()
                .media_waveform
                .as_deref(),
            Some("unchanged")
        );
        let count: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
            .bind(job.job_id)
            .fetch_one(&reopened.message_dao.0)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_registry_is_ordered_and_current() {
        MIGRATOR.validate().unwrap();
    }

    #[tokio::test]
    async fn new_v28_and_flutter_v29_databases_open_with_indexed_expiration_lookup() {
        for source_version in [0, 28, 29] {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            if source_version != 0 {
                crate::db::raw_sql(include_str!("schema.sql"))
                    .execute(&pool)
                    .await
                    .unwrap();
                crate::db::query(
                    "INSERT INTO expired_messages VALUES ('due', 60, 100), ('pending', 60, NULL)",
                )
                .execute(&pool)
                .await
                .unwrap();
                if source_version == 28 {
                    crate::db::query("DROP INDEX IF EXISTS index_expired_messages_expire_at")
                        .execute(&pool)
                        .await
                        .unwrap();
                }
                if source_version == 29 {
                    crate::db::raw_sql(
                        "CREATE INDEX IF NOT EXISTS index_expired_messages_expire_at \
                         ON expired_messages (expire_at) WHERE expire_at IS NOT NULL; \
                         PRAGMA user_version = 29;",
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                } else {
                    crate::db::query("PRAGMA user_version = 28")
                        .execute(&pool)
                        .await
                        .unwrap();
                }
            }

            MIGRATOR.migrate(&pool).await.unwrap();
            let version: i64 = crate::db::query_scalar("PRAGMA user_version")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(version, SCHEMA_VERSION);
            if source_version == 0 {
                crate::db::query(
                    "INSERT INTO expired_messages VALUES ('due', 60, 100), ('pending', 60, NULL)",
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            let due: Vec<String> = crate::db::query_scalar(
                "SELECT message_id FROM expired_messages \
                 INDEXED BY index_expired_messages_expire_at WHERE expire_at <= 100",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            assert_eq!(due, ["due"]);
            let count: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM expired_messages")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 2);
            pool.close().await;
        }
    }

    #[tokio::test]
    async fn upgrades_flutter_v1_database_to_current_version() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixin.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        crate::db::raw_sql(include_str!("../../../tests/fixtures/mixin_v1.sql"))
            .execute(&pool)
            .await
            .unwrap();
        crate::db::raw_sql(
            "PRAGMA user_version = 1; \
             INSERT INTO users (user_id, identity_number, full_name) \
             VALUES ('legacy-user', '7000', 'Legacy User');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let database = MixinDatabase::connect_at(&path).await.unwrap();
        let version: i64 = crate::db::query_scalar("PRAGMA user_version")
            .fetch_one(&database.user_dao.0)
            .await
            .unwrap();
        let membership: Option<String> =
            crate::db::query_scalar("SELECT membership FROM users WHERE user_id = 'legacy-user'")
                .fetch_one(&database.user_dao.0)
                .await
                .unwrap();
        let migration_job: i64 =
            crate::db::query_scalar("SELECT COUNT(*) FROM jobs WHERE action = 'LOCAL_MIGRATE_FTS'")
                .fetch_one(&database.user_dao.0)
                .await
                .unwrap();
        let sqlx_table: bool = crate::db::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = '_sqlx_migrations')",
        )
        .fetch_one(&database.user_dao.0)
        .await
        .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(membership, None);
        assert_eq!(migration_job, 1);
        assert!(!sqlx_table);
    }
}
