use chrono::{DateTime, Utc};
use sqlx::{Pool, QueryBuilder, Sqlite};
use std::collections::HashMap;

use sdk::SYSTEM_USER;

use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::mention_cache::MentionCache;
use crate::db::mixin::util::{expand_var, BindList};
use crate::db::Error;

#[derive(Clone)]
pub struct UserDao(pub(crate) Pool<Sqlite>, MentionCache);

#[derive(sqlx::FromRow, Debug)]
pub struct User {
    pub user_id: String,
    pub identity_number: String,
    pub relationship: Option<sdk::UserRelationship>,
    pub full_name: String,
    pub avatar_url: String,
    pub phone: String,
    pub is_verified: bool,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub mute_until: DateTime<Utc>,
    pub has_pin: bool,
    pub app_id: Option<String>,
    pub biography: String,
    pub is_scam: bool,
    pub code_url: String,
    pub code_id: String,
    pub is_deactivated: bool,
    pub membership: Option<String>,
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
            membership: value
                .membership
                .and_then(|membership| serde_json::to_string(&membership).ok()),
        }
    }
}

impl UserDao {
    pub(crate) fn new(pool: Pool<Sqlite>) -> Self {
        Self(pool, MentionCache::default())
    }

    pub async fn selectable_users(&self, current_user_id: &str) -> Result<Vec<User>, Error> {
        Ok(sqlx::query_as::<_, User>(
            r#"SELECT * FROM users
               WHERE user_id != ?
                 AND identity_number != '0'
                 AND is_deactivated = FALSE
                 AND (relationship = 'FRIEND'
                      OR (app_id IS NOT NULL AND app_id != ''))
               ORDER BY CASE WHEN app_id IS NULL OR app_id = '' THEN 0 ELSE 1 END,
                        full_name COLLATE NOCASE,
                        user_id"#,
        )
        .bind(current_user_id)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn fuzzy_search_users(
        &self,
        current_user_id: &str,
        keyword: &str,
        limit: i64,
    ) -> Result<Vec<User>, Error> {
        Ok(sqlx::query_as::<_, User>(
            r#"SELECT * FROM users
               WHERE user_id != ?
                 AND identity_number != '0'
                 AND (instr(lower(full_name), lower(?)) > 0
                      OR instr(lower(identity_number), lower(?)) > 0)
               ORDER BY (full_name = ? COLLATE NOCASE
                         OR identity_number = ? COLLATE NOCASE) DESC,
                        CASE relationship WHEN 'FRIEND' THEN 0 ELSE 1 END,
                        full_name COLLATE NOCASE,
                        user_id
               LIMIT ?"#,
        )
        .bind(current_user_id)
        .bind(keyword)
        .bind(keyword)
        .bind(keyword)
        .bind(keyword)
        .bind(limit)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn search_bot_group_users(
        &self,
        current_user_id: &str,
        conversation_id: &str,
        keyword: &str,
    ) -> Result<Vec<User>, Error> {
        if keyword.is_empty() {
            return Ok(sqlx::query_as::<_, User>(
                "SELECT * FROM users WHERE relationship = 'FRIEND' ORDER BY full_name, user_id ASC",
            )
            .fetch_all(&self.0)
            .await?);
        }
        let created_at = Utc::now() - chrono::Duration::days(7);
        Ok(sqlx::query_as::<_, User>(
            r#"SELECT DISTINCT u.* FROM users AS u
               WHERE (u.user_id IN (
                        SELECT m.user_id FROM messages AS m
                        WHERE m.conversation_id = ? AND m.created_at > ?
                      ) OR u.relationship = 'FRIEND')
                 AND u.user_id != ?
                 AND u.identity_number != '0'
                 AND (u.full_name LIKE '%' || ? || '%' ESCAPE '\'
                      OR u.identity_number LIKE '%' || ? || '%' ESCAPE '\')
               ORDER BY CASE u.relationship WHEN 'FRIEND' THEN 1 ELSE 2 END,
                        u.full_name = ? COLLATE NOCASE OR
                        u.identity_number = ? COLLATE NOCASE DESC"#,
        )
        .bind(conversation_id)
        .bind(created_at.timestamp_millis())
        .bind(current_user_id)
        .bind(keyword)
        .bind(keyword)
        .bind(keyword)
        .bind(keyword)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn find_user_by_id(&self, user_id: &str) -> Result<Option<User>, Error> {
        Ok(
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = ?")
                .bind(user_id)
                .fetch_optional(&self.0)
                .await?,
        )
    }

    pub async fn find_user_by_identity_number(
        &self,
        identity_number: &str,
    ) -> Result<Option<User>, Error> {
        Ok(
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE identity_number = ? LIMIT 1")
                .bind(identity_number)
                .fetch_optional(&self.0)
                .await?,
        )
    }

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
        let result = sqlx::query_as::<_, User>(sqlx::AssertSqlSafe(query_str))
            .bind_list(ids)
            .fetch_all(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn find_user_ids_by_identity_numbers(
        &self,
        identity_numbers: &[String],
    ) -> Result<Vec<String>, Error> {
        let mut user_ids = Vec::new();
        for chunk in identity_numbers.chunks(MARK_LIMIT) {
            let query = format!(
                "SELECT user_id FROM users WHERE identity_number IN ({})",
                expand_var(chunk.len())
            );
            user_ids.extend(
                sqlx::query_as::<_, (String,)>(sqlx::AssertSqlSafe(query))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?
                    .into_iter()
                    .map(|(user_id,)| user_id),
            );
        }
        Ok(user_ids)
    }

    pub async fn find_users_by_identity_numbers(
        &self,
        identity_numbers: &[String],
    ) -> Result<Vec<User>, Error> {
        let mut users = Vec::new();
        for chunk in identity_numbers.chunks(MARK_LIMIT) {
            let query = format!(
                "SELECT * FROM users WHERE identity_number IN ({})",
                expand_var(chunk.len())
            );
            users.extend(
                sqlx::query_as::<_, User>(sqlx::AssertSqlSafe(query))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?,
            );
        }
        Ok(users)
    }

    pub async fn insert_sdk_users(&self, users: Vec<sdk::User>) -> Result<Vec<User>, Error> {
        if users.is_empty() {
            return Ok(Vec::new());
        }
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            r#"INSERT OR REPLACE INTO users (user_id, identity_number, relationship, full_name, avatar_url,
              phone, is_verified, created_at, mute_until, has_pin, app_id, biography, is_scam, 
              code_url, code_id, is_deactivated, membership)"#,
        );

        let db_users = users.into_iter().map(User::from).collect::<Vec<_>>();

        query_builder.push_values(db_users.iter(), |mut b, user| {
            b.push_bind(&user.user_id)
                .push_bind(&user.identity_number)
                .push_bind(&user.relationship)
                .push_bind(&user.full_name)
                .push_bind(&user.avatar_url)
                .push_bind(&user.phone)
                .push_bind(user.is_verified)
                .push_bind(user.created_at.timestamp_millis())
                .push_bind(user.mute_until.timestamp_millis())
                .push_bind(user.has_pin)
                .push_bind(&user.app_id)
                .push_bind(&user.biography)
                .push_bind(user.is_scam)
                .push_bind(&user.code_url)
                .push_bind(&user.code_id)
                .push_bind(user.is_deactivated)
                .push_bind(&user.membership);
        });

        let query = query_builder.build();
        query.execute(&self.0).await?;
        self.1.cache_users(&db_users);

        Ok(db_users)
    }

    pub async fn replace_mentions(&self, contents: &[String]) -> Result<Vec<String>, Error> {
        self.load_mention_users(contents).await?;
        Ok(contents
            .iter()
            .map(|content| self.1.replace_mentions(content))
            .collect())
    }

    pub async fn mention_names(
        &self,
        contents: &[String],
    ) -> Result<HashMap<String, String>, Error> {
        self.load_mention_users(contents).await?;
        Ok(self.1.mention_names(contents))
    }

    async fn load_mention_users(&self, contents: &[String]) -> Result<(), Error> {
        let missing = self.1.missing_identity_numbers(contents);
        if !missing.is_empty() {
            let users = self.find_users_by_identity_numbers(&missing).await?;
            self.1.cache_users(&users);
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::db::MixinDatabase;

    #[tokio::test]
    async fn accepts_empty_user_batch() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();

        assert!(database
            .user_dao
            .insert_sdk_users(Vec::new())
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn finds_users_by_identity_numbers_in_one_batch() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        for (user_id, identity_number, full_name) in [
            ("user-1", "7001", "Alice"),
            ("user-2", "7002", "Bob"),
            ("user-3", "7003", "Carol"),
        ] {
            sqlx::query(
                r#"INSERT INTO users (
                    user_id, identity_number, relationship, full_name, avatar_url,
                    phone, is_verified, created_at, mute_until, has_pin, app_id,
                    biography, is_scam, code_url, code_id, is_deactivated
                ) VALUES (?, ?, 'STRANGER', ?, '', '', FALSE, CURRENT_TIMESTAMP,
                    CURRENT_TIMESTAMP, FALSE, NULL, '', FALSE, '', '', FALSE)"#,
            )
            .bind(user_id)
            .bind(identity_number)
            .bind(full_name)
            .execute(&database.user_dao.0)
            .await
            .unwrap();
        }

        let mut users = database
            .user_dao
            .find_users_by_identity_numbers(&[
                "7001".to_string(),
                "7003".to_string(),
                "9999".to_string(),
            ])
            .await
            .unwrap();
        users.sort_by(|first, second| first.identity_number.cmp(&second.identity_number));

        assert_eq!(
            users
                .into_iter()
                .map(|user| (user.identity_number, user.full_name))
                .collect::<Vec<_>>(),
            vec![
                ("7001".to_string(), "Alice".to_string()),
                ("7003".to_string(), "Carol".to_string()),
            ]
        );

        assert_eq!(
            database
                .user_dao
                .replace_mentions(&[
                    "hello @7001 and @7003".to_string(),
                    "unknown @9999".to_string(),
                ])
                .await
                .unwrap(),
            ["hello @Alice and @Carol", "unknown @9999"]
        );
        assert_eq!(
            database
                .user_dao
                .mention_names(&["hello @7001 and @7003".to_string()])
                .await
                .unwrap(),
            HashMap::from([
                ("7001".to_string(), "Alice".to_string()),
                ("7003".to_string(), "Carol".to_string()),
            ])
        );

        let mut updated_users = database
            .user_dao
            .find_users_by_identity_numbers(&["7001".to_string()])
            .await
            .unwrap();
        updated_users[0].full_name = "Alicia".to_string();
        database.user_dao.1.cache_users(&updated_users);
        assert_eq!(
            database
                .user_dao
                .replace_mentions(&["hello @7001".to_string()])
                .await
                .unwrap(),
            ["hello @Alicia"]
        );
    }

    #[tokio::test]
    async fn searches_bot_group_friends_with_flutter_semantics() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        for (user_id, identity_number, relationship, full_name) in [
            ("current", "7000", "ME", "Current"),
            ("friend", "7001", "FRIEND", "Alice"),
            ("stranger", "7002", "STRANGER", "Alice Stranger"),
        ] {
            sqlx::query(
                r#"INSERT INTO users (
                    user_id, identity_number, relationship, full_name, avatar_url,
                    phone, is_verified, created_at, mute_until, has_pin, app_id,
                    biography, is_scam, code_url, code_id, is_deactivated
                ) VALUES (?, ?, ?, ?, '', '', FALSE, CURRENT_TIMESTAMP,
                    CURRENT_TIMESTAMP, FALSE, NULL, '', FALSE, '', '', FALSE)"#,
            )
            .bind(user_id)
            .bind(identity_number)
            .bind(relationship)
            .bind(full_name)
            .execute(&database.user_dao.0)
            .await
            .unwrap();
        }

        let friends = database
            .user_dao
            .search_bot_group_users("current", "conversation", "")
            .await
            .unwrap();
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].user_id, "friend");
        let matches = database
            .user_dao
            .search_bot_group_users("current", "conversation", "Alice")
            .await
            .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].user_id, "friend");
    }
}
