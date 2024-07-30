use std::error::Error;

use sqlx::{Pool, Sqlite};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::db::mixin::message::MessageDao;
use crate::db::mixin::user::UserDao;

pub struct MixinDatabase {
    pub(crate) pool: Pool<Sqlite>,
    pub user_dao: UserDao,
    pub message_dao: MessageDao,
}

impl MixinDatabase {
    pub async fn new(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename("mixin.db")
                    .create_if_missing(true),
            )
            .await?;
        let migrator = sqlx::migrate!("./src/db/mixin/migrations");
        migrator.run(&pool).await?;
        Ok(MixinDatabase {
            pool: pool.clone(),
            user_dao: UserDao(pool.clone()),
            message_dao: MessageDao(pool.clone()),
        })
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
