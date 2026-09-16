import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
const context = {};
vm.runInNewContext(await readFile(new URL('./dist/main.js', import.meta.url), 'utf8'), context);
assert.deepEqual(Array.from(context.result), [true, true, true, true, true, true, true, true, 42]);
