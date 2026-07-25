use std::io::BufReader;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Sqlite};

use crate::core::crypto::signal_protocol::MAX_VALUE;

pub struct CryptoKeyValue {
    pool: Pool<Sqlite>,
    inner: Arc<Mutex<CryptoKeyValueInner>>,
}

struct CryptoKeyValueInner {
    next_pre_key_id: u32,
    next_signed_pre_key_id: u32,
    has_push_signal_keys: bool,
}

const KEY_NEXT_PRE_KEY_ID: &str = "next_pre_key_id";
const KEY_NEXT_SIGNED_PRE_KEY_ID: &str = "next_signed_pre_key_id";
const KEY_HAS_PUSH_SIGNAL_KEYS: &str = "has_push_signal_keys";

impl CryptoKeyValue {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        CryptoKeyValue {
            pool,
            inner: Arc::new(Mutex::new(CryptoKeyValueInner {
                next_pre_key_id: 0,
                next_signed_pre_key_id: 0,
                has_push_signal_keys: false,
            })),
        }
    }

    pub async fn init(&self) -> Result<()> {
        let next_pre_key_id: u32 = self
            .get_property(KEY_NEXT_PRE_KEY_ID)
            .await?
            .unwrap_or_else(|| rand::random_range(0..MAX_VALUE));
        let next_signed_pre_key_id: u32 = self
            .get_property(KEY_NEXT_SIGNED_PRE_KEY_ID)
            .await?
            .unwrap_or_else(|| rand::random_range(0..MAX_VALUE));
        let has_push_signal_keys: bool = self
            .get_property(KEY_HAS_PUSH_SIGNAL_KEYS)
            .await?
            .unwrap_or(false);

        let mut inner = self.inner.lock().unwrap();
        inner.next_pre_key_id = next_pre_key_id;
        inner.next_signed_pre_key_id = next_signed_pre_key_id;
        inner.has_push_signal_keys = has_push_signal_keys;
        Ok(())
    }

    pub fn next_pre_key_id(&self) -> u32 {
        self.inner.lock().unwrap().next_pre_key_id
    }

    pub fn next_signed_pre_key_id(&self) -> u32 {
        self.inner.lock().unwrap().next_signed_pre_key_id
    }

    pub fn has_push_signal_keys(&self) -> bool {
        self.inner.lock().unwrap().has_push_signal_keys
    }

    pub async fn set_next_pre_key_id(&self, next_pre_key_id: u32) -> Result<()> {
        self.inner.lock().unwrap().next_pre_key_id = next_pre_key_id;
        self.set_property(KEY_NEXT_PRE_KEY_ID, &next_pre_key_id)
            .await
    }

    pub async fn set_next_signed_pre_key_id(&self, next_signed_pre_key_id: u32) -> Result<()> {
        self.inner.lock().unwrap().next_signed_pre_key_id = next_signed_pre_key_id;
        self.set_property(KEY_NEXT_SIGNED_PRE_KEY_ID, &next_signed_pre_key_id)
            .await
    }

    pub async fn set_has_push_signal_keys(&self, has_push_signal_keys: bool) -> Result<()> {
        self.inner.lock().unwrap().has_push_signal_keys = has_push_signal_keys;
        self.set_property(KEY_HAS_PUSH_SIGNAL_KEYS, &has_push_signal_keys)
            .await
    }

    async fn get_property<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM properties WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        let Some(value) = value else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_reader(BufReader::new(
            value.as_bytes(),
        ))?))
    }

    async fn set_property<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let value = serde_json::to_string(value)?;
        sqlx::query(
            "INSERT INTO properties (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
