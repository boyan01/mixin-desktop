//! The Flutter app's per-account `security_box`, shared as the screen lock
//! store. Retry throttling has no Flutter equivalent; it is kept in the same
//! box under keys the Flutter app does not read.
use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::Store;
use crate::db::hive::{account_box, HiveBox, SECURITY_BOX};

const PASSCODE: &str = "passcode";
const BIOMETRIC: &str = "biometric";
const LOCK_DURATION: &str = "lockDuration";
const FAILED_ATTEMPTS: &str = "nshFailedAttempts";
const RETRY_UNTIL_MILLIS: &str = "nshRetryUntilMillis";
/// `SecurityKeyValue.lockDuration` when no duration was saved.
const DEFAULT_LOCK_MINUTES: u64 = 1;

pub(super) struct SecurityBox(Arc<HiveBox>);

impl SecurityBox {
    pub(super) fn open(account_directory: &std::path::Path) -> Result<Self> {
        Ok(Self(account_box(account_directory, SECURITY_BOX)?))
    }

    fn number(&self, key: &str) -> Result<Option<u64>> {
        Ok(self.0.get(key)?.and_then(|value| value.as_u64()))
    }
}

impl Store for SecurityBox {
    fn load(&self) -> Result<Option<Vec<u8>>> {
        let Some(passcode) = self.0.get(PASSCODE)?.and_then(|value| match value {
            Value::String(passcode) => Some(passcode),
            _ => None,
        }) else {
            return Ok(None);
        };
        let record = json!({
            "passcode": passcode,
            "auto_lock_minutes": self.number(LOCK_DURATION)?.unwrap_or(DEFAULT_LOCK_MINUTES),
            "biometric_enabled": self.0.get(BIOMETRIC)?.and_then(|value| value.as_bool()).unwrap_or(false),
            "failed_attempts": self.number(FAILED_ATTEMPTS)?.unwrap_or(0),
            "retry_until_millis": self.number(RETRY_UNTIL_MILLIS)?.unwrap_or(0),
        });
        Ok(Some(serde_json::to_vec(&record)?))
    }

    fn save(&self, bytes: &[u8]) -> Result<()> {
        let record: Value =
            serde_json::from_slice(bytes).context("Could not encode screen lock credentials")?;
        let field = |key: &str| record.get(key).cloned().unwrap_or(Value::Null);
        self.0.write(&[
            (PASSCODE, Some(&field("passcode"))),
            (BIOMETRIC, Some(&field("biometric_enabled"))),
            (LOCK_DURATION, Some(&field("auto_lock_minutes"))),
            (FAILED_ATTEMPTS, Some(&field("failed_attempts"))),
            (RETRY_UNTIL_MILLIS, Some(&field("retry_until_millis"))),
        ])
    }

    /// Matches `SecurityKeyValue.passcode = null`, which also resets the lock
    /// duration and biometrics.
    fn delete(&self) -> Result<()> {
        self.0.write(&[
            (PASSCODE, Some(&Value::Null)),
            (LOCK_DURATION, Some(&Value::Null)),
            (BIOMETRIC, Some(&Value::Bool(false))),
            (FAILED_ATTEMPTS, None),
            (RETRY_UNTIL_MILLIS, None),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flutter_passcode_settings_load_and_disable_like_security_key_value() {
        let directory = tempfile::tempdir().unwrap();
        let hive_box = account_box(directory.path(), SECURITY_BOX).unwrap();
        hive_box.put(PASSCODE, &json!("123456")).unwrap();
        hive_box.put(LOCK_DURATION, &json!(60)).unwrap();
        hive_box.put(BIOMETRIC, &json!(true)).unwrap();
        let store = SecurityBox::open(directory.path()).unwrap();

        let record: Value = serde_json::from_slice(&store.load().unwrap().unwrap()).unwrap();
        assert_eq!(record["passcode"], "123456");
        assert_eq!(record["auto_lock_minutes"], 60);
        assert_eq!(record["biometric_enabled"], true);

        store.delete().unwrap();
        assert_eq!(store.load().unwrap(), None);
        assert_eq!(hive_box.get(PASSCODE).unwrap(), Some(Value::Null));
        assert_eq!(hive_box.get(LOCK_DURATION).unwrap(), Some(Value::Null));
        assert_eq!(hive_box.get(BIOMETRIC).unwrap(), Some(json!(false)));
    }

    #[test]
    fn saved_settings_use_flutter_keys_and_value_types() {
        let directory = tempfile::tempdir().unwrap();
        let store = SecurityBox::open(directory.path()).unwrap();
        store
            .save(
                br#"{"passcode":"654321","auto_lock_minutes":5,"biometric_enabled":false,"failed_attempts":0,"retry_until_millis":0}"#,
            )
            .unwrap();
        let hive_box = account_box(directory.path(), SECURITY_BOX).unwrap();
        assert_eq!(hive_box.get(PASSCODE).unwrap(), Some(json!("654321")));
        assert!(hive_box.get(LOCK_DURATION).unwrap().unwrap().is_i64());
        assert_eq!(hive_box.get(BIOMETRIC).unwrap(), Some(json!(false)));
    }
}
