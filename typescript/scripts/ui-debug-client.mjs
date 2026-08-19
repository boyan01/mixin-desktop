import { readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const metadataPath = join(tmpdir(), 'mixin-typescript-ui-debug.json');

async function readMetadata() {
  let metadata;
  try {
    metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  } catch {
    throw new Error(`UI debug metadata is unavailable at ${metadataPath}. Start the app with \`npm run tauri dev\`.`);
  }

  try {
    process.kill(metadata.pid, 0);
  } catch {
    throw new Error(`The UI debug metadata belongs to an inactive process (${metadata.pid}). Restart \`npm run tauri dev\`.`);
  }
  return metadata;
}

export async function evaluate(expression) {
  const metadata = await readMetadata();
  let response;
  try {
    response = await fetch(`http://127.0.0.1:${metadata.port}/eval`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${metadata.token}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify({ expression }),
    });
  } catch (error) {
    throw new Error(`Unable to reach the UI debug server: ${error.message}`);
  }

  const result = await response.json();
  return { response, result };
}

export async function hmrStatus(after) {
  const metadata = await readMetadata();
  const query = after === undefined ? '' : `?after=${after}`;
  let response;
  try {
    response = await fetch(`http://127.0.0.1:${metadata.port}/hmr${query}`, {
      headers: { authorization: `Bearer ${metadata.token}` },
    });
  } catch (error) {
    throw new Error(`Unable to reach the UI debug server: ${error.message}`);
  }
  return { response, result: await response.json() };
}
