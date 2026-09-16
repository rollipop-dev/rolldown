import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const context = {};
vm.runInNewContext(await readFile(new URL('./dist/main.js', import.meta.url), 'utf8'), context);
assert.deepEqual(JSON.parse(JSON.stringify(context.result)), {
  split: 'split-name',
  splitNested: 42,
  name: 'package-name',
  version: '1.2.3',
  nested: 42,
  quoted: true,
  defaultField: 'field',
  mutable: 2,
  escaped: { value: 2 },
  array: [1, 2, 3],
  primitive: 42,
});
assert.equal(context.result.missing, undefined);
