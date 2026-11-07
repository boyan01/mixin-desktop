#[cfg(target_arch = "wasm32")]
use futures::FutureExt;

#[cfg(target_arch = "wasm32")]
use crate::db::migration::MigrationFuture;
use crate::db::migration::{Migration, Migrator};

/// Native signal.db stays at the Flutter app's schema version so both apps
/// can open it. Native Signal counters live in the shared Hive boxes.
#[cfg(not(target_arch = "wasm32"))]
const MIGRATIONS: &[Migration] = &[];
#[cfg(not(target_arch = "wasm32"))]
pub(super) const SCHEMA_VERSION: i64 = 1;
#[cfg(not(target_arch = "wasm32"))]
const SCHEMA: &str = include_str!("schema.sql");

/// wasm has no Hive store, so it keeps Signal counters in `properties`.
#[cfg(target_arch = "wasm32")]
const MIGRATIONS: &[Migration] = &[Migration::action(2, "add signal properties", migrate_to_v2)];
#[cfg(target_arch = "wasm32")]
pub(super) const SCHEMA_VERSION: i64 = 2;
#[cfg(target_arch = "wasm32")]
const PROPERTIES: &str =
    "CREATE TABLE properties (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);";
#[cfg(target_arch = "wasm32")]
const SCHEMA: &str = concat!(
    include_str!("schema.sql"),
    "\nCREATE TABLE properties (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);\n"
);

pub(super) const MIGRATOR: Migrator = Migrator::new("signal", SCHEMA_VERSION, SCHEMA, MIGRATIONS);

#[cfg(target_arch = "wasm32")]
fn migrate_to_v2(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::query(PROPERTIES)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }
    .boxed()
}
