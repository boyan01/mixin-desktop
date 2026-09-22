import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, extname } from 'node:path';
const root = fileURLToPath(new URL('../../build/web', import.meta.url));
const mime = {'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json','.png':'image/png','.svg':'image/svg+xml','.otf':'font/otf','.ttf':'font/ttf'};
const server = createServer(async (req,res) => {
  res.setHeader('Cross-Origin-Opener-Policy','same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy','require-corp');
  res.setHeader('Cache-Control','no-store');
  try {
    const path = decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    const file = resolve(root, `.${path === '/' ? '/index.html' : path}`);
    if (!file.startsWith(`${root}/`)) throw new Error('Invalid path');
    res.setHeader('Content-Type',mime[extname(file)] || 'application/octet-stream');
    res.end(await readFile(file));
  } catch { res.statusCode=404;res.end('Not found'); }
});
server.listen(Number(process.env.PORT || 7357),'127.0.0.1',() => {
  console.log(`Mixin Web: http://127.0.0.1:${server.address().port}`);
});
