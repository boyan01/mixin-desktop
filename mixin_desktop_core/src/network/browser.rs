use std::path::Path;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/web/platform.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name=uploadFile)]
    async fn upload(
        path: &str,
        url: &str,
        signal: &web_sys::AbortSignal,
    ) -> Result<JsValue, JsValue>;
}
struct AbortOnDrop(web_sys::AbortController);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

// Fetch accepts an OPFS File/Blob directly, avoiding a full copy in WASM memory.
pub(crate) async fn upload_file(path: &Path, url: &str) -> anyhow::Result<()> {
    let controller = AbortOnDrop(
        web_sys::AbortController::new()
            .map_err(|e| anyhow::anyhow!("create upload cancellation: {e:?}"))?,
    );
    upload(&path.to_string_lossy(), url, &controller.0.signal())
        .await
        .map_err(|e| anyhow::anyhow!("attachment upload failed: {e:?}"))?;
    Ok(())
}
