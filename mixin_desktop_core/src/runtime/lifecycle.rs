//! Account retirement. Authentication loss cannot leave a reusable runtime.
use super::{runner, AccountRuntime, AccountState};
use crate::db::app::Auth;
use anyhow::Result;
use log::warn;
use std::{
    io::ErrorKind,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tokio::sync::watch;

impl AccountRuntime {
    pub async fn revoke_session(&self) {
        if let Err(error) = self
            .client
            .account_api
            .logout(&self.account().session_id)
            .await
        {
            warn!("failed to revoke account session during sign out: {error}");
        }
    }

    pub fn begin_sign_out(&self) {
        self.active.store(false, Ordering::Release);
        self.cancel_attachment_downloads();
    }

    pub(super) async fn prepare_sign_out(&self) -> Result<PathBuf> {
        self.ensure_active()?;
        #[cfg(not(target_arch = "wasm32"))]
        self.screen_lock_service()
            .await?
            .clear_for_sign_out(self.auth_service.clear_auth(&self.account_id))
            .await?;
        // Failed persistence remains retryable while this runtime and MCP live.
        #[cfg(target_arch = "wasm32")]
        self.auth_service.clear_auth(&self.account_id).await?;
        self.begin_sign_out();
        Ok(self.account_data_dir.clone())
    }

    pub(super) async fn complete_sign_out(&self, directory: PathBuf) {
        let _mutation = self.mutation_gate.write().await;
        let session_id = self.account().session_id;
        self.shutdown_inner().await;
        self.revoke_session().await;
        if let Err(error) = self.signal_database.clear().await {
            warn!("failed to clear signal state during sign out: {error}");
        }
        if let Err(error) = self
            .database
            .participant_session_dao
            .clear_for_sign_out(&session_id)
            .await
        {
            warn!("failed to clear participant sessions during sign out: {error}");
        }
        self.signal_database.close().await;
        self.database.close().await;
        if let Err(error) = crate::db::storage::files::remove_directory(&directory).await {
            warn!("failed to clear account storage during sign out: {error}");
        }
        match crate::fs::remove_dir_all(&directory).await {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => warn!("failed to remove signed-out account files: {error}"),
        }
    }

    pub async fn sign_out(&self) -> Result<()> {
        let directory = self.prepare_sign_out().await?;
        self.complete_sign_out(directory).await;
        Ok(())
    }
}

/// Owns no AccountRuntime, so neither a retained service nor runner can cycle
/// back through this task. AccountRuntime::drop explicitly closes its lifetime.
pub(super) async fn monitor_session(
    state: Arc<AccountState>,
    session: Auth,
    shutdown: watch::Sender<bool>,
    mut completion: watch::Receiver<Option<runner::Exit>>,
) {
    let mut closing = shutdown.subscribe();
    let mut authentication = state.client.subscribe_authentication_errors();
    let mut errors = state.client.subscribe_server_error_codes();
    loop {
        if *closing.borrow_and_update() {
            return;
        }
        let code = *errors.borrow_and_update();
        if *authentication.borrow_and_update()
            || code == Some(sdk::err::error_code::AUTHENTICATION)
            || *completion.borrow_and_update() == Some(runner::Exit::Unauthorized)
        {
            // An explicit sign-out already owns its cleanup and presentation.
            if !state.active.swap(false, Ordering::AcqRel) {
                return;
            }
            state.cancel_attachment_downloads();
            if let Err(error) = state.auth_service.invalidate_session(&session).await {
                // Cached authority has already been removed. A later restore
                // must not reuse these credentials while persistence recovers.
                warn!("failed to remove invalid saved session: {error}");
            }
            state.account_health.send_replace("unauthorized".into());
            shutdown.send_replace(true);
            return;
        }
        if completion.borrow().is_some() {
            return;
        }
        let health = match code {
            Some(sdk::err::error_code::TIME_INACCURATE) => Some("time_inaccurate"),
            Some(sdk::err::error_code::OLD_VERSION) => Some("update_required"),
            _ => None,
        };
        if let Some(health) = health {
            state.account_health.send_replace(health.into());
        }
        tokio::select! {
            biased;
            changed = closing.changed() => if changed.is_err() { return; },
            changed = authentication.changed() => if changed.is_err() { return; },
            changed = errors.changed() => if changed.is_err() { return; },
            changed = completion.changed() => if changed.is_err() { return; },
        }
    }
}
