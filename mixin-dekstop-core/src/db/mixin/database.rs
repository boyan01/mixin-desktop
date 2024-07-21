use std::error::Error;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};

pub struct MixinDatabase {
    pub(crate) pool: Pool<Sqlite>,
}

impl MixinDatabase {
    pub async fn new(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename("signal.db")
                    .create_if_missing(true),
            )
            .await?;
        let migrator = sqlx::migrate!("./src/db/mixin/migrations");
        return Ok(MixinDatabase { pool });
    }
}

impl MixinDatabase {}

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
