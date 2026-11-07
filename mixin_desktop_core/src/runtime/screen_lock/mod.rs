//! Account-scoped screen privacy credentials. Background account work continues.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use tokio::sync::{watch, Mutex as AsyncMutex};
use zeroize::Zeroizing;

use super::model::ScreenLockState;

mod platform;
mod record;
#[cfg(test)]
pub(super) mod tests;

trait Store: Send + Sync {
    fn load(&self) -> Result<Option<Vec<u8>>>;
    fn save(&self, bytes: &[u8]) -> Result<()>;
    fn delete(&self) -> Result<()>;
}

#[async_trait]
trait Authenticator: Send + Sync {
    fn available(&self) -> bool;
    async fn authenticate(&self, reason: String) -> Result<()>;
}

struct Inner {
    record: Option<record::Record>,
    locked: bool,
}

pub(crate) struct ScreenLockService {
    available: bool,
    store: Arc<dyn Store>,
    authenticator: Arc<dyn Authenticator>,
    biometrics_available: AtomicBool,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    command: AsyncMutex<()>,
    inner: Mutex<Inner>,
    changes: watch::Sender<ScreenLockState>,
}

impl ScreenLockService {
    pub(crate) async fn open(account_id: String, namespace: String) -> Result<Arc<Self>> {
        if account_id.is_empty() || namespace.is_empty() {
            bail!("Screen lock account and namespace are required");
        }
        Self::open_with(
            platform::store(account_id, namespace)?,
            platform::authenticator(),
            cfg!(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux"
            )),
            Arc::new(now_millis),
        )
        .await
    }

    async fn open_with(
        store: Arc<dyn Store>,
        authenticator: Arc<dyn Authenticator>,
        available: bool,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Result<Arc<Self>> {
        let read = store.clone();
        let auth = authenticator.clone();
        let (bytes, biometrics_available) = if available {
            tokio::task::spawn_blocking(move || {
                Ok::<_, anyhow::Error>((read.load()?, auth.available()))
            })
            .await??
        } else {
            (None, false)
        };
        let record = bytes
            .map(|bytes| {
                if bytes.len() > 4096 {
                    bail!("Invalid screen lock credential record");
                }
                serde_json::from_slice::<record::Record>(&bytes)
                    .context("Invalid screen lock credential record")
            })
            .transpose()?;
        if let Some(record) = &record {
            record.validate()?;
        }
        let locked = record.is_some();
        let (changes, _) = watch::channel(ScreenLockState::default());
        let service = Arc::new(Self {
            available,
            store,
            authenticator,
            biometrics_available: AtomicBool::new(biometrics_available),
            clock,
            command: AsyncMutex::new(()),
            inner: Mutex::new(Inner { record, locked }),
            changes,
        });
        service.publish();
        Ok(service)
    }

    pub(crate) fn state(&self) -> ScreenLockState {
        let inner = self.inner.lock().expect("screen lock state");
        ScreenLockState {
            available: self.available,
            enabled: inner.record.is_some(),
            locked: inner.locked,
            auto_lock_minutes: inner
                .record
                .as_ref()
                .map_or(0, |record| record.auto_lock_minutes),
            biometrics_available: self.biometrics_available.load(Ordering::Relaxed),
            biometric_enabled: inner
                .record
                .as_ref()
                .is_some_and(|record| record.biometric_enabled),
            retry_after_millis: inner.record.as_ref().map_or(0, |record| {
                record.retry_until_millis.saturating_sub((self.clock)())
            }),
        }
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<ScreenLockState> {
        self.changes.subscribe()
    }

    pub(crate) async fn set_passcode(&self, passcode: String) -> Result<()> {
        let passcode = Zeroizing::new(passcode);
        let _command = self.command.lock().await;
        self.ensure_available()?;
        self.ensure_unlocked()?;
        record::validate_passcode(&passcode)?;
        let previous = self.record();
        let next = tokio::task::spawn_blocking(move || record::Record::create(&passcode, previous))
            .await??;
        self.persist(&next).await?;
        {
            self.inner.lock().expect("screen lock state").record = Some(next);
        }
        self.publish();
        Ok(())
    }

    pub(crate) async fn disable_passcode(&self) -> Result<()> {
        let _command = self.command.lock().await;
        self.ensure_available()?;
        self.ensure_unlocked()?;
        self.delete().await
    }

    pub(crate) async fn lock(&self) -> Result<()> {
        let _command = self.command.lock().await;
        self.ensure_available()?;
        {
            let mut inner = self.inner.lock().expect("screen lock state");
            inner.locked = inner.record.is_some();
        }
        self.publish();
        Ok(())
    }

    pub(crate) async fn unlock(&self, passcode: String) -> Result<()> {
        let passcode = Zeroizing::new(passcode);
        let _command = self.command.lock().await;
        self.ensure_available()?;
        if !self.state().locked {
            return Ok(());
        }
        record::validate_passcode(&passcode)?;
        let mut record = self.record().context("Screen passcode is not configured")?;
        let now = (self.clock)();
        if record.retry_until_millis > now {
            bail!("Wait before trying the screen passcode again");
        }
        let verify = record.clone();
        let matches = tokio::task::spawn_blocking(move || verify.matches(&passcode)).await?;
        if matches {
            record.failed_attempts = 0;
            record.retry_until_millis = 0;
            self.persist(&record).await?;
            {
                let mut inner = self.inner.lock().expect("screen lock state");
                inner.record = Some(record);
                inner.locked = false;
            }
            self.publish();
            Ok(())
        } else {
            record.fail((self.clock)());
            self.persist(&record).await?;
            self.inner.lock().expect("screen lock state").record = Some(record);
            self.publish();
            bail!("Incorrect screen passcode")
        }
    }

    pub(crate) async fn set_auto_lock_minutes(&self, minutes: u32) -> Result<()> {
        let _command = self.command.lock().await;
        self.ensure_available()?;
        self.ensure_unlocked()?;
        record::validate_minutes(minutes)?;
        let mut record = self.record().context("Screen passcode is not configured")?;
        record.auto_lock_minutes = minutes;
        self.persist(&record).await?;
        self.inner.lock().expect("screen lock state").record = Some(record);
        self.publish();
        Ok(())
    }

    pub(crate) async fn set_biometric_enabled(&self, enabled: bool) -> Result<()> {
        let _command = self.command.lock().await;
        self.ensure_available()?;
        self.ensure_unlocked()?;
        let auth = self.authenticator.clone();
        let available = tokio::task::spawn_blocking(move || auth.available()).await?;
        self.biometrics_available
            .store(available, Ordering::Relaxed);
        if enabled && !available {
            self.publish();
            bail!("Biometric authentication is unavailable");
        }
        let mut record = self.record().context("Screen passcode is not configured")?;
        record.biometric_enabled = enabled;
        self.persist(&record).await?;
        self.inner.lock().expect("screen lock state").record = Some(record);
        self.publish();
        Ok(())
    }

    pub(crate) async fn unlock_with_biometrics(&self, reason: String) -> Result<()> {
        let _command = self.command.lock().await;
        self.ensure_available()?;
        if !self.state().locked {
            return Ok(());
        }
        let mut record = self.record().context("Screen passcode is not configured")?;
        if !record.biometric_enabled {
            bail!("Biometric authentication is disabled");
        }
        if reason.trim().is_empty() {
            bail!("Biometric authentication requires an explanation");
        }
        self.authenticator.authenticate(reason).await?;
        record.failed_attempts = 0;
        record.retry_until_millis = 0;
        self.persist(&record).await?;
        {
            let mut inner = self.inner.lock().expect("screen lock state");
            inner.record = Some(record);
            inner.locked = false;
        }
        self.publish();
        Ok(())
    }

    pub(crate) async fn clear_for_sign_out(
        &self,
        remove_auth: impl std::future::Future<Output = Result<()>>,
    ) -> Result<()> {
        let _command = self.command.lock().await;
        if !self.available {
            return remove_auth.await;
        }
        let record = self.record();
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.delete()).await??;
        if let Err(error) = remove_auth.await {
            if let Some(record) = record {
                if let Err(restore) = self.persist(&record).await {
                    // Keep the known verifier and cover content if both stores
                    // fail. Recovery can restore the credential before unlock.
                    self.inner.lock().expect("screen lock state").locked = true;
                    self.publish();
                    return Err(error.context(format!(
                        "Could not restore screen credentials after failed sign-out: {restore}"
                    )));
                }
            }
            return Err(error);
        }
        {
            let mut inner = self.inner.lock().expect("screen lock state");
            inner.record = None;
            inner.locked = false;
        }
        self.publish();
        Ok(())
    }

    fn ensure_available(&self) -> Result<()> {
        if !self.available {
            bail!("Screen lock is unavailable on this platform");
        }
        Ok(())
    }
    fn ensure_unlocked(&self) -> Result<()> {
        if self.inner.lock().expect("screen lock state").locked {
            bail!("Unlock before changing screen privacy settings");
        }
        Ok(())
    }
    fn record(&self) -> Option<record::Record> {
        self.inner.lock().expect("screen lock state").record.clone()
    }
    fn publish(&self) {
        let next = self.state();
        self.changes.send_if_modified(|state| {
            if *state == next {
                false
            } else {
                *state = next;
                true
            }
        });
    }
    async fn persist(&self, record: &record::Record) -> Result<()> {
        let bytes =
            serde_json::to_vec(record).context("Could not encode screen lock credentials")?;
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.save(&bytes)).await?
    }
    async fn delete(&self) -> Result<()> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.delete()).await??;
        {
            let mut inner = self.inner.lock().expect("screen lock state");
            inner.record = None;
            inner.locked = false;
        }
        self.publish();
        Ok(())
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
