use anyhow::{bail, Context};
use futures::{future::BoxFuture, FutureExt};
use sqlx::{Pool, Sqlite, SqliteConnection};
use uuid::Uuid;

pub(crate) const SCHEMA_VERSION: i64 = 28;

type MigrationFuture<'a> = BoxFuture<'a, anyhow::Result<()>>;
type MigrationAction = for<'a> fn(&'a mut SqliteConnection) -> MigrationFuture<'a>;

/// One command in the ordered database migration pipeline.
struct Migration {
    target_version: i64,
    description: &'static str,
    apply: MigrationAction,
}

impl Migration {
    const fn new(target_version: i64, description: &'static str, apply: MigrationAction) -> Self {
        Self {
            target_version,
            description,
            apply,
        }
    }
}

/// The single source of truth for migration order.
const MIGRATIONS: &[Migration] = &[
    Migration::new(3, "rebuild query indexes", migrate_to_v3),
    Migration::new(
        4,
        "recreate addresses and add message fields",
        migrate_to_v4,
    ),
    Migration::new(5, "add transcript messages", migrate_to_v5),
    Migration::new(6, "add pinned messages", migrate_to_v6),
    Migration::new(7, "add fiat rates", migrate_to_v7),
    Migration::new(8, "remove the legacy update trigger", migrate_to_v8),
    Migration::new(9, "add message status lookup index", migrate_to_v9),
    Migration::new(10, "extend sticker albums", migrate_to_v10),
    Migration::new(11, "add favorite apps", migrate_to_v11),
    Migration::new(12, "add snapshot trace ids", migrate_to_v12),
    Migration::new(13, "add verified sticker albums", migrate_to_v13),
    Migration::new(14, "add expiring messages", migrate_to_v14),
    Migration::new(15, "add message category index", migrate_to_v15),
    Migration::new(16, "add user identity index", migrate_to_v16),
    Migration::new(17, "add user code fields", migrate_to_v17),
    Migration::new(18, "add quoted message index", migrate_to_v18),
    Migration::new(19, "add chains", migrate_to_v19),
    Migration::new(20, "remove the legacy delete trigger", migrate_to_v20),
    Migration::new(21, "schedule the FTS background migration", migrate_to_v21),
    Migration::new(22, "add snapshot balance fields", migrate_to_v22),
    Migration::new(23, "add properties", migrate_to_v23),
    Migration::new(24, "add deactivated users", migrate_to_v24),
    Migration::new(25, "add safe snapshots and tokens", migrate_to_v25),
    Migration::new(26, "add inscriptions", migrate_to_v26),
    Migration::new(27, "add memberships", migrate_to_v27),
    Migration::new(28, "add token precision", migrate_to_v28),
];

pub(crate) async fn migrate(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    validate_registry()?;
    let source_version = crate::db::migration::user_version(pool).await?;
    if source_version > SCHEMA_VERSION {
        bail!("mixin database version {source_version} is newer than supported {SCHEMA_VERSION}");
    }
    if source_version == SCHEMA_VERSION {
        return Ok(());
    }
    if source_version == 0 {
        return create_current_schema(pool).await;
    }

    run_migrations(pool, source_version).await
}

fn validate_registry() -> anyhow::Result<()> {
    let mut previous = 0;
    for migration in MIGRATIONS {
        if migration.target_version <= previous {
            bail!("mixin migrations must be ordered by target version");
        }
        previous = migration.target_version;
    }
    if previous != SCHEMA_VERSION {
        bail!("latest mixin migration is v{previous}, but schema version is v{SCHEMA_VERSION}");
    }
    Ok(())
}

