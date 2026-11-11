use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::anyhow;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use directories::BaseDirs;
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
use directories::ProjectDirs;
#[cfg(target_os = "windows")]
use directories::UserDirs;

#[cfg(target_os = "macos")]
use objc2::rc::autoreleasepool;
#[cfg(target_os = "macos")]
use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};

const DATA_DIRECTORY_ENV: &str = "MIXIN_DESKTOP_DATA_DIR";
/// The production Flutter desktop app is sandboxed under this bundle id.
#[cfg(target_os = "macos")]
const FLUTTER_MACOS_BUNDLE_ID: &str = "one.mixin.messenger.desktop";

static DATA_DIRECTORY: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// Sets the host's private data directory before any database or log opens.
pub fn configure_data_directory(path: PathBuf) -> anyhow::Result<()> {
    configure_directory(&DATA_DIRECTORY, path)
}

fn configure_directory(
    directory: &OnceLock<Result<PathBuf, String>>,
    path: PathBuf,
) -> anyhow::Result<()> {
    if !path.is_absolute() {
        return Err(anyhow!("application data directory must be absolute"));
    }
    match directory.get_or_init(|| Ok(path.clone())) {
        Ok(current) if current == &path => Ok(()),
        _ => Err(anyhow!("application data directory is already initialized")),
    }
}

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

#[cfg(not(target_arch = "wasm32"))]
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

/// wasm: directories inside the virtual filesystem are created by the Worker
/// bridge when it opens a file, so there is nothing to do in Rust.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn create_parent_directory(_path: &Path) -> anyhow::Result<()> {
    Ok(())
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

/// The Flutter app's data directory, so both apps open the same data.
#[cfg(target_os = "macos")]
fn platform_data_directory() -> Option<PathBuf> {
    if env::var_os("APP_SANDBOX_CONTAINER_ID").is_none() {
        return Some(
            BaseDirs::new()?
                .home_dir()
                .join("Library/Containers")
                .join(FLUTTER_MACOS_BUNDLE_ID)
                .join("Data/Documents"),
        );
    }
    // A sandboxed build resolves inside its own container automatically,
    // like path_provider_foundation's getApplicationDocumentsDirectory.
    autoreleasepool(|_| {
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

#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
fn platform_data_directory() -> Option<PathBuf> {
    ProjectDirs::from("dev", "Mixin", "Mixin Desktop")
        .map(|directories| directories.data_local_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_directory_cannot_switch_after_initialization() {
        let directory = OnceLock::new();
        let first = env::temp_dir().join("mixin-first");
        let second = env::temp_dir().join("mixin-second");
        configure_directory(&directory, first.clone()).unwrap();
        configure_directory(&directory, first.clone()).unwrap();
        assert!(configure_directory(&directory, second).is_err());
        assert_eq!(directory.get(), Some(&Ok(first)));
    }

    #[test]
    fn relative_directory_is_rejected_without_initializing_storage() {
        let directory = OnceLock::new();
        assert!(configure_directory(&directory, PathBuf::from("relative")).is_err());
        assert!(directory.get().is_none());
    }

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
        assert_eq!(
            directory,
            BaseDirs::new()
                .unwrap()
                .home_dir()
                .join("Library/Containers/one.mixin.messenger.desktop/Data/Documents")
        );
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

#[cfg(target_arch = "wasm32")]
fn platform_data_directory() -> Option<PathBuf> {
    Some(PathBuf::from("/mixin"))
}
