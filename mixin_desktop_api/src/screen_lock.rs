//! Account-scoped screen privacy. Credentials and authentication stay in Rust.
use crate::{ClientResult, ScreenLockState};
use futures::{Stream, StreamExt as _};
use mixin_desktop_core::runtime::{model::ScreenLockCommand, AccountRuntime};
use std::sync::Arc;

#[derive(Clone)]
pub struct ScreenLockClient {
    pub(crate) runtime: Arc<AccountRuntime>,
}

impl ScreenLockClient {
    pub async fn state(&self) -> ClientResult<ScreenLockState> {
        Ok(self.runtime.screen_lock_state().await?)
    }
    pub fn changes(&self) -> impl Stream<Item = ClientResult<ScreenLockState>> + Send + 'static {
        self.runtime
            .screen_lock_changes()
            .map(|result| result.map_err(Into::into))
    }
    pub async fn set_passcode(&self, passcode: String) -> ClientResult<()> {
        self.command(ScreenLockCommand::SetPasscode(passcode)).await
    }
    pub async fn disable_passcode(&self) -> ClientResult<()> {
        self.command(ScreenLockCommand::DisablePasscode).await
    }
    pub async fn lock(&self) -> ClientResult<()> {
        self.command(ScreenLockCommand::Lock).await
    }
    pub async fn unlock(&self, passcode: String) -> ClientResult<()> {
        self.command(ScreenLockCommand::Unlock(passcode)).await
    }
    pub async fn set_auto_lock_minutes(&self, minutes: u32) -> ClientResult<()> {
        self.command(ScreenLockCommand::SetAutoLockMinutes(minutes))
            .await
    }
    pub async fn set_biometric_enabled(&self, enabled: bool) -> ClientResult<()> {
        self.command(ScreenLockCommand::SetBiometricEnabled(enabled))
            .await
    }
    pub async fn unlock_with_biometrics(&self, reason: String) -> ClientResult<()> {
        self.command(ScreenLockCommand::UnlockWithBiometrics(reason))
            .await
    }
    async fn command(&self, command: ScreenLockCommand) -> ClientResult<()> {
        Ok(self.runtime.screen_lock_command(command).await?)
    }
}
