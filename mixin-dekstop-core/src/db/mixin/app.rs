use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}
