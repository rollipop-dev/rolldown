import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
if (!globalThis.__configName?.startsWith('no-tree-shaking')) {
  assert.doesNotMatch(code, /#region barrel\.js/);
}
const context = {};
vm.runInNewContext(code, context);
assert.deepEqual(JSON.parse(JSON.stringify(context.result)), [['increment', 'value'], 2, true, 2]);
