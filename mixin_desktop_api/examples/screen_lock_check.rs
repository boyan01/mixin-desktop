//! Explicit isolated-mixtest check of the real OS credential store. Removes its PIN afterwards.
use mixin_desktop_api::DesktopClient;
use std::{error::Error, io::Read as _, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("MIXIN_DESKTOP_DATA_DIR").is_none_or(|path| path.is_empty()) {
        return Err("an isolated MIXIN_DESKTOP_DATA_DIR is required".into());
    }
    let desktop = DesktopClient::open().await?;
    let account = desktop.restore_account().await?;
    if !account.profile().full_name.starts_with("mixtest-") {
        account.shutdown().await;
        return Err("this check requires a mixtest account".into());
    }
    let lock = account.screen_lock();
    let original = lock.state().await?;
    if !original.available || original.enabled {
        account.shutdown().await;
        return Err("this check requires an available, unconfigured isolated screen lock".into());
    }
    let mut random = [0_u8; 6];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let pin: String = random
        .iter()
        .map(|byte| char::from(b'0' + byte % 10))
        .collect();
    let wrong: String = std::iter::once(char::from(b'0' + (random[0] % 10 + 1) % 10))
        .chain(pin.chars().skip(1))
        .collect();
    let result = async {
        lock.set_passcode(pin.clone()).await?;
        if !lock.state().await?.enabled {
            return Err("passcode was not enabled".into());
        }
        lock.lock().await?;
        if !lock.state().await?.locked {
            return Err("screen content did not remain locked".into());
        }
        if lock.unlock(wrong).await.is_ok() {
            return Err("wrong passcode unlocked the screen".into());
        }
        if !lock.state().await?.locked {
            return Err("screen content did not remain locked".into());
        }
        lock.unlock(pin.clone()).await?;
        if lock.state().await?.locked {
            return Err("correct passcode did not unlock".into());
        }
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    // Remove only the credential created by this isolated check, even after an assertion-free error.
    let cleanup = if lock.state().await?.enabled {
        match lock.unlock(pin).await {
            Ok(()) => lock.disable_passcode().await,
            Err(error) => Err(error),
        }
    } else {
        Ok(())
    };
    let disabled = lock.state().await?.enabled;
    tokio::time::timeout(Duration::from_secs(15), account.shutdown()).await?;
    result?;
    cleanup?;
    assert!(!disabled);
    println!("PASS: real credential write, lock, wrong-PIN rejection, correct unlock and credential cleanup");
    Ok(())
}
