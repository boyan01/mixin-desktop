use anyhow::{bail, Context};
use futures::future::BoxFuture;
use sqlx::{Pool, Sqlite, SqliteConnection};

pub(crate) type MigrationFuture<'a> = BoxFuture<'a, anyhow::Result<()>>;
type MigrationAction = for<'a> fn(&'a mut SqliteConnection) -> MigrationFuture<'a>;

#[derive(Clone, Copy)]
enum MigrationKind {
    Sql(&'static str),
    Action(MigrationAction),
}

/// One command in an ordered SQLite database migration pipeline.
#[derive(Clone, Copy)]
pub(crate) struct Migration {
    target_version: i64,
    description: &'static str,
    kind: MigrationKind,
}

impl Migration {
    pub(crate) const fn action(
        target_version: i64,
        description: &'static str,
        apply: MigrationAction,
    ) -> Self {
        Self {
            target_version,
            description,
            kind: MigrationKind::Action(apply),
        }
    }

    pub(crate) const fn sql(
        target_version: i64,
        description: &'static str,
        sql: &'static str,
    ) -> Self {
        Self {
            target_version,
            description,
            kind: MigrationKind::Sql(sql),
        }
    }

    async fn apply(&self, connection: &mut SqliteConnection) -> anyhow::Result<()> {
        match self.kind {
            MigrationKind::Sql(sql) => {
                sqlx::raw_sql(sql).execute(connection).await?;
                Ok(())
            }
            MigrationKind::Action(apply) => apply(connection).await,
        }
    }
}

/// Reusable SQLite migration pipeline.
///
/// New databases are created directly from `schema`. Existing databases run
/// every registered migration newer than their `PRAGMA user_version` inside a
/// single transaction.
pub(crate) struct Migrator {
    database_name: &'static str,
    schema_version: i64,
    schema: &'static str,
    migrations: &'static [Migration],
}

impl Migrator {
    pub(crate) const fn new(
        database_name: &'static str,
        schema_version: i64,
        schema: &'static str,
        migrations: &'static [Migration],
    ) -> Self {
        Self {
            database_name,
            schema_version,
            schema,
            migrations,
        }
    }

    pub(crate) async fn migrate(&self, pool: &Pool<Sqlite>) -> anyhow::Result<()> {
        self.validate()?;
        let source_version = user_version(pool).await?;
        if source_version > self.schema_version {
            bail!(
                "{} database version {source_version} is newer than supported {}",
                self.database_name,
                self.schema_version
            );
        }
        if source_version == self.schema_version {
            return Ok(());
        }
        if source_version == 0 {
            return self.create_current_schema(pool).await;
        }
        self.run_migrations(pool, source_version).await
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if self.schema_version < 1 {
            bail!(
                "{} database schema version must be positive",
                self.database_name
            );
        }
        let mut previous = 0;
        for migration in self.migrations {
            if migration.target_version <= previous {
                bail!(
                    "{} migrations must be ordered by target version",
                    self.database_name
                );
            }
            if migration.target_version > self.schema_version {
                bail!(
                    "{} migration v{} exceeds schema version {}",
                    self.database_name,
                    migration.target_version,
                    self.schema_version
                );
            }
            previous = migration.target_version;
        }
        if self.migrations.is_empty() {
            if self.schema_version != 1 {
                bail!(
                    "{} database schema is v{}, but no migrations are registered",
                    self.database_name,
                    self.schema_version
                );
            }
        } else if previous != self.schema_version {
            bail!(
                "latest {} migration is v{previous}, but schema version is v{}",
                self.database_name,
                self.schema_version
            );
        }
        Ok(())
    }

    async fn create_current_schema(&self, pool: &Pool<Sqlite>) -> anyhow::Result<()> {
        if has_application_tables(pool).await? {
            bail!("{} database has no Drift user_version", self.database_name);
        }

        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::raw_sql(self.schema)
            .execute(&mut *transaction)
            .await
            .with_context(|| {
                format!(
                    "create {} database schema v{}",
                    self.database_name, self.schema_version
                )
            })?;
        set_user_version(&mut transaction, self.schema_version).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn run_migrations(&self, pool: &Pool<Sqlite>, source_version: i64) -> anyhow::Result<()> {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        for migration in self
            .migrations
            .iter()
            .filter(|migration| migration.target_version > source_version)
        {
            log::info!(
                "Migrating {} database to v{}: {}",
                self.database_name,
                migration.target_version,
                migration.description
            );
            migration.apply(&mut transaction).await.with_context(|| {
                format!(
                    "migrate {} database to v{}: {}",
                    self.database_name, migration.target_version, migration.description
                )
            })?;
        }
        set_user_version(&mut transaction, self.schema_version).await?;
        transaction.commit().await?;
        Ok(())
    }
}

async fn user_version(pool: &Pool<Sqlite>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
}

async fn has_application_tables(pool: &Pool<Sqlite>) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%')",
    )
    .fetch_one(pool)
    .await
}

