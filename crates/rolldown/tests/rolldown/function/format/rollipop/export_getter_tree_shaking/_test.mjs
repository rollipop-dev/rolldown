import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');

for (const name of [
  'unusedEffect',
  'unusedRead',
  'unusedSpecifier',
  'unusedForwarded',
  'unusedSideEffectOnly',
]) {
  const getter = new RegExp(`\\b${name}\\s*:\\s*\\(\\)\\s*=>`);
  if (globalThis.__configName?.startsWith('no-tree-shaking')) {
    assert.match(code, getter);
  } else {
    assert.doesNotMatch(code, getter);
  }
}
assert.doesNotMatch(code, /\.d\(\s*[^,]+,\s*\{\s*\}\s*\)/);
assert.match(code, /\bused\s*:\s*\(\)\s*=>/);
assert.match(code, /\bforwarded\s*:\s*\(\)\s*=>/);

const context = {};
vm.runInNewContext(code, context);
assert.deepEqual(JSON.parse(JSON.stringify(context.result)), {
  values: [
    [1, 1],
    [2, 2],
    [3, 3],
  ],
  effects: ['effect', 'read', 'local', 'reexport', 'side-effect-only', 'entry'],
});
