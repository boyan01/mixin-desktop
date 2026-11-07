use std::sync::mpsc;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use block2::RcBlock;
use objc2_06::{rc::autoreleasepool, runtime::Bool};
use objc2_foundation_03::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAPolicy};
use security_framework::passwords::{
    delete_generic_password_options, generic_password, set_generic_password_options,
    PasswordOptions,
};

use super::super::{Authenticator, Store};

// Security.framework's errSecItemNotFound. All other statuses remain errors.
const ITEM_NOT_FOUND: i32 = -25300;

pub(super) struct Keychain {
    service: String,
    account: String,
}

impl Keychain {
    pub fn new(account: String, namespace: String) -> Self {
        let digest = ring::digest::digest(&ring::digest::SHA256, namespace.as_bytes());
        Self {
            service: format!(
                "one.mixin.messenger.screen-lock.{}",
                hex::encode(digest.as_ref())
            ),
            account,
        }
    }

    fn options(&self) -> PasswordOptions {
        let mut options = PasswordOptions::new_generic_password(&self.service, &self.account);
        options.set_access_synchronized(Some(false));
        options
    }
}

impl Store for Keychain {
    fn load(&self) -> Result<Option<Vec<u8>>> {
        match generic_password(self.options()) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(error) => {
                Err(error).context("Could not read screen lock credentials from Keychain")
            }
        }
    }

    fn save(&self, bytes: &[u8]) -> Result<()> {
        set_generic_password_options(bytes, self.options())
            .context("Could not save screen lock credentials to Keychain")
    }

    fn delete(&self) -> Result<()> {
        match delete_generic_password_options(self.options()) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
            Err(error) => {
                Err(error).context("Could not remove screen lock credentials from Keychain")
            }
        }
    }
}

pub(super) struct Biometrics;

enum Reply {
    Approved,
    Rejected(String),
    Cancelled,
}

struct CancelOnDrop(mpsc::Sender<Reply>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(Reply::Cancelled);
    }
}

#[async_trait]
impl Authenticator for Biometrics {
    fn available(&self) -> bool {
        autoreleasepool(|_| {
            // SAFETY: The context stays on this thread, is queried once, and is
            // released at the end of this pool. No authentication prompt is shown.
            unsafe {
                LAContext::new()
                    .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
                    .is_ok()
            }
        })
    }

    async fn authenticate(&self, reason: String) -> Result<()> {
        let (sender, receiver) = mpsc::channel();
        let _cancel = CancelOnDrop(sender.clone());
        tokio::task::spawn_blocking(move || {
            autoreleasepool(|_| {
                // SAFETY: This worker owns the context for the entire evaluation.
                // Only plain Rust results cross the callback's thread boundary.
                let context = unsafe { LAContext::new() };
                unsafe {
                    context
                        .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
                }
                .map_err(|_| anyhow::anyhow!("Biometric authentication is unavailable"))?;
                let completion = RcBlock::new(move |approved: Bool, error: *mut NSError| {
                    let reply = if approved.as_bool() {
                        Reply::Approved
                    } else if error.is_null() {
                        Reply::Rejected("Biometric authentication was not approved".into())
                    } else {
                        // SAFETY: Apple keeps this NSError valid for this callback.
                        Reply::Rejected(unsafe { &*error }.localizedDescription().to_string())
                    };
                    let _ = sender.send(reply);
                });
                // SAFETY: The reply captures only a thread-safe Rust sender. This
                // biometric-only policy never accepts the desktop screen passcode as
                // an unverified boolean supplied by presentation code.
                unsafe {
                    context.setLocalizedFallbackTitle(Some(&NSString::from_str("")));
                    context.evaluatePolicy_localizedReason_reply(
                        LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
                        &NSString::from_str(&reason),
                        &completion,
                    );
                }
                let reply = receiver
                    .recv()
                    .context("Biometric authentication ended without a result");
                // SAFETY: Invalidation occurs on the context's owning worker, never
                // from inside Apple's reply block. Cancellation wakes this worker.
                unsafe {
                    context.invalidate();
                }
                match reply? {
                    Reply::Approved => Ok(()),
                    Reply::Rejected(message) => bail!(message),
                    Reply::Cancelled => bail!("Biometric authentication was cancelled"),
                }
            })
        })
        .await?
    }
}
