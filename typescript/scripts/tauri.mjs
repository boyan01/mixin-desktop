import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { run } from '@tauri-apps/cli';

export function buildArguments(args, platform, identifier) {
  const [command, ...rest] = args;
  const separator = rest.indexOf('--');
  const options = separator < 0 ? rest : rest.slice(0, separator);
  const debug = (command === 'dev' && !options.includes('--release'))
    || (['build', 'bundle'].includes(command) && (options.includes('--debug') || options.includes('-d')));
  if (platform !== 'darwin' || !debug) return args;

  return [command, '--config', JSON.stringify({ identifier: `${identifier}.debug` }), ...rest];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const config = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url)));
  await run(buildArguments(process.argv.slice(2), process.platform, config.identifier));
}
