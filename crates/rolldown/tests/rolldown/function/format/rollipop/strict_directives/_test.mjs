import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const context = {};
vm.runInNewContext(await readFile(new URL('./dist/main.js', import.meta.url), 'utf8'), context);
const config = globalThis.__configName ?? '';
const expected = config.startsWith('always')
  ? [true, true, true]
  : config.startsWith('never')
    ? [false, false, false]
    : [true, false, true];
assert.deepEqual(Array.from(context.result), expected);
