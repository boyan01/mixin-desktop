// Windows Credential Manager and Linux Secret Service share the same contract.
use anyhow::{Context, Result};

use super::super::Store;

pub(super) struct CredentialStore {
    entry: keyring::Entry,
}

impl CredentialStore {
    pub(super) fn new(account: String, namespace: String) -> Result<Self> {
        let digest = ring::digest::digest(&ring::digest::SHA256, namespace.as_bytes());
        let service = format!(
            "one.mixin.messenger.screen-lock.{}",
            hex::encode(digest.as_ref())
        );
        Ok(Self {
            entry: keyring::Entry::new(&service, &account).context(
                "Could not address screen lock credentials in the system credential store",
            )?,
        })
    }
}

impl Store for CredentialStore {
    fn load(&self) -> Result<Option<Vec<u8>>> {
        match self.entry.get_secret() {
            Ok(bytes) => Ok(Some(bytes)),
            Err(keyring::Error::NoEntry) => Ok(None),
            // A locked or disconnected provider cannot establish that no code
            // was configured. Keep that error so presentation stays covered.
            Err(error) => Err(error)
                .context("Could not read screen lock credentials from the system credential store"),
        }
    }

    fn save(&self, bytes: &[u8]) -> Result<()> {
        self.entry
            .set_secret(bytes)
            .context("Could not save screen lock credentials to the system credential store")
    }

    fn delete(&self) -> Result<()> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error).context(
                "Could not remove screen lock credentials from the system credential store",
            ),
        }
    }
}
