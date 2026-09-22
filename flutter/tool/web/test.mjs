// Exercises the actual generated Dart bridge; no UI interactions or accounts.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, extname } from 'node:path';
import { chromium } from 'playwright';
const flutter = fileURLToPath(new URL('../../', import.meta.url));
const mime = {'.js':'text/javascript','.wasm':'application/wasm','.html':'text/html','.json':'application/json'};
const server = createServer(async (req, res) => {
  res.setHeader('Cross-Origin-Opener-Policy','same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy','require-corp');
  try {
    const path = new URL(req.url,'http://localhost').pathname;
    if (path === '/favicon.ico') { res.statusCode=204;res.end();return; }
    if (path === '/') {
      res.setHeader('Content-Type','text/html');
      res.end('<!doctype html><script src="web_contract.js" defer></script>');
      return;
    }
    const base = path.startsWith('/pkg/') || path === '/core_worker.js' ? `${flutter}web` : `${flutter}build/web`;
    const file = resolve(base, `.${path}`);
    if (!file.startsWith(`${base}/`)) throw new Error('Invalid path');
    res.setHeader('Content-Type',mime[extname(file)] || 'application/octet-stream');
    res.end(await readFile(file));
  } catch (error) { res.statusCode=404;res.end(String(error)); }
});
await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
let browser;
try {
  browser = await chromium.launch({channel:'chrome',headless:true});
  const page = await browser.newPage();
  page.on('console', msg => { if(msg.type() === 'error') console.error(msg.text()); });
  page.on('pageerror',error => console.error(error));
  for (const restore of [false,true]) {
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(() => globalThis.mixinContractResult, null, {timeout:60000});
    const result = await page.evaluate(() => globalThis.mixinContractResult);
    if (!result.ok || result.restored !== restore) throw new Error(JSON.stringify(result));
    console.log(restore ? 'PASS: Flutter bridge restores OPFS settings after reload' : 'PASS: Flutter bridge concurrent startup, SQLite settings, event stream and unsupported operations');
    // Tear down the page and its Worker before reacquiring OPFS handles.
    await page.goto('about:blank');
  }
} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
