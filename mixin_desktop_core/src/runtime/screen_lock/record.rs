use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Screen privacy settings as the Flutter app keeps them in `security_box`:
/// the six-digit passcode itself, its lock duration, and biometrics.
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Record {
    passcode: String,
    pub auto_lock_minutes: u32,
    pub biometric_enabled: bool,
    pub failed_attempts: u32,
    pub retry_until_millis: u64,
}

impl Record {
    pub fn create(passcode: &str, previous: Option<Self>) -> Result<Self> {
        validate_passcode(passcode)?;
        Ok(Self {
            passcode: passcode.to_owned(),
            auto_lock_minutes: previous
                .as_ref()
                .map_or(1, |record| record.auto_lock_minutes),
            biometric_enabled: previous.is_some_and(|record| record.biometric_enabled),
            failed_attempts: 0,
            retry_until_millis: 0,
        })
    }

    pub fn validate(&self) -> Result<()> {
        validate_passcode(&self.passcode)?;
        if self.failed_attempts > 32 {
            bail!("Invalid screen lock credential record");
        }
        validate_minutes(self.auto_lock_minutes)
    }

    pub fn matches(&self, passcode: &str) -> bool {
        self.passcode.as_bytes() == passcode.as_bytes()
    }

    pub fn fail(&mut self, now_millis: u64) {
        self.failed_attempts = self.failed_attempts.saturating_add(1).min(32);
        if self.failed_attempts >= 5 {
            let seconds = (1_u64 << (self.failed_attempts - 5).min(6)).min(60);
            self.retry_until_millis = now_millis.saturating_add(seconds * 1000);
        }
    }
}

pub(super) fn validate_passcode(passcode: &str) -> Result<()> {
    if passcode.len() != 6 || !passcode.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("The screen passcode must contain six digits");
    }
    Ok(())
}

/// The Flutter security page offers the same durations.
pub(super) fn validate_minutes(minutes: u32) -> Result<()> {
    if !matches!(minutes, 0 | 1 | 5 | 60 | 300) {
        bail!("Unsupported automatic screen lock duration");
    }
    Ok(())
}
