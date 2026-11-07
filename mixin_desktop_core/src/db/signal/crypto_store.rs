#[cfg(target_arch = "wasm32")]
use std::io::BufReader;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::core::crypto::signal_protocol::MAX_VALUE;

/// Signal counters. Native builds share the Flutter app's per-account
/// `crypto_box` and `privacy_box`; wasm keeps them in `signal.db`.
pub struct CryptoKeyValue {
    store: Store,
    inner: Arc<Mutex<CryptoKeyValueInner>>,
}

#[cfg(not(target_arch = "wasm32"))]
struct Store {
    crypto: Arc<crate::db::hive::HiveBox>,
    privacy: Arc<crate::db::hive::HiveBox>,
}

#[cfg(target_arch = "wasm32")]
struct Store {
    pool: crate::db::Pool,
}

struct CryptoKeyValueInner {
    next_pre_key_id: u32,
    next_signed_pre_key_id: u32,
    has_push_signal_keys: bool,
}

const KEY_NEXT_PRE_KEY_ID: &str = "next_pre_key_id";
const KEY_NEXT_SIGNED_PRE_KEY_ID: &str = "next_signed_pre_key_id";
const KEY_HAS_PUSH_SIGNAL_KEYS: &str = "has_push_signal_keys";
#[cfg(not(target_arch = "wasm32"))]
const KEY_LOCAL_REGISTRATION_ID: &str = "local_registration_id";
#[cfg(not(target_arch = "wasm32"))]
const KEY_ACTIVE_SIGNED_PRE_KEY_ID: &str = "active_signed_pre_key_id";

impl CryptoKeyValue {
    /// Opens `<account_directory>/{crypto_box,privacy_box}`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(account_directory: &std::path::Path) -> Result<Self> {
        use crate::db::hive::{account_box, CRYPTO_BOX, PRIVACY_BOX};
        Ok(Self::with_store(Store {
            crypto: account_box(account_directory, CRYPTO_BOX)?,
            privacy: account_box(account_directory, PRIVACY_BOX)?,
        }))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(pool: crate::db::Pool) -> Self {
        Self::with_store(Store { pool })
    }

    fn with_store(store: Store) -> Self {
        CryptoKeyValue {
            store,
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

    /// Mirrors the Flutter login, which records the registration id it sent.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn set_local_registration_id(&self, registration_id: u32) -> Result<()> {
        self.set_property(KEY_LOCAL_REGISTRATION_ID, &registration_id)
            .await
    }

    /// Advances the signed pre-key counter and, on native, records the new
    /// key as active the way the Flutter app's `generateSignedPreKey` does.
    pub async fn set_signed_pre_key(&self, active: u32) -> Result<()> {
        self.set_next_signed_pre_key_id((active + 1) % MAX_VALUE)
            .await?;
        #[cfg(not(target_arch = "wasm32"))]
        self.set_property(KEY_ACTIVE_SIGNED_PRE_KEY_ID, &active)
            .await?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn hive_box(&self, key: &str) -> &crate::db::hive::HiveBox {
        if key == KEY_HAS_PUSH_SIGNAL_KEYS {
            &self.store.privacy
        } else {
            &self.store.crypto
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn get_property<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        Ok(self
            .hive_box(key)
            .get(key)?
            .map(serde_json::from_value)
            .transpose()?)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn set_property<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.hive_box(key).put(key, &serde_json::to_value(value)?)
    }

    #[cfg(target_arch = "wasm32")]
    async fn get_property<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let value: Option<String> =
            crate::db::query_scalar("SELECT value FROM properties WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.store.pool)
                .await?;
        let Some(value) = value else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_reader(BufReader::new(
            value.as_bytes(),
        ))?))
    }

    #[cfg(target_arch = "wasm32")]
    async fn set_property<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let value = serde_json::to_string(value)?;
        crate::db::query(
            "INSERT INTO properties (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.store.pool)
        .await?;
        Ok(())
    }
}
