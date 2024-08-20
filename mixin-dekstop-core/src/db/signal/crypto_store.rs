use std::sync::{Arc, Mutex};

use rand::rngs::OsRng;
use rand::Rng;

use crate::core::crypto::signal_protocol::MAX_VALUE;
use crate::db::key_value::KeyValue;

pub struct CryptoKeyValue {
    key_value: KeyValue,
    inner: Arc<Mutex<CryptoKeyValueInner>>,
}

struct CryptoKeyValueInner {
    next_pre_key_id: u32,
    next_signed_pre_key_id: u32,
    has_push_signal_keys: bool,
}

const GROUP: &str = "crypto";
const KEY_NEXT_PRE_KEY_ID: &str = "next_pre_key_id";
const KEY_NEXT_SIGNED_PRE_KEY_ID: &str = "next_signed_pre_key_id";
const KEY_HAS_PUSH_SIGNAL_KEYS: &str = "has_push_signal_keys";

impl CryptoKeyValue {
    pub fn new(key_value: KeyValue) -> Self {
        CryptoKeyValue {
            key_value,
            inner: Arc::new(Mutex::new(CryptoKeyValueInner {
                next_pre_key_id: 0,
                next_signed_pre_key_id: 0,
                has_push_signal_keys: false,
            })),
        }
    }

    pub async fn init(&self) {
        let next_pre_key_id: u32 = self
            .key_value
            .get_value(KEY_NEXT_PRE_KEY_ID, GROUP)
            .await
            .unwrap_or(OsRng.gen_range(0..MAX_VALUE));
        let next_signed_pre_key_id: u32 = self
            .key_value
            .get_value(KEY_NEXT_SIGNED_PRE_KEY_ID, GROUP)
            .await
            .unwrap_or(OsRng.gen_range(0..MAX_VALUE));
        let has_push_signal_keys: bool = self
            .key_value
            .get_value(KEY_HAS_PUSH_SIGNAL_KEYS, GROUP)
            .await
            .unwrap_or(false);

        let mut inner = self.inner.lock().unwrap();
        inner.next_pre_key_id = next_pre_key_id;
        inner.next_signed_pre_key_id = next_signed_pre_key_id;
        inner.has_push_signal_keys = has_push_signal_keys;
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

    pub async fn set_next_pre_key_id(&self, next_pre_key_id: u32) {
        self.inner.lock().unwrap().next_pre_key_id = next_pre_key_id;
        self.key_value
            .set_value(KEY_NEXT_PRE_KEY_ID, GROUP, &next_pre_key_id)
            .await;
    }

    pub async fn set_next_signed_pre_key_id(&self, next_signed_pre_key_id: u32) {
        self.inner.lock().unwrap().next_signed_pre_key_id = next_signed_pre_key_id;
        self.key_value
            .set_value(KEY_NEXT_SIGNED_PRE_KEY_ID, GROUP, &next_signed_pre_key_id)
            .await;
    }

    pub async fn set_has_push_signal_keys(&self, has_push_signal_keys: bool) {
        self.inner.lock().unwrap().has_push_signal_keys = has_push_signal_keys;
        self.key_value
            .set_value(KEY_HAS_PUSH_SIGNAL_KEYS, GROUP, &has_push_signal_keys)
            .await;
    }
}
