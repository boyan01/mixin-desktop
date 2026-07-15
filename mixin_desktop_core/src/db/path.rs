use std::env;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use directories::ProjectDirs;

const DATA_DIRECTORY_ENV: &str = "MIXIN_DESKTOP_DATA_DIR";

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

fn data_directory() -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os(DATA_DIRECTORY_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    ProjectDirs::from("dev", "Mixin", "Mixin Desktop")
        .map(|directories| directories.data_local_dir().to_path_buf())
        .ok_or_else(|| anyhow!("failed to resolve application data directory"))
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
}
