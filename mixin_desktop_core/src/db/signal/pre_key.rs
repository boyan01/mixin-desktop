use crate::db::QueryBuilder;

use crate::db::Error;

pub struct PreKey {
    pub prekey_id: u32,
    pub record: Vec<u8>,
}

pub struct PreKeyDao(pub(crate) crate::db::Pool);

impl PreKeyDao {
    pub async fn find_pre_key(&self, prekey_id: u32) -> Result<Option<Vec<u8>>, Error> {
        let result =
            crate::db::query_scalar::<Vec<u8>>("SELECT record FROM prekeys WHERE prekey_id = ?")
                .bind(prekey_id)
                .fetch_optional(&self.0)
                .await?;
        Ok(result)
    }

    pub async fn save_pre_key(&self, prekey_id: u32, record: Vec<u8>) -> Result<(), Error> {
        let _ =
            crate::db::query("INSERT OR REPLACE INTO prekeys (prekey_id, record) VALUES (?, ?)")
                .bind(prekey_id)
                .bind(record)
                .execute(&self.0)
                .await?;
        Ok(())
    }

    pub async fn insert_pre_key_list(&self, list: &[PreKey]) -> Result<(), Error> {
        let mut query_builder: QueryBuilder =
            QueryBuilder::new("INSERT OR REPLACE INTO prekeys (prekey_id, record) ");
        query_builder.push_values(list, |mut b, PreKey { prekey_id, record }| {
            b.push_bind(prekey_id).push_bind(record);
        });
        query_builder.build().execute(&self.0).await?;
        Ok(())
    }

    pub async fn delete_pre_key(&self, prekey_id: u32) -> Result<(), Error> {
        let _ = crate::db::query("DELETE FROM prekeys WHERE prekey_id = ?")
            .bind(prekey_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

crate::db::storage::row_mapping!(PreKey {
    prekey_id: "prekey_id",
    record: "record",
});
