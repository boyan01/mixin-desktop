import { hmrStatus } from './ui-debug-client.mjs';

const revision = Number.parseInt(process.argv[2] ?? '', 10);
if (!Number.isSafeInteger(revision) || revision < 0) {
  console.error('Usage: npm run debug:wait-hmr -- <revision>');
  process.exit(2);
}

try {
  const { response, result } = await hmrStatus(revision);
  console.log(JSON.stringify(result, null, 2));
  if (!response.ok || result.ok === false) process.exitCode = 1;
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
