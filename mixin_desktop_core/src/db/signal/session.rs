use crate::db::Error;

pub struct Session {
    pub address: String,
    pub device: u32,
    pub record: Vec<u8>,
}

pub struct SessionDao(pub(crate) crate::db::Pool);

impl SessionDao {
    pub async fn has_session(&self, address: &str) -> Result<bool, Error> {
        let result = crate::db::query_scalar::<bool>(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE address = ?)",
        )
        .bind(address)
        .fetch_one(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn find_session(&self, address: &str, device: u32) -> Result<Option<Vec<u8>>, Error> {
        let result = crate::db::query_scalar::<Vec<u8>>(
            "SELECT record FROM sessions WHERE address = ? AND device = ?",
        )
        .bind(address)
        .bind(device)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn save_session(
        &self,
        address: &str,
        device: u32,
        record: Vec<u8>,
    ) -> Result<(), Error> {
        let _ = crate::db::query(
            "INSERT OR REPLACE INTO sessions (address, device, record, timestamp) VALUES (?, ?, ?, ?)",
        )
        .bind(address)
        .bind(device)
        .bind(record)
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete_sessions(&self, address: &str) -> Result<(), Error> {
        let _ = crate::db::query("DELETE FROM sessions WHERE address = ?")
            .bind(address)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn delete_session(&self, address: &str, device: u32) -> Result<(), Error> {
        let _ = crate::db::query("DELETE FROM sessions WHERE address = ? AND device = ?")
            .bind(address)
            .bind(device)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

crate::db::storage::row_mapping!(Session {
    address: "address",
    device: "device",
    record: "record",
});
