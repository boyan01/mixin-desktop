use std::io::BufReader;

use log::error;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub(crate) struct KeyValue(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl KeyValue {
    pub async fn get(&self, key: &str, group: &str) -> Result<Option<String>, sqlx::Error> {
        let result = sqlx::query_scalar::<_, String>(
            r#"SELECT value FROM properties WHERE key = ? AND "group" = ?"#,
        )
        .bind(key)
        .bind(group)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn set(&self, key: &str, group: &str, value: &str) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(
            r#"INSERT OR REPLACE INTO properties (key, "group", value) VALUES (?, ?, ?)"#,
        )
        .bind(key)
        .bind(group)
        .bind(value)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn remove(&self, key: &str, group: &str) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(r#"DELETE FROM properties WHERE key = ? AND "group" = ?"#)
            .bind(key)
            .bind(group)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn clear_by_group(&self, group: &str) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(r#"DELETE FROM properties WHERE "group" = ?"#)
            .bind(group)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

impl KeyValue {
    pub async fn get_value<T>(&self, key: &str, group: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let result = self
            .get(key, group)
            .await
            .map_err(|e| {
                error!("failed to get properties({}, {}): {:?}", key, group, e);
                e
            })
            .ok()
            .flatten();
        if let Some(data) = result {
            let result: serde_json::Result<T> =
                serde_json::from_reader(BufReader::new(data.as_bytes()));

            match result {
                Ok(result) => Some(result),
                Err(err) => {
                    error!(
                        "failed to deserialize properties({}, {}): {:?}",
                        key, group, err
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    pub async fn set_value<T>(&self, key: &str, group: &str, value: &T)
    where
        T: Serialize + ?Sized,
    {
        let content = serde_json::to_string(&value);
        let content = match content {
            Ok(data) => data,
            Err(err) => {
                error!(
                    "failed to serialize properties({}, {}): {:?}",
                    key, group, err
                );
                return;
            }
        };
        let result = self.set(key, group, &content).await;
        if let Err(e) = result {
            error!("failed to set properties({}, {}): {:?}", key, group, e);
        }
    }
}
