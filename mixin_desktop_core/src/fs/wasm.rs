//! OPFS handles: async acquisition, bounded binary reads/writes, explicit flush.
use super::Metadata;
use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/web/platform.js")]
extern "C" {
    #[wasm_bindgen(js_name=fileReserve)]
    fn reserve() -> u32;
    #[wasm_bindgen(catch,js_name=fileOpen)]
    async fn open(handle: u32, path: &str, mode: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=fileMetadata)]
    async fn stat(path: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=createDirectory)]
    async fn mkdir(path: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=removePath)]
    async fn remove(path: &str, recursive: bool) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=moveFile)]
    async fn move_file(from: &str, to: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=fileRead)]
    fn read_chunk(handle: u32, offset: u64, length: u32) -> Result<Vec<u8>, JsValue>;
    #[wasm_bindgen(catch,js_name=fileWrite)]
    fn write_chunk(handle: u32, offset: u64, data: &[u8]) -> Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_name=fileFlush)]
    fn flush(handle: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(js_name=fileClose)]
    fn close(handle: u32);
    #[wasm_bindgen(catch,js_name=listDirectory)]
    async fn list(path: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=fileSize)]
    fn size(handle: u32) -> Result<f64, JsValue>;
}
fn error(value: JsValue) -> io::Error {
    let name = js_sys::Reflect::get(&value, &"name".into())
        .ok()
        .and_then(|v| v.as_string());
    let kind = match name.as_deref() {
        Some("NotFoundError") => io::ErrorKind::NotFound,
        Some("NotAllowedError") => io::ErrorKind::PermissionDenied,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, format!("OPFS: {value:?}"))
}
#[derive(Debug)]
pub struct File {
    handle: Option<u32>,
    offset: u64,
}
impl File {
    async fn with_mode(path: &Path, mode: &str) -> io::Result<Self> {
        let handle = reserve();
        // Own the reservation before the first await, so cancellation closes
        // an acquired handle or prevents a pending acquisition from leaking.
        let file = Self {
            handle: Some(handle),
            offset: 0,
        };
        open(handle, &path.to_string_lossy(), mode)
            .await
            .map_err(error)?;
        Ok(file)
    }

    fn handle(&self) -> io::Result<u32> {
        self.handle.ok_or_else(|| io::Error::other("file closed"))
    }
    pub async fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::with_mode(path.as_ref(), "read").await
    }
    pub async fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata {
            len: size(self.handle()?).map_err(error)? as u64,
            is_file: true,
        })
    }
    pub async fn sync_all(&self) -> io::Result<()> {
        flush(self.handle()?).map_err(error)
    }
}
impl Drop for File {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            close(handle);
        }
    }
}
impl tokio::io::AsyncRead for File {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let result = (|| {
            let bytes = read_chunk(
                this.handle()?,
                this.offset,
                buf.remaining().min(64 * 1024) as u32,
            )
            .map_err(error)?;
            buf.put_slice(&bytes);
            this.offset += bytes.len() as u64;
            Ok(())
        })();
        Poll::Ready(result)
    }
}
impl tokio::io::AsyncWrite for File {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        Poll::Ready((|| {
            let written = write_chunk(
                this.handle()?,
                this.offset,
                &buf[..buf.len().min(64 * 1024)],
            )
            .map_err(error)?;
            this.offset += u64::from(written);
            Ok(written as usize)
        })())
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.handle().and_then(|h| flush(h).map_err(error)))
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
#[derive(Default)]
pub struct OpenOptions {
    create_new: bool,
}
impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn create_new(&mut self, value: bool) -> &mut Self {
        self.create_new = value;
        self
    }
    pub fn write(&mut self, _: bool) -> &mut Self {
        self
    }
    pub async fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        File::with_mode(
            path.as_ref(),
            if self.create_new {
                "write-new"
            } else {
                "write"
            },
        )
        .await
    }
}
pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut file = File::open(path).await?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await?;
    Ok(bytes)
}
pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut file = File::with_mode(path.as_ref(), "write").await?;
    file.write_all(contents.as_ref()).await?;
    file.sync_all().await
}
pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let mut input = File::open(from).await?;
    let mut output = File::with_mode(to.as_ref(), "write").await?;
    let count = tokio::io::copy(&mut input, &mut output).await?;
    output.sync_all().await?;
    Ok(count)
}
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    move_file(
        &from.as_ref().to_string_lossy(),
        &to.as_ref().to_string_lossy(),
    )
    .await
    .map_err(error)?;
    Ok(())
}
pub async fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    remove(&path.as_ref().to_string_lossy(), false)
        .await
        .map_err(error)?;
    Ok(())
}
pub async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    remove(&path.as_ref().to_string_lossy(), true)
        .await
        .map_err(error)?;
    Ok(())
}
pub async fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    mkdir(&path.as_ref().to_string_lossy())
        .await
        .map_err(error)?;
    Ok(())
}
pub async fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let value = stat(&path.as_ref().to_string_lossy())
        .await
        .map_err(error)?;
    serde_json::from_str(
        &value
            .as_string()
            .ok_or_else(|| io::Error::other("invalid file metadata"))?,
    )
    .map_err(io::Error::other)
}
pub async fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let mut result = PathBuf::from("/");
    for part in path.as_ref().components() {
        match part {
            std::path::Component::Normal(p) => result.push(p),
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return Err(io::Error::other("path escapes storage"));
                }
            }
            _ => {}
        }
    }
    metadata(&result).await?;
    Ok(result)
}

#[derive(serde::Deserialize)]
struct Entry {
    path: String,
    is_file: bool,
}
async fn entries(path: &Path) -> io::Result<Vec<Entry>> {
    let value = list(&path.to_string_lossy()).await.map_err(error)?;
    serde_json::from_str(
        &value
            .as_string()
            .ok_or_else(|| io::Error::other("invalid directory listing"))?,
    )
    .map_err(io::Error::other)
}
pub async fn directory_size(path: &Path) -> u64 {
    let mut pending = vec![path.to_owned()];
    let mut total = 0u64;
    while let Some(path) = pending.pop() {
        if let Ok(items) = entries(&path).await {
            for entry in items {
                if entry.is_file {
                    if let Ok(stat) = metadata(&entry.path).await {
                        total = total.saturating_add(stat.len());
                    }
                } else {
                    pending.push(PathBuf::from(entry.path));
                }
            }
        }
    }
    total
}
pub async fn clear_directory_contents(path: &Path) -> anyhow::Result<()> {
    let items = match entries(path).await {
        Ok(items) => items,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for entry in items {
        if entry.is_file {
            remove_file(&entry.path).await?;
        } else {
            remove_dir_all(&entry.path).await?;
        }
    }
    Ok(())
}
