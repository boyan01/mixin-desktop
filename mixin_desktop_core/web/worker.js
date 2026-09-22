import init, { run_contract } from "./pkg/browser_contract.js";
self.onmessage = async ({ data }) => {
  try {
    await init();
    await run_contract(data.restore);
    self.postMessage({ ok: true });
  } catch (error) {
    self.postMessage({ ok: false, error: String(error), stack: error?.stack });
  }
};
