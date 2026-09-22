//! Account file access: native files or asynchronously acquired OPFS handles.

#[cfg(not(target_arch = "wasm32"))]
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

/// File size and kind as used by attachment and media code.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Metadata {
    len: u64,
    is_file: bool,
}

impl Metadata {
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }
}

// ---------------------------------------------------------------------------
// Native: direct tokio::fs delegation.
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub use tokio::fs::{File, OpenOptions};

#[cfg(not(target_arch = "wasm32"))]
pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    tokio::fs::read(path).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    tokio::fs::write(path, contents).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::remove_file(path).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::remove_dir_all(path).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::create_dir_all(path).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::rename(from, to).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    tokio::fs::copy(from, to).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    tokio::fs::canonicalize(path).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let metadata = tokio::fs::metadata(path).await?;
    Ok(Metadata {
        len: metadata.len(),
        is_file: metadata.is_file(),
    })
}

// ---------------------------------------------------------------------------
// wasm32: synchronous Worker bridge over OPFS.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::{
    canonicalize, copy, create_dir_all, metadata, read, remove_dir_all, remove_file, rename, write,
    File, OpenOptions,
};

#[cfg(not(target_arch = "wasm32"))]
mod native_stats;
#[cfg(not(target_arch = "wasm32"))]
pub use native_stats::{clear_directory_contents, directory_size};
#[cfg(target_arch = "wasm32")]
pub use wasm::{clear_directory_contents, directory_size};
