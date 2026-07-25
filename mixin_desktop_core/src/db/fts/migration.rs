use crate::db::migration::Migrator;

const SCHEMA_VERSION: i64 = 1;
pub(super) const MIGRATOR: Migrator =
    Migrator::new("fts", SCHEMA_VERSION, include_str!("schema.sql"), &[]);
