use chrono::Utc;
use sqlx::{QueryBuilder, Sqlite};

use db::Error;

use crate::db;
use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::util::{expand_var_with_index, BindListForQuery};

#[derive(Clone)]
pub struct ExpiredMessageDao(pub(crate) sqlx::Pool<Sqlite>);

impl ExpiredMessageDao {
    pub async fn update_message_expired_at(&self, data: &[(String, i64)]) -> Result<u64, Error> {
        let mut rows_affected: u64 = 0;
        for chunk in data.chunks(MARK_LIMIT) {
            let mut query_builder: QueryBuilder<Sqlite> = sqlx::QueryBuilder::new(
                "INSERT OR REPLACE INTO expired_messages (message_id, expire_at) VALUES ",
            );
            query_builder.push_values(chunk, |mut builder, (message_id, expire_at)| {
                builder.push_bind(message_id).push_bind(expire_at);
            });
            rows_affected += query_builder
                .build()
                .execute(&self.0)
                .await?
                .rows_affected();
        }
        Ok(rows_affected)
    }

    pub async fn mark_expired_message_read(&self, message_ids: &[String]) -> Result<u64, Error> {
        let sql = format!(
            "UPDATE expired_messages SET expire_at = CAST((?1 + expire_in) AS INTEGER) \
        WHERE (expire_at > (?1 + expire_in) OR expire_at IS NULL) AND message_id IN ({})",
            expand_var_with_index(2, message_ids.len())
        );

        let now = Utc::now().timestamp_millis() / 1000;
        let iter = message_ids.chunks(MARK_LIMIT);
        let mut rows_affected: u64 = 0;
        for chunk in iter {
            let affected = sqlx::query(&sql)
                .bind(now / 1000)
                .bind_list(chunk)
                .execute(&self.0)
                .await?
                .rows_affected();
            rows_affected += affected;
        }
        Ok(rows_affected)
    }
}
