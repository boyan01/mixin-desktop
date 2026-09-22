use crate::db::Error;

pub struct SignedPreKey {
    pub prekey_id: u32,
    pub record: Vec<u8>,
}

pub struct SignedPreKeyDao(pub(crate) crate::db::Pool);

impl SignedPreKeyDao {
    pub async fn find_signed_pre_key(&self, prekey_id: u32) -> Result<Option<Vec<u8>>, Error> {
        let result = crate::db::query_scalar::<Vec<u8>>(
            "SELECT record FROM signed_prekeys WHERE prekey_id = ?",
        )
        .bind(prekey_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn save_signed_pre_key(&self, prekey_id: u32, record: Vec<u8>) -> Result<(), Error> {
        let _ = crate::db::query(
            "INSERT OR REPLACE INTO signed_prekeys (prekey_id, record, timestamp) VALUES (?, ?, ?)",
        )
        .bind(prekey_id)
        .bind(record)
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete_signed_pre_key(&self, prekey_id: u32) -> Result<(), Error> {
        let _ = crate::db::query("DELETE FROM signed_prekeys WHERE prekey_id = ?")
            .bind(prekey_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

crate::db::storage::row_mapping!(SignedPreKey {
    prekey_id: "prekey_id",
    record: "record",
});
