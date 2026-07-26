use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{anyhow, Context};
#[cfg(target_os = "linux")]
use directories::BaseDirs;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use directories::ProjectDirs;
#[cfg(target_os = "windows")]
use directories::UserDirs;

#[cfg(target_os = "macos")]
use objc2::rc::autoreleasepool;
#[cfg(target_os = "macos")]
use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};

const DATA_DIRECTORY_ENV: &str = "MIXIN_DESKTOP_DATA_DIR";

static DATA_DIRECTORY: OnceLock<Result<PathBuf, String>> = OnceLock::new();

pub(crate) fn app_database_path(file_name: &str) -> anyhow::Result<PathBuf> {
    Ok(data_directory()?.join(file_name))
}

pub(crate) fn account_database_path(
    identity_number: &str,
    file_name: &str,
) -> anyhow::Result<PathBuf> {
    if identity_number.is_empty()
        || !identity_number
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(anyhow!("invalid identity number"));
    }

    Ok(data_directory()?.join(identity_number).join(file_name))
}

pub fn account_data_directory(identity_number: &str) -> anyhow::Result<PathBuf> {
    if identity_number.is_empty()
        || !identity_number
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(anyhow!("invalid identity number"));
    }
    Ok(data_directory()?.join(identity_number))
}

pub fn log_directory() -> anyhow::Result<PathBuf> {
    Ok(data_directory()?.join("log"))
}

pub(crate) async fn create_parent_directory(path: &Path) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("database path has no parent: {}", path.display()))?;
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("create database directory {}", parent.display()))
}

pub fn data_directory() -> anyhow::Result<PathBuf> {
    DATA_DIRECTORY
        .get_or_init(resolve_data_directory)
        .clone()
        .map_err(|error| anyhow!(error))
}

fn resolve_data_directory() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os(DATA_DIRECTORY_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    platform_data_directory()
        .ok_or_else(|| "failed to resolve application data directory".to_string())
}

#[cfg(target_os = "macos")]
fn platform_data_directory() -> Option<PathBuf> {
    autoreleasepool(|_| {
        // This is the same native API used by path_provider_foundation. In a
        // sandboxed app it resolves inside the app container automatically.
        let file_manager = unsafe { NSFileManager::defaultManager() };
        let urls = unsafe {
            file_manager.URLsForDirectory_inDomains(
                NSSearchPathDirectory::NSDocumentDirectory,
                NSSearchPathDomainMask::NSUserDomainMask,
            )
        };
        let path = unsafe { urls.first()?.path()? };
        Some(PathBuf::from(path.to_string()))
    })
}

#[cfg(target_os = "windows")]
fn platform_data_directory() -> Option<PathBuf> {
    Some(UserDirs::new()?.document_dir()?.join("Mixin"))
}

#[cfg(target_os = "linux")]
fn platform_data_directory() -> Option<PathBuf> {
    Some(BaseDirs::new()?.home_dir().join(".mixin"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_data_directory() -> Option<PathBuf> {
    ProjectDirs::from("dev", "Mixin", "Mixin Desktop")
        .map(|directories| directories.data_local_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_path_rejects_unsafe_identity_numbers() {
        assert!(account_database_path("../other-account", "mixin.db").is_err());
        assert!(account_database_path("", "mixin.db").is_err());
    }

    #[test]
    fn account_paths_are_isolated() {
        let first = account_database_path("7000", "mixin.db").unwrap();
        let second = account_database_path("7001", "mixin.db").unwrap();

        assert_ne!(first, second);
        assert_eq!(first.file_name().unwrap(), "mixin.db");
        assert_eq!(first.parent().unwrap().file_name().unwrap(), "7000");
        assert_eq!(second.parent().unwrap().file_name().unwrap(), "7001");
    }

    #[test]
    fn default_directory_matches_flutter_app() {
        let directory = platform_data_directory().unwrap();

        #[cfg(target_os = "macos")]
        assert!(directory.ends_with("Documents"));
        #[cfg(target_os = "windows")]
        assert_eq!(
            directory,
            UserDirs::new()
                .unwrap()
                .document_dir()
                .unwrap()
                .join("Mixin")
        );
        #[cfg(target_os = "linux")]
        assert_eq!(
            directory,
            BaseDirs::new().unwrap().home_dir().join(".mixin")
        );
    }
}
