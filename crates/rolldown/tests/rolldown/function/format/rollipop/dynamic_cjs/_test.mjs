import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const context = {};
vm.runInNewContext(await readFile(new URL('./dist/main.js', import.meta.url), 'utf8'), context);
assert.deepEqual(JSON.parse(JSON.stringify(await context.result)), {
  answer: 42,
  named: 42,
  events: ['sync', 'module'],
  babel: 'babel-default',
  node: 'babel-default',
  nodeNamed: 'named',
});
