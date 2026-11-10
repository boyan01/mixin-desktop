//! Browser builds have no platform credential store: screen lock is unavailable.
use crate::{ClientError, ClientResult, ScreenLockState};
use futures::Stream;
use mixin_desktop_core::runtime::AccountRuntime;
use std::sync::Arc;

#[derive(Clone)]
pub struct ScreenLockClient {
    // Kept so the account constructs this client the same way on every target.
    #[allow(dead_code)]
    pub(crate) runtime: Arc<AccountRuntime>,
}

fn unsupported<T>() -> ClientResult<T> {
    Err(ClientError::Internal(
        "screen lock is not supported in the browser".into(),
    ))
}

impl ScreenLockClient {
    pub async fn state(&self) -> ClientResult<ScreenLockState> {
        unsupported()
    }
    pub fn changes(&self) -> impl Stream<Item = ClientResult<ScreenLockState>> + Send + 'static {
        futures::stream::empty()
    }
    pub async fn set_passcode(&self, _passcode: String) -> ClientResult<()> {
        unsupported()
    }
    pub async fn disable_passcode(&self) -> ClientResult<()> {
        unsupported()
    }
    pub async fn lock(&self) -> ClientResult<()> {
        unsupported()
    }
    pub async fn unlock(&self, _passcode: String) -> ClientResult<()> {
        unsupported()
    }
    pub async fn set_auto_lock_minutes(&self, _minutes: u32) -> ClientResult<()> {
        unsupported()
    }
    pub async fn set_biometric_enabled(&self, _enabled: bool) -> ClientResult<()> {
        unsupported()
    }
    pub async fn unlock_with_biometrics(&self, _reason: String) -> ClientResult<()> {
        unsupported()
    }
}
