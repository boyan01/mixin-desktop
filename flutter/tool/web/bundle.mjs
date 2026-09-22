import { build } from "esbuild";
import { copyFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const flutter = fileURLToPath(new URL("../../", import.meta.url));
await build({
  entryPoints: [`${flutter}build/rust-web/mixin_desktop_flutter.js`],
  outfile: `${flutter}web/pkg/mixin_desktop_flutter.js`,
  bundle: true,
  format: "iife",
  globalName: "MixinRust",
  nodePaths: [fileURLToPath(new URL("node_modules", import.meta.url))],
  define: { "import.meta.url": "globalThis.__mixinRustScript" },
  banner: { js: "globalThis.__mixinRustScript ||= document.currentScript.src;" },
  footer: { js: "globalThis.wasm_bindgen = Object.assign(MixinRust.default, MixinRust);" },
});
await copyFile(`${flutter}build/rust-web/mixin_desktop_flutter_bg.wasm`, `${flutter}web/pkg/mixin_desktop_flutter_bg.wasm`);
await copyFile(fileURLToPath(new URL("node_modules/@sqlite.org/sqlite-wasm/dist/sqlite3.wasm", import.meta.url)), `${flutter}web/pkg/sqlite3.wasm`);
