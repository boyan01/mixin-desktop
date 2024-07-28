use sqlx::{FromRow, Pool, Sqlite};

use crate::db::Error;

#[derive(FromRow)]
pub struct Session {
    pub address: String,
    pub device: u32,
    pub record: Vec<u8>,
}

pub struct SessionDao(pub(crate) Pool<Sqlite>);

impl SessionDao {
    pub async fn find_session(&self, address: &str, device: u32) -> Result<Option<Vec<u8>>, Error> {
        let result = sqlx::query_scalar::<_, Vec<u8>>(
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
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO sessions (address, device, record) VALUES (?, ?, ?)",
        )
        .bind(address)
        .bind(device)
        .bind(record)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
