import assert from 'node:assert/strict';
import test from 'node:test';
import { buildArguments } from './tauri.mjs';

test('macOS dev and debug builds use the development identity; release keeps the production identity', () => {
  const identifier = 'one.mixin.messenger.desktop';
  for (const [args, debug] of [
    [['dev'], true],
    [['dev', '--release'], false],
    [['build'], false],
    [['build', '--debug'], true],
    [['build', '-d'], true],
    [['bundle'], false],
    [['bundle', '--debug'], true],
    [['bundle', '-d'], true],
    [['dev', '--', '--release'], true],
    [['build', '--', '--debug'], false],
    [['info'], false],
  ]) {
    const result = buildArguments(args, 'darwin', identifier);
    if (debug) {
      assert.equal(JSON.parse(result[2]).identifier, `${identifier}.debug`);
      assert.deepEqual([result[0], ...result.slice(3)], args);
    } else {
      assert.deepEqual(result, args);
    }
    assert.deepEqual(buildArguments(args, 'linux', identifier), args);
    assert.deepEqual(buildArguments(args, 'win32', identifier), args);
  }
});
