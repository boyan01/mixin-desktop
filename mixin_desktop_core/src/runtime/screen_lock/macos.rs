use std::sync::mpsc;

use super::super::Authenticator;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use block2::RcBlock;
use objc2_06::{rc::autoreleasepool, runtime::Bool};
use objc2_foundation_03::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAPolicy};

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
