// One long-lived Rust instance owns all browser database and file handles.
globalThis.__mixinRustScript = new URL("pkg/mixin_desktop_flutter.js", location.href).href;
importScripts(globalThis.__mixinRustScript);
let initialized;
onmessage = ({ data }) => {
  if (!initialized) {
    initialized = wasm_bindgen(data);
    return;
  }
  initialized.then(() => {
    const [payload, ...transfers] = data;
    wasm_bindgen.receive_transfer_closure(payload, transfers);
  }).catch((error) => {
    console.error("Core Worker call failed", error);
    throw error;
  });
};