async fn create_current_schema(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    if crate::db::migration::has_application_tables(pool).await? {
        bail!("mixin database has no Drift user_version");
    }

    let mut transaction = pool.begin().await?;
    sqlx::raw_sql(include_str!("schema.sql"))
        .execute(&mut *transaction)
        .await?;
    set_current_version(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}

async fn run_migrations(pool: &Pool<Sqlite>, source_version: i64) -> anyhow::Result<()> {
    let mut transaction = pool.begin().await?;
    for migration in MIGRATIONS
        .iter()
        .filter(|migration| migration.target_version > source_version)
    {
        log::info!(
            "Migrating mixin database to v{}: {}",
            migration.target_version,
            migration.description
        );
        (migration.apply)(&mut transaction).await.with_context(|| {
            format!(
                "migrate mixin database to v{}: {}",
                migration.target_version, migration.description
            )
        })?;
    }
    set_current_version(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}

async fn set_current_version(connection: &mut SqliteConnection) -> anyhow::Result<()> {
    let statement = format!("PRAGMA user_version = {SCHEMA_VERSION}");
    sqlx::query(sqlx::AssertSqlSafe(statement))
        .execute(connection)
        .await?;
    Ok(())
}

fn migrate_to_v21(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        sqlx::query(
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

fn migrate_to_v3(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        sqlx::raw_sql(
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

fn migrate_to_v4(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        sqlx::raw_sql(
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

fn migrate_to_v5(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        sqlx::raw_sql(
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

fn migrate_to_v6(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        sqlx::raw_sql(
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

fn migrate_to_v7(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE TABLE fiats (code TEXT NOT NULL, rate REAL NOT NULL, PRIMARY KEY(code))",
    )
}

fn migrate_to_v8(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "DROP TRIGGER IF EXISTS conversation_last_message_update",
    )
}

fn migrate_to_v9(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE INDEX index_message_conversation_id_status_user_id \
         ON messages(conversation_id, status, user_id)",
    )
}

fn migrate_to_v10(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
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
        sqlx::query("UPDATE sticker_albums SET added = TRUE")
            .execute(connection)
            .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v11(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE TABLE favorite_apps (app_id TEXT NOT NULL, user_id TEXT NOT NULL, \
         created_at INTEGER NOT NULL, PRIMARY KEY(app_id, user_id))",
    )
}

fn migrate_to_v12(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    add_column_boxed(connection, "snapshots", "trace_id", "TEXT")
}

fn migrate_to_v13(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    add_column_boxed(
        connection,
        "sticker_albums",
        "is_verified",
        "BOOLEAN NOT NULL DEFAULT FALSE",
    )
}

fn migrate_to_v14(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "conversations", "expire_in", "INTEGER").await?;
        sqlx::query(
            "CREATE TABLE expired_messages (message_id TEXT NOT NULL, expire_in INTEGER NOT NULL, \
             expire_at INTEGER, PRIMARY KEY(message_id))",
        )
        .execute(connection)
        .await?;
        Ok(())
    }
    .boxed()
}

fn migrate_to_v15(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE INDEX index_messages_conversation_id_category_created_at \
         ON messages(conversation_id, category, created_at DESC)",
    )
}

fn migrate_to_v16(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE INDEX index_users_identity_number ON users(identity_number)",
    )
}

fn migrate_to_v17(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "users", "code_url", "TEXT").await?;
        add_column(connection, "users", "code_id", "TEXT").await
    }
    .boxed()
}

fn migrate_to_v18(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE INDEX index_messages_conversation_id_quote_message_id \
         ON messages(conversation_id, quote_message_id)",
    )
}

fn migrate_to_v19(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE TABLE chains (chain_id TEXT NOT NULL, name TEXT NOT NULL, symbol TEXT NOT NULL, \
         icon_url TEXT NOT NULL, threshold INTEGER NOT NULL, PRIMARY KEY(chain_id))",
    )
}

fn migrate_to_v20(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "DROP TRIGGER IF EXISTS conversation_last_message_delete",
    )
}

fn migrate_to_v22(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "snapshots", "snapshot_hash", "TEXT").await?;
        add_column(connection, "snapshots", "opening_balance", "TEXT").await?;
        add_column(connection, "snapshots", "closing_balance", "TEXT").await
    }
    .boxed()
}

fn migrate_to_v23(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    execute(
        connection,
        "CREATE TABLE properties (\"key\" TEXT NOT NULL, \"group\" TEXT NOT NULL, \
         \"value\" TEXT NOT NULL, PRIMARY KEY(\"key\", \"group\"))",
    )
}

fn migrate_to_v24(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    add_column_boxed(connection, "users", "is_deactivated", "BOOLEAN")
}

fn migrate_to_v25(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        sqlx::raw_sql(
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

fn migrate_to_v26(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    async move {
        add_column(connection, "safe_snapshots", "inscription_hash", "TEXT").await?;
        add_column(connection, "tokens", "collection_hash", "TEXT").await?;
        sqlx::raw_sql(
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

fn migrate_to_v27(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    add_column_boxed(connection, "users", "membership", "TEXT")
}

fn migrate_to_v28(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
    add_column_boxed(
        connection,
        "tokens",
        "precision",
        "INTEGER NOT NULL DEFAULT -1",
    )
}

fn execute<'a>(connection: &'a mut SqliteConnection, sql: &'static str) -> MigrationFuture<'a> {
    async move {
        sqlx::query(sql).execute(connection).await?;
        Ok(())
    }
    .boxed()
}

fn add_column_boxed<'a>(
    connection: &'a mut SqliteConnection,
    table: &'static str,
    column: &'static str,
    definition: &'static str,
) -> MigrationFuture<'a> {
    async move { add_column(connection, table, column, definition).await }.boxed()
}

async fn add_column(
    connection: &mut SqliteConnection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pragma_table_info(?) WHERE name = ?)")
            .bind(table)
            .bind(column)
            .fetch_one(&mut *connection)
            .await?;
    if !exists {
        let statement = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
        sqlx::query(sqlx::AssertSqlSafe(statement))
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

    #[test]
    fn migration_registry_is_ordered_and_current() {
        validate_registry().unwrap();
    }

    #[tokio::test]
    async fn upgrades_flutter_v1_database_to_v28() {
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
        sqlx::raw_sql(include_str!("../../../tests/fixtures/mixin_v1.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(
            "PRAGMA user_version = 1; \
             INSERT INTO users (user_id, identity_number, full_name) \
             VALUES ('legacy-user', '7000', 'Legacy User');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let database = MixinDatabase::connect_at(&path).await.unwrap();
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&database.user_dao.0)
            .await
            .unwrap();
        let membership: Option<String> =
            sqlx::query_scalar("SELECT membership FROM users WHERE user_id = 'legacy-user'")
                .fetch_one(&database.user_dao.0)
                .await
                .unwrap();
        let migration_job: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE action = 'LOCAL_MIGRATE_FTS'")
                .fetch_one(&database.user_dao.0)
                .await
                .unwrap();
        let sqlx_table: bool = sqlx::query_scalar(
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
