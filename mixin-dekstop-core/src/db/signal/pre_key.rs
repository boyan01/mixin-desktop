use sqlx::{Executor, Pool, Sqlite};

use crate::db::Error;

#[derive(sqlx::FromRow)]
pub struct PreKey {
    pub prekey_id: u32,
    pub record: Vec<u8>,
}

pub struct PreKeyDao(Pool<Sqlite>);

impl PreKeyDao {
    pub async fn find_pre_key(&self, prekey_id: u32) -> Result<Option<PreKey>, Error> {
        let result = sqlx::query_as::<_, PreKey>("SELECT * FROM prekeys WHERE prekey_id = ?")
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

    pub async fn delete_pre_key(&self, prekey_id: u32) -> Result<(), Error> {
        let _ = sqlx::query("DELETE FROM prekeys WHERE prekey_id = ?")
            .bind(prekey_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
