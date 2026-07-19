use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;

use crate::db::Error;

#[derive(Clone)]
pub struct AppDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct App {
    pub app_id: String,
    pub app_number: String,
    pub home_uri: String,
    pub redirect_uri: String,
    pub name: String,
    pub icon_url: String,
    pub category: String,
    pub description: String,
    pub app_secret: String,
    pub capabilities: Option<String>,
    pub creator_id: String,
    pub resource_patterns: Option<String>,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub updated_at: DateTime<Utc>,
}

impl AppDao {
    pub async fn find_app_by_id(&self, app_id: &str) -> Result<Option<App>, Error> {
        let result = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE app_id = ?")
            .bind(app_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn insert_sdk_apps(&self, apps: &[sdk::App]) -> Result<(), Error> {
        if apps.is_empty() {
            return Ok(());
        }
        let mut query_builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new(
            r#"INSERT OR REPLACE INTO apps (
                app_id, app_number, home_uri, redirect_uri, name, icon_url,
                category, description, app_secret, capabilities, creator_id,
                resource_patterns, updated_at
            ) "#,
        );
        query_builder.push_values(apps, |mut builder, app| {
            builder
                .push_bind(&app.app_id)
                .push_bind(&app.app_number)
                .push_bind(&app.home_uri)
                .push_bind(&app.redirect_uri)
                .push_bind(&app.name)
                .push_bind(&app.icon_url)
                .push_bind(&app.category)
                .push_bind(&app.description)
                .push_bind(&app.app_secret)
                .push_bind(format!("[{}]", app.capabilities.join(", ")))
                .push_bind(&app.creator_id)
                .push_bind(format!("[{}]", app.resource_patterns.join(", ")))
                .push_bind(app.updated_at.timestamp_millis());
        });
        query_builder.build().execute(&self.0).await?;
        Ok(())
    }
}
