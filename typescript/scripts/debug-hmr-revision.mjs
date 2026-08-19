import { hmrStatus } from './ui-debug-client.mjs';

try {
  const { response, result } = await hmrStatus();
  console.log(JSON.stringify(result, null, 2));
  if (!response.ok || result.ok === false) process.exitCode = 1;
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
