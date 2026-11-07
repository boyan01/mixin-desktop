//! Database file lifecycle. All connections must be closed before these calls.
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn rename(from: &Path, to: &Path) -> anyhow::Result<()> {
    match crate::fs::rename(from, to).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => Ok(result?),
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn remove(path: &Path) -> anyhow::Result<()> {
    match crate::fs::remove_file(path).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => Ok(result?),
    }
}
#[cfg(target_arch = "wasm32")]
mod browser {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(module = "/web/platform.js")]
    extern "C" {
        #[wasm_bindgen(catch, js_name=databaseFiles)]
        pub async fn operation(action: &str, path: &str, target: &str) -> Result<JsValue, JsValue>;
    }
    pub async fn run(action: &str, path: &std::path::Path, target: &str) -> anyhow::Result<()> {
        operation(action, &path.to_string_lossy(), target)
            .await
            .map_err(|error| anyhow::anyhow!("database file operation: {error:?}"))?;
        Ok(())
    }
}
#[cfg(target_arch = "wasm32")]
pub(crate) async fn rename(from: &Path, to: &Path) -> anyhow::Result<()> {
    browser::run("rename", from, &to.to_string_lossy()).await
}
#[cfg(target_arch = "wasm32")]
pub(crate) async fn remove(path: &Path) -> anyhow::Result<()> {
    browser::run("remove", path, "").await
}
#[cfg(target_arch = "wasm32")]
pub(crate) async fn remove_directory(path: &Path) -> anyhow::Result<()> {
    browser::run("remove-directory", path, "").await
}
