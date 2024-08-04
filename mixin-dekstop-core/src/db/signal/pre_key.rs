use sqlx::{Pool, QueryBuilder, Sqlite};

use crate::db::Error;

#[derive(sqlx::FromRow)]
pub struct PreKey {
    pub prekey_id: u32,
    pub record: Vec<u8>,
}

pub struct PreKeyDao(pub(crate) Pool<Sqlite>);

impl PreKeyDao {
    pub async fn find_pre_key(&self, prekey_id: u32) -> Result<Option<Vec<u8>>, Error> {
        let result =
            sqlx::query_scalar::<_, Vec<u8>>("SELECT record FROM prekeys WHERE prekey_id = ?")
                .bind(prekey_id)
                .fetch_optional(&self.0)
                .await?;
        Ok(result)
    }

    pub async fn save_pre_key(&self, prekey_id: u32, record: Vec<u8>) -> Result<(), Error> {
        let _ = sqlx::query("INSERT OR REPLACE INTO prekeys (prekey_id, record) VALUES (?, ?)")
            .bind(prekey_id)
            .bind(record)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn insert_pre_key_list(&self, list: &[PreKey]) -> Result<(), Error> {
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT OR REPLACE INTO prekeys (prekey_id, record) ");
        query_builder.push_values(list, |mut b, PreKey { prekey_id, record }| {
            b.push_bind(prekey_id).push_bind(record);
        });
        query_builder.build().execute(&self.0).await?;
        Ok(())
    }

    pub async fn delete_pre_key(&self, prekey_id: u32) -> Result<(), Error> {
        let _ = sqlx::query("DELETE FROM prekeys WHERE prekey_id = ?")
            .bind(prekey_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
