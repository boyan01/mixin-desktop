use std::sync::Arc;

use anyhow::Result;
#[cfg(not(target_os = "macos"))]
use async_trait::async_trait;

use super::{Authenticator, Store};

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod macos;

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
#[path = "security_box.rs"]
mod security_box;

pub(super) fn store(account_directory: &std::path::Path) -> Result<Arc<dyn Store>> {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        Ok(Arc::new(security_box::SecurityBox::open(
            account_directory,
        )?))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = account_directory;
        Ok(Arc::new(Unavailable))
    }
}

pub(super) fn authenticator() -> Arc<dyn Authenticator> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::Biometrics)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Arc::new(Unavailable)
    }
}

#[cfg(not(target_os = "macos"))]
struct Unavailable;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl Store for Unavailable {
    fn load(&self) -> Result<Option<Vec<u8>>> {
        anyhow::bail!("Screen lock is unavailable on this platform")
    }
    fn save(&self, _bytes: &[u8]) -> Result<()> {
        anyhow::bail!("Screen lock is unavailable on this platform")
    }
    fn delete(&self) -> Result<()> {
        anyhow::bail!("Screen lock is unavailable on this platform")
    }
}

#[cfg(not(target_os = "macos"))]
#[async_trait]
impl Authenticator for Unavailable {
    fn available(&self) -> bool {
        false
    }
    async fn authenticate(&self, _reason: String) -> Result<()> {
        anyhow::bail!("Biometric authentication is unavailable on this platform")
    }
}
