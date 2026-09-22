#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn run_contract(restore: bool) -> Result<(), JsValue> {
    mixin_desktop_core::browser_tests::run(restore)
        .await
        .map_err(|error| JsValue::from_str(&format!("{error:#}")))
}