async fn set_user_version(connection: &mut SqliteConnection, version: i64) -> anyhow::Result<()> {
    let statement = format!("PRAGMA user_version = {version}");
    sqlx::query(sqlx::AssertSqlSafe(statement))
        .execute(connection)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    const CURRENT_SCHEMA: &str =
        "CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT, enabled INTEGER NOT NULL);";
    const MIGRATIONS: &[Migration] = &[
        Migration::sql(2, "add value", "ALTER TABLE records ADD COLUMN value TEXT"),
        Migration::action(3, "add enabled", add_enabled),
    ];
    const MIGRATOR: Migrator = Migrator::new("test", 3, CURRENT_SCHEMA, MIGRATIONS);

    fn add_enabled(connection: &mut SqliteConnection) -> MigrationFuture<'_> {
        async move {
            sqlx::query("ALTER TABLE records ADD COLUMN enabled INTEGER NOT NULL DEFAULT 0")
                .execute(connection)
                .await?;
            Ok(())
        }
        .boxed()
    }

    async fn pool() -> Pool<Sqlite> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn empty_database_uses_current_schema_without_replaying_migrations() {
        let pool = pool().await;

        MIGRATOR.migrate(&pool).await.unwrap();

        assert_eq!(user_version(&pool).await.unwrap(), 3);
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('records')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(columns, vec!["id", "value", "enabled"]);
    }

    #[tokio::test]
    async fn existing_database_runs_newer_migrations_in_order() {
        let pool = pool().await;
        sqlx::raw_sql("CREATE TABLE records (id INTEGER PRIMARY KEY); PRAGMA user_version = 1;")
            .execute(&pool)
            .await
            .unwrap();

        MIGRATOR.migrate(&pool).await.unwrap();

        assert_eq!(user_version(&pool).await.unwrap(), 3);
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('records')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(columns, vec!["id", "value", "enabled"]);
    }

    #[tokio::test]
    async fn failed_pipeline_rolls_back_all_migrations_and_version() {
        const FAILING_MIGRATIONS: &[Migration] = &[
            Migration::sql(2, "add value", "ALTER TABLE records ADD COLUMN value TEXT"),
            Migration::sql(3, "fail", "ALTER TABLE missing ADD COLUMN value TEXT"),
        ];
        const FAILING_MIGRATOR: Migrator =
            Migrator::new("failing", 3, CURRENT_SCHEMA, FAILING_MIGRATIONS);
        let pool = pool().await;
        sqlx::raw_sql("CREATE TABLE records (id INTEGER PRIMARY KEY); PRAGMA user_version = 1;")
            .execute(&pool)
            .await
            .unwrap();

        assert!(FAILING_MIGRATOR.migrate(&pool).await.is_err());

        assert_eq!(user_version(&pool).await.unwrap(), 1);
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('records')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(columns, vec!["id"]);
    }

    #[tokio::test]
    async fn database_newer_than_supported_is_rejected_without_changes() {
        let pool = pool().await;
        sqlx::raw_sql("CREATE TABLE records (id INTEGER PRIMARY KEY); PRAGMA user_version = 4;")
            .execute(&pool)
            .await
            .unwrap();

        let error = MIGRATOR.migrate(&pool).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("test database version 4 is newer than supported 3"));
        assert_eq!(user_version(&pool).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn unversioned_database_with_tables_is_rejected_without_overwriting_schema() {
        let pool = pool().await;
        sqlx::query("CREATE TABLE legacy (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        let error = MIGRATOR.migrate(&pool).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("test database has no Drift user_version"));
        let legacy_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'legacy')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(legacy_exists);
    }

    #[test]
    fn registry_must_be_ordered_and_end_at_current_version() {
        const INVALID: Migrator = Migrator::new(
            "invalid",
            3,
            CURRENT_SCHEMA,
            &[
                Migration::sql(3, "three", "SELECT 1"),
                Migration::sql(2, "two", "SELECT 1"),
            ],
        );

        assert!(INVALID.validate().is_err());
    }
}
