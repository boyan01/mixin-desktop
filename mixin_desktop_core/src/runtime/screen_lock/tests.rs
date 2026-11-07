use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

#[derive(Default)]
struct MemoryStore {
    bytes: Mutex<Option<Vec<u8>>>,
    read_failed: AtomicBool,
    write_failed: AtomicBool,
    delete_failed: AtomicBool,
}

impl Store for MemoryStore {
    fn load(&self) -> Result<Option<Vec<u8>>> {
        if self.read_failed.load(Ordering::Relaxed) {
            bail!("read rejected");
        }
        Ok(self.bytes.lock().unwrap().clone())
    }
    fn save(&self, bytes: &[u8]) -> Result<()> {
        if self.write_failed.load(Ordering::Relaxed) {
            bail!("write rejected");
        }
        *self.bytes.lock().unwrap() = Some(bytes.to_vec());
        Ok(())
    }
    fn delete(&self) -> Result<()> {
        if self.delete_failed.load(Ordering::Relaxed) {
            bail!("delete rejected");
        }
        *self.bytes.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Default)]
struct TestBiometrics {
    expected: bool,
    approve: bool,
    reasons: Mutex<Vec<String>>,
}
#[async_trait]
impl Authenticator for TestBiometrics {
    fn available(&self) -> bool {
        true
    }
    async fn authenticate(&self, reason: String) -> Result<()> {
        assert!(self.expected, "unexpected biometric authentication");
        self.reasons.lock().unwrap().push(reason);
        if self.approve {
            Ok(())
        } else {
            bail!("user cancelled")
        }
    }
}

