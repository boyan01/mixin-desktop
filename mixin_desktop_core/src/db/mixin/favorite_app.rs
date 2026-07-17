use sqlx::QueryBuilder;

use crate::db::mixin::app::App;
use crate::db::Error;

#[derive(Clone)]
pub struct FavoriteAppDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl FavoriteAppDao {
    pub async fn replace_for_user(
        &self,
        user_id: &str,
        apps: &[sdk::FavoriteApp],
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        sqlx::query("DELETE FROM favorite_apps WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;

        if !apps.is_empty() {
            let mut query_builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new(
                "INSERT OR REPLACE INTO favorite_apps (app_id, user_id, created_at) ",
            );
            query_builder.push_values(apps, |mut builder, app| {
                builder
                    .push_bind(&app.app_id)
                    .push_bind(&app.user_id)
                    .push_bind(app.created_at);
            });
            query_builder.build().execute(&mut *transaction).await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn find_apps_by_user_id(&self, user_id: &str) -> Result<Vec<App>, Error> {
        Ok(sqlx::query_as::<_, App>(
            r#"SELECT a.* FROM favorite_apps AS fa
               INNER JOIN apps AS a ON fa.app_id = a.app_id
               WHERE fa.user_id = ?"#,
        )
        .bind(user_id)
        .fetch_all(&self.0)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::db::MixinDatabase;

    #[tokio::test]
    async fn replaces_and_reads_cached_favorite_apps() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let apps = [("app-1", "First"), ("app-2", "Second")]
            .into_iter()
            .map(|(app_id, name)| sdk::App {
                app_id: app_id.to_string(),
                app_number: String::new(),
                app_secret: String::new(),
                capabilities: Vec::new(),
                category: String::new(),
                creator_id: String::new(),
                description: String::new(),
                has_safe: false,
                home_uri: String::new(),
                icon_url: String::new(),
                is_verified: false,
                name: name.to_string(),
                redirect_uri: String::new(),
                resource_patterns: Vec::new(),
                safe_created_at: String::new(),
                updated_at: Utc.timestamp_opt(1, 0).unwrap(),
            })
            .collect::<Vec<_>>();
        database.app_dao.insert_sdk_apps(&apps).await.unwrap();

        database
            .favorite_app_dao
            .replace_for_user(
                "user-1",
                &[
                    sdk::FavoriteApp {
                        app_id: "app-1".to_string(),
                        user_id: "user-1".to_string(),
                        created_at: Utc.timestamp_opt(2, 0).unwrap(),
                    },
                    sdk::FavoriteApp {
                        app_id: "app-2".to_string(),
                        user_id: "user-1".to_string(),
                        created_at: Utc.timestamp_opt(3, 0).unwrap(),
                    },
                ],
            )
            .await
            .unwrap();
        let mut cached_apps = database
            .favorite_app_dao
            .find_apps_by_user_id("user-1")
            .await
            .unwrap();
        cached_apps.sort_by(|first, second| first.app_id.cmp(&second.app_id));
        assert_eq!(
            cached_apps
                .into_iter()
                .map(|app| (app.app_id, app.name))
                .collect::<Vec<_>>(),
            vec![
                ("app-1".to_string(), "First".to_string()),
                ("app-2".to_string(), "Second".to_string()),
            ]
        );

        database
            .favorite_app_dao
            .replace_for_user(
                "user-2",
                &[sdk::FavoriteApp {
                    app_id: "app-1".to_string(),
                    user_id: "user-2".to_string(),
                    created_at: Utc.timestamp_opt(4, 0).unwrap(),
                }],
            )
            .await
            .unwrap();

        database
            .favorite_app_dao
            .replace_for_user("user-1", &[])
            .await
            .unwrap();
        assert!(database
            .favorite_app_dao
            .find_apps_by_user_id("user-1")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            database
                .favorite_app_dao
                .find_apps_by_user_id("user-2")
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
