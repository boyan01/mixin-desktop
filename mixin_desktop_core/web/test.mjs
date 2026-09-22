// Non-UI runtime test: no app windows, accounts, credentials or remote mutations.
import { createServer } from "vite";
import { chromium } from "playwright";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL(".", import.meta.url));
const server = await createServer({
  plugins: [
    {
      name: "upload-contract",
      configureServer(server) {
        server.middlewares.use("/upload", async (request, response) => {
          let size = 0;
          let valid = request.method === "PUT";
          for await (const chunk of request) {
            size += chunk.length;
            valid &&= chunk.every((value) => value === 42);
          }
          response.statusCode = valid && size === 128 * 1024 + 7 ? 204 : 400;
          response.end();
        });
      },
    },
  ],
  root,
  configFile: false,
  optimizeDeps: { exclude: ["@sqlite.org/sqlite-wasm"] },
  server: { host: "127.0.0.1", port: 0 },
});
let browser;
try {
  await server.listen();
  browser = await chromium.launch({ channel: "chrome", headless: true });
  const page = await browser.newPage();
  page.on("console", (message) => {
    if (message.type() === "error") console.error(message.text());
  });
  page.on("pageerror", (error) => console.error(error));
  await page.goto(server.resolvedUrls.local[0]);
  for (const restore of [false, true]) {
    const result = await page.evaluate(async (restore) => {
      const worker = new Worker("/worker.js", { type: "module" });
      try {
        return await new Promise((resolve, reject) => {
          const deadline = setTimeout(
            () => reject(new Error("Worker contract timed out")),
            60000,
          );
          worker.onmessage = ({ data }) => {
            clearTimeout(deadline);
            resolve(data);
          };
          worker.onerror = (error) => {
            clearTimeout(deadline);
            reject(new Error(error.message));
          };
          worker.postMessage({ restore });
        });
      } finally {
        worker.terminate();
      }
    }, restore);
    if (!result.ok) throw new Error(JSON.stringify(result));
    console.log(
      restore
        ? "PASS: Worker restart restores messages and files"
        : "PASS: SQLite transactions, NULL/i64/blob, Signal, attachments, task cancellation and timers",
    );
  }
} finally {
  await browser?.close();
  await server.close();
}
