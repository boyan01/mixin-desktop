use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
fn size_sync(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(std::result::Result::ok)
        .map(|entry| size_sync(&entry.path()))
        .sum()
}

fn clear_sync(path: &PathBuf) -> Result<()> {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(anyhow!("invalid storage directory: {}", path.display()));
    }
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let metadata = std::fs::symlink_metadata(&child)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            std::fs::remove_dir_all(child)?;
        } else {
            std::fs::remove_file(child)?;
        }
    }
    Ok(())
}

pub async fn directory_size(path: &Path) -> u64 {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || size_sync(&path))
        .await
        .unwrap_or_default()
}
pub async fn clear_directory_contents(path: &Path) -> Result<()> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || clear_sync(&path)).await?
}
