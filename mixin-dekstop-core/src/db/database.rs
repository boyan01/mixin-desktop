use diesel::{Queryable, result, RunQueryDsl, SqliteConnection};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::db;
use crate::db::users;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub struct MixinDatabase {
    pub(crate) pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl MixinDatabase {
    pub async fn new(identity_number: String) -> Result<Self, String> {
        let manager = ConnectionManager::<SqliteConnection>::new("mixin.db");
        let pool = Pool::builder().build(manager).expect("failed to create pool");
        pool.get().expect("a").run_pending_migrations(MIGRATIONS).expect("TODO: panic message");
        return Ok(MixinDatabase { pool });
    }
}

type Connection = PooledConnection<ConnectionManager<SqliteConnection>>;

impl MixinDatabase {
    pub(crate) fn get_connection(&self) -> Result<Connection, db::Error> {
        let c = self.pool.get()?;
        Ok(c)
    }

    pub async fn query_friends(&self) -> Result<(), result::Error> {
        let mut conn = self.pool.get().expect("Failed to get connection.");
        let a = users::table.select(users::user_id).load::<String>(&mut conn)?;
        Ok(())
    }
}

#[derive(Queryable)]
struct User {
    id: String,
    identity_number: String,
    relationship: Option<String>,
    full_name: Option<String>,
    avatar_url: Option<String>,
    phone: Option<String>,
    is_verified: Option<bool>,
    created_at: Option<i32>,
    mute_until: Option<i32>,
    has_pin: Option<i32>,
    biography: Option<String>,
    is_scam: Option<i32>,
    code_url: Option<String>,
    code_id: Option<String>,
    is_deactivated: Option<bool>,
}