async fn service(store: Arc<MemoryStore>, now: Arc<AtomicU64>) -> Arc<ScreenLockService> {
    ScreenLockService::open_with(
        store,
        Arc::new(TestBiometrics::default()),
        true,
        Arc::new(move || now.load(Ordering::Relaxed)),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn configured_passcode_reopens_locked_and_only_the_correct_code_unlocks() {
    let store = Arc::new(MemoryStore::default());
    let now = Arc::new(AtomicU64::new(10_000));
    let first = service(store.clone(), now.clone()).await;
    first.set_passcode("012345".into()).await.unwrap();
    assert!(first.state().enabled);
    assert!(!first.state().locked);
    let reopened = service(store, now).await;
    assert!(reopened.state().locked);
    assert!(reopened.unlock("999999".into()).await.is_err());
    assert!(reopened.state().locked);
    reopened.unlock("012345".into()).await.unwrap();
    assert!(!reopened.state().locked);
}

#[tokio::test]
async fn failed_attempt_backoff_survives_reopen_and_expires_at_the_recorded_deadline() {
    let store = Arc::new(MemoryStore::default());
    let now = Arc::new(AtomicU64::new(10_000));
    let first = service(store.clone(), now.clone()).await;
    first.set_passcode("123456".into()).await.unwrap();
    first.lock().await.unwrap();
    for _ in 0..5 {
        assert!(first.unlock("999999".into()).await.is_err());
    }
    assert_eq!(first.state().retry_after_millis, 1000);
    let reopened = service(store, now.clone()).await;
    assert_eq!(reopened.state().retry_after_millis, 1000);
    assert!(reopened.unlock("123456".into()).await.is_err());
    assert!(reopened.state().locked);
    now.store(11_000, Ordering::Relaxed);
    reopened.unlock("123456".into()).await.unwrap();
    assert!(!reopened.state().locked);
    assert_eq!(reopened.state().retry_after_millis, 0);
}

#[tokio::test]
async fn storage_read_or_corruption_fails_closed_instead_of_disabling_the_passcode() {
    let store = Arc::new(MemoryStore::default());
    store.read_failed.store(true, Ordering::Relaxed);
    assert!(ScreenLockService::open_with(
        store.clone(),
        Arc::new(TestBiometrics::default()),
        true,
        Arc::new(|| 0)
    )
    .await
    .is_err());
    store.read_failed.store(false, Ordering::Relaxed);
    *store.bytes.lock().unwrap() = Some(b"not a credential record".to_vec());
    assert!(ScreenLockService::open_with(
        store,
        Arc::new(TestBiometrics::default()),
        true,
        Arc::new(|| 0)
    )
    .await
    .is_err());
}

#[tokio::test]
async fn unavailable_configured_store_retries_locked_after_provider_recovers() {
    let store = Arc::new(MemoryStore::default());
    let now = Arc::new(AtomicU64::new(0));
    let configured = service(store.clone(), now.clone()).await;
    configured.set_passcode("012345".into()).await.unwrap();
    store.read_failed.store(true, Ordering::Relaxed);

    let unavailable = ScreenLockService::open_with(
        store.clone(),
        Arc::new(TestBiometrics::default()),
        true,
        Arc::new(|| 0),
    )
    .await;
    assert!(unavailable.is_err());

    store.read_failed.store(false, Ordering::Relaxed);
    let recovered = service(store, now).await;
    assert!(recovered.state().available);
    assert!(recovered.state().enabled);
    assert!(recovered.state().locked);
    assert!(recovered.unlock("999999".into()).await.is_err());
    assert!(recovered.state().locked);
    recovered.unlock("012345".into()).await.unwrap();
    assert!(!recovered.state().locked);
}

#[tokio::test]
async fn rejected_credential_writes_do_not_publish_configuration_or_unlock() {
    let store = Arc::new(MemoryStore::default());
    let client = service(store.clone(), Arc::new(AtomicU64::new(0))).await;
    store.write_failed.store(true, Ordering::Relaxed);
    assert!(client.set_passcode("123456".into()).await.is_err());
    assert!(!client.state().enabled);
    store.write_failed.store(false, Ordering::Relaxed);
    client.set_passcode("123456".into()).await.unwrap();
    client.lock().await.unwrap();
    store.write_failed.store(true, Ordering::Relaxed);
    assert!(client.unlock("123456".into()).await.is_err());
    assert!(client.state().locked);
}

#[tokio::test]
async fn disabling_clears_automatic_lock_and_biometrics_only_after_credential_deletion() {
    let store = Arc::new(MemoryStore::default());
    let client = service(store.clone(), Arc::new(AtomicU64::new(0))).await;
    client.set_passcode("123456".into()).await.unwrap();
    client.set_auto_lock_minutes(5).await.unwrap();
    client.set_biometric_enabled(true).await.unwrap();
    store.delete_failed.store(true, Ordering::Relaxed);
    assert!(client.disable_passcode().await.is_err());
    assert!(client.state().enabled);
    store.delete_failed.store(false, Ordering::Relaxed);
    client.disable_passcode().await.unwrap();
    let state = client.state();
    assert!(!state.enabled && !state.locked && !state.biometric_enabled);
    assert_eq!(state.auto_lock_minutes, 0);
    assert!(store.load().unwrap().is_none());
}

#[tokio::test]
async fn locked_configuration_and_cancelled_biometrics_never_unlock_the_account_surface() {
    let biometrics = Arc::new(TestBiometrics {
        expected: true,
        ..Default::default()
    });
    let client = ScreenLockService::open_with(
        Arc::new(MemoryStore::default()),
        biometrics.clone(),
        true,
        Arc::new(|| 0),
    )
    .await
    .unwrap();
    client.set_passcode("123456".into()).await.unwrap();
    client.set_biometric_enabled(true).await.unwrap();
    client.lock().await.unwrap();
    assert!(client.set_passcode("654321".into()).await.is_err());
    assert!(client.disable_passcode().await.is_err());
    assert!(client.set_auto_lock_minutes(0).await.is_err());
    assert!(client.set_biometric_enabled(false).await.is_err());
    assert!(client
        .unlock_with_biometrics("Unlock Mixin".into())
        .await
        .is_err());
    assert!(client.state().locked);
    assert_eq!(*biometrics.reasons.lock().unwrap(), vec!["Unlock Mixin"]);
}

#[tokio::test]
async fn invalid_passcode_and_lock_duration_leave_the_previous_configuration_intact() {
    let client = service(
        Arc::new(MemoryStore::default()),
        Arc::new(AtomicU64::new(0)),
    )
    .await;
    for code in ["12345", "abcdef", "１２３４５６", "1234567"] {
        assert!(client.set_passcode(code.into()).await.is_err());
    }
    assert!(!client.state().enabled);
    client.set_passcode("123456".into()).await.unwrap();
    for minutes in [2, 30, 301, u32::MAX] {
        assert!(client.set_auto_lock_minutes(minutes).await.is_err());
    }
    assert_eq!(client.state().auto_lock_minutes, 1);
}

#[tokio::test]
async fn approved_native_biometrics_unlocks_and_keeps_the_configured_passcode_on_reopen() {
    let store = Arc::new(MemoryStore::default());
    let biometrics = Arc::new(TestBiometrics {
        expected: true,
        approve: true,
        ..Default::default()
    });
    let client =
        ScreenLockService::open_with(store.clone(), biometrics.clone(), true, Arc::new(|| 0))
            .await
            .unwrap();
    client.set_passcode("123456".into()).await.unwrap();
    client.set_biometric_enabled(true).await.unwrap();
    client.lock().await.unwrap();
    client
        .unlock_with_biometrics("Unlock Mixin".into())
        .await
        .unwrap();
    assert!(!client.state().locked);
    assert!(client.state().enabled && client.state().biometric_enabled);
    assert_eq!(*biometrics.reasons.lock().unwrap(), vec!["Unlock Mixin"]);
    let reopened = ScreenLockService::open_with(store, biometrics, true, Arc::new(|| 0))
        .await
        .unwrap();
    assert!(reopened.state().locked);
    reopened.unlock("123456".into()).await.unwrap();
    assert!(!reopened.state().locked);
}

pub(in crate::runtime) async fn deletion_failure_fixture() -> (
    Arc<ScreenLockService>,
    impl Fn(bool),
    impl Fn(bool),
    impl Fn() -> Option<Vec<u8>>,
) {
    let store = Arc::new(MemoryStore::default());
    let service = service(store.clone(), Arc::new(AtomicU64::new(10_000))).await;
    service.set_passcode("024680".into()).await.unwrap();
    let deletion = store.clone();
    let set_delete_failure = move |failed| deletion.delete_failed.store(failed, Ordering::Relaxed);
    let saving = store.clone();
    let set_save_failure = move |failed| saving.write_failed.store(failed, Ordering::Relaxed);
    let stored = move || store.bytes.lock().unwrap().clone();
    (service, set_delete_failure, set_save_failure, stored)
}
