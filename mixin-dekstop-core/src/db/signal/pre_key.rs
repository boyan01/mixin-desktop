use libsignal_protocol::{Context, error, PreKeyRecord, PreKeyStore, SignalProtocolError};
use log::error;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::async_trait::async_trait;
use sqlx::Executor;

use crate::db::signal::database::SignalDatabase;

#[derive(sqlx::FromRow)]
pub struct PreKey {
    pub prekey_id: u32,
    pub record: Vec<u8>,
}

#[async_trait(? Send)]
impl PreKeyStore for SignalDatabase {
    async fn get_pre_key(&self, prekey_id: u32, ctx: Context) -> error::Result<PreKeyRecord> {
        let result = sqlx::query_as::<_, PreKey>("SELECT * FROM prekeys WHERE prekey_id = ?")
            .bind(prekey_id)
            .fetch_one(&self.pool).await;
        if let Err(err) = result {
            error!("Failed to get prekey: {}", err);
            return Err(SignalProtocolError::InvalidPreKeyId);
        }
        let prekey = result.unwrap();
        Ok(PreKeyRecord::deserialize(&prekey.record)?)
    }

    async fn save_pre_key(&mut self, prekey_id: u32, record: &PreKeyRecord, ctx: Context) -> error::Result<()> {
        let result = sqlx::query("INSERT OR REPLACE INTO prekeys (prekey_id, record) VALUES (?, ?)")
            .bind(prekey_id)
            .bind(record.serialize()?)
            .execute(&self.pool).await;
        if let Err(err) = result {
            error!("Failed to save prekey: {}", err);
            return Err(SignalProtocolError::InvalidPreKeyId);
        }
        Ok(())
    }

    async fn remove_pre_key(&mut self, prekey_id: u32, ctx: Context) -> error::Result<()> {
        if let Err(err) = sqlx::query("DELETE FROM prekeys WHERE prekey_id = ?")
            .bind(prekey_id).execute(&self.pool).await {
            error!("Failed to remove prekey: {}", err);
            return Err(SignalProtocolError::InvalidPreKeyId);
        }
        Ok(())
    }
}