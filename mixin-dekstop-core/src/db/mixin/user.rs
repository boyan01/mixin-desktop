use chrono::{DateTime, Utc};
use sqlx::{Pool, QueryBuilder, Sqlite};

use crate::db::Error;

pub struct UserDao(pub(crate) Pool<Sqlite>);

#[derive(sqlx::FromRow, Debug)]
pub struct User {
    pub user_id: String,
    pub identity_number: String,
    pub relationship: sdk::UserRelationship,
    pub full_name: String,
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

    pub async fn find_users(&self, ids: &[String]) -> Result<Vec<User>, Error> {
        let params = format!("?{}", ", ?".repeat(ids.len() - 1));
        let query_str = format!("SELECT * FROM users WHERE user_id IN ({})", params);
        let mut query = sqlx::query_as::<_, User>(&query_str);
        for id in ids {
            query = query.bind(id);
        }
        let result = query.fetch_all(&self.0).await?;
        Ok(result)
    }

    pub async fn insert_sdk_users(&self, users: Vec<sdk::User>) -> Result<(), Error> {
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            r#"INSERT OR REPLACE INTO users (user_id, identity_number, relationship, full_name, avatar_url,
              phone, is_verified, created_at, mute_until, has_pin, app_id, biography, is_scam, 
              code_url, code_id, is_deactivated)"#,
        );
        query_builder.push_values(users, |mut b, user| {
            b.push_bind(user.user_id)
                .push_bind(user.identity_number)
                .push_bind(user.relationship)
                .push_bind(user.full_name)
                .push_bind(user.avatar_url)
                .push_bind(user.phone)
                .push_bind(user.is_verified)
                .push_bind(user.created_at)
                .push_bind(user.mute_until)
                .push_bind(user.has_pin)
                .push_bind(user.app.map(|app| app.app_id))
                .push_bind(user.biography)
                .push_bind(user.is_scam)
                .push_bind(user.code_url)
                .push_bind(user.code_id)
                .push_bind(user.is_deactivated);
        });
        let query = query_builder.build();
        query.execute(&self.0).await?;
        Ok(())
    }
}
