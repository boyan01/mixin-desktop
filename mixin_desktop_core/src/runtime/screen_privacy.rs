//! Coarse account boundary for screen privacy. The credential service stays private.

use anyhow::Result;
use futures::Stream;
use std::sync::Arc;

use super::{
    model::{ScreenLockCommand, ScreenLockState},
    screen_lock::ScreenLockService,
    AccountRuntime, AccountState,
};

impl AccountState {
    pub(super) async fn screen_lock_service(&self) -> Result<Arc<ScreenLockService>> {
        let service = self
            .screen_lock
            .get_or_try_init(|| ScreenLockService::open(self.account_data_dir.clone()))
            .await?;
        Ok(service.clone())
    }
}

impl AccountRuntime {
    pub async fn screen_lock_state(&self) -> Result<ScreenLockState> {
        self.ensure_active()?;
        Ok(self.screen_lock_service().await?.state())
    }

    pub fn screen_lock_changes(
        &self,
    ) -> impl Stream<Item = Result<ScreenLockState>> + Send + 'static {
        let state = self.state.clone();
        async_stream::stream! {
            let service = match state.screen_lock_service().await {
                Ok(service) => service,
                Err(error) => { yield Err(error); return; }
            };
            let mut changes = service.subscribe();
            let mut shutdown = state.shutdown.clone();
            yield Ok(service.state());
            loop {
                tokio::select! {
                    next = changes.changed() => {
                        if next.is_err() { break; }
                        let value = changes.borrow_and_update().clone();
                        yield Ok(value);
                    }
                    next = shutdown.changed() => {
                        if next.is_err() || *shutdown.borrow() { break; }
                    }
                }
            }
        }
    }

    pub async fn screen_lock_command(&self, command: ScreenLockCommand) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let service = self.screen_lock_service().await?;
        match command {
            ScreenLockCommand::SetPasscode(passcode) => service.set_passcode(passcode).await,
            ScreenLockCommand::DisablePasscode => service.disable_passcode().await,
            ScreenLockCommand::Lock => service.lock().await,
            ScreenLockCommand::Unlock(passcode) => service.unlock(passcode).await,
            ScreenLockCommand::SetAutoLockMinutes(minutes) => {
                service.set_auto_lock_minutes(minutes).await
            }
            ScreenLockCommand::SetBiometricEnabled(enabled) => {
                service.set_biometric_enabled(enabled).await
            }
            ScreenLockCommand::UnlockWithBiometrics(reason) => {
                service.unlock_with_biometrics(reason).await
            }
        }
    }
}
