export function createCoreWorker(module, memory) {
  const worker = new Worker(new URL("../core_worker.js?opfs-disable", globalThis.__mixinRustScript));
  worker.postMessage({ module_or_path: module, memory });
  return worker;
}
