use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};

use sdk::api::user_api::UserRelationship;

use crate::sdk;

pub struct UserDao(pub(crate) Pool<Sqlite>);

#[derive(sqlx::FromRow)]
pub struct User {
    pub user_id: String,
    pub identity_number: String,
    pub relationship: UserRelationship,
    pub avatar_url: String,
    pub phone: String,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub mute_until: DateTime<Utc>,
    pub has_pin: bool,
    pub app_id: Option<String>,
    pub biography: String,
    pub is_scam: bool,
    pub code_url: String,
    pub code_id: String,
    pub is_deactivated: bool,
}

impl UserDao {
    pub async fn find_user(&self, identity_number: &str) -> Result<Option<String>, sqlx::Error> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT relationship FROM users WHERE identity_number = ?",
        )
        .bind(identity_number)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn insert_sdk_users(&self, users: Vec<String>) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO users (identity_number, relationship) VALUES (?, ?)",
        )
        .bind(users[0].clone())
        .bind(users[1].clone())
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
