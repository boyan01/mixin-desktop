use chrono::{DateTime, Utc};
use sqlx::{Pool, QueryBuilder, Sqlite};

use sdk::SYSTEM_USER;

use crate::db::mixin::util::{expand_var, BindList};
use crate::db::Error;

#[derive(Clone)]
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

impl From<sdk::User> for User {
    fn from(value: sdk::User) -> Self {
        User {
            user_id: value.user_id,
            identity_number: value.identity_number,
            relationship: value.relationship,
            full_name: value.full_name,
            avatar_url: value.avatar_url,
            phone: value.phone,
            is_verified: value.is_verified,
            created_at: value.created_at,
            mute_until: value.mute_until,
            has_pin: value.has_pin,
            app_id: value.app.map(|app| app.app_id),
            biography: value.biography,
            is_scam: value.is_scam,
            code_url: value.code_url,
            code_id: value.code_id,
            is_deactivated: value.is_deactivated,
        }
    }
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
        let query_str = format!(
            "SELECT * FROM users WHERE user_id IN ({})",
            expand_var(ids.len())
        );
        let result = sqlx::query_as::<_, User>(&query_str)
            .bind_list(ids)
            .fetch_all(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn insert_sdk_users(&self, users: Vec<sdk::User>) -> Result<Vec<User>, Error> {
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            r#"INSERT OR REPLACE INTO users (user_id, identity_number, relationship, full_name, avatar_url,
              phone, is_verified, created_at, mute_until, has_pin, app_id, biography, is_scam, 
              code_url, code_id, is_deactivated)"#,
        );

        let db_users = users
            .into_iter()
            .map(move |user| User::from(user))
            .collect::<Vec<_>>();

        query_builder.push_values(db_users.iter(), |mut b, user| {
            b.push_bind(&user.user_id)
                .push_bind(&user.identity_number)
                .push_bind(&user.relationship)
                .push_bind(&user.full_name)
                .push_bind(&user.avatar_url)
                .push_bind(&user.phone)
                .push_bind(&user.is_verified)
                .push_bind(&user.created_at)
                .push_bind(&user.mute_until)
                .push_bind(&user.has_pin)
                .push_bind(&user.app_id)
                .push_bind(&user.biography)
                .push_bind(&user.is_scam)
                .push_bind(&user.code_url)
                .push_bind(&user.code_id)
                .push_bind(&user.is_deactivated);
        });

        let query = query_builder.build();
        query.execute(&self.0).await?;

        Ok(db_users)
    }

    pub async fn has_user(&self, user_id: &str) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE user_id = ?)")
                .bind(user_id)
                .fetch_one(&self.0)
                .await?;
        Ok(result)
    }

    pub async fn insert_system_user_if_not_exist(&self) -> Result<(), sqlx::Error> {
        if self.has_user(SYSTEM_USER).await? {
            return Ok(());
        }
        sqlx::query("INSERT OR REPLACE INTO users (user_id, identity_number) VALUES (?, ?)")
            .bind(SYSTEM_USER)
            .bind("0")
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
