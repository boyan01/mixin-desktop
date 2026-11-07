use std::num::NonZeroU32;

use anyhow::{bail, Result};
use ring::{
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};

// OWASP Password Storage Cheat Sheet's PBKDF2-HMAC-SHA256 work factor.
const ITERATIONS: u32 = 600_000;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Record {
    version: u32,
    iterations: u32,
    salt: [u8; 32],
    verifier: [u8; 32],
    pub auto_lock_minutes: u32,
    pub biometric_enabled: bool,
    pub failed_attempts: u32,
    pub retry_until_millis: u64,
}

impl Record {
    pub fn create(passcode: &str, previous: Option<Self>) -> Result<Self> {
        validate_passcode(passcode)?;
        let mut salt = [0_u8; 32];
        SystemRandom::new()
            .fill(&mut salt)
            .map_err(|_| anyhow::anyhow!("Could not generate screen lock salt"))?;
        let mut verifier = [0_u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(ITERATIONS).expect("nonzero work factor"),
            &salt,
            passcode.as_bytes(),
            &mut verifier,
        );
        Ok(Self {
            version: 1,
            iterations: ITERATIONS,
            salt,
            verifier,
            auto_lock_minutes: previous
                .as_ref()
                .map_or(1, |record| record.auto_lock_minutes),
            biometric_enabled: previous.is_some_and(|record| record.biometric_enabled),
            failed_attempts: 0,
            retry_until_millis: 0,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || self.iterations != ITERATIONS || self.failed_attempts > 32 {
            bail!("Invalid screen lock credential record");
        }
        validate_minutes(self.auto_lock_minutes)
    }

    pub fn matches(&self, passcode: &str) -> bool {
        pbkdf2::verify(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(self.iterations).expect("validated work factor"),
            &self.salt,
            passcode.as_bytes(),
            &self.verifier,
        )
        .is_ok()
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

pub(super) fn validate_minutes(minutes: u32) -> Result<()> {
    if !matches!(minutes, 0 | 1 | 5 | 60 | 300) {
        bail!("Unsupported automatic screen lock duration");
    }
    Ok(())
}
