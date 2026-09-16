import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
if (!globalThis.__configName?.startsWith('no-tree-shaking')) {
  assert.doesNotMatch(code, /#region removed\.js/);
}
const context = { seed: 7 };
vm.runInNewContext(code, context);
assert.deepEqual(Array.from(context.result), [42, 2, 7, 'default-value']);
