use crate::db::migration::Migrator;

pub(super) const SCHEMA_VERSION: i64 = 1;
pub(super) const MIGRATOR: Migrator =
    Migrator::new("app", SCHEMA_VERSION, include_str!("schema.sql"), &[]);
