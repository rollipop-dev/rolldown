import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const context = { crypto: webcrypto };
vm.runInNewContext(await readFile(new URL('./dist/main.js', import.meta.url), 'utf8'), context);

const runtime = context.__rolldown_runtime__;
const dep = runtime.loadExports('dep.js');
const reexport = runtime.loadExports('reexport.js');
assert.deepEqual(Object.keys(dep).sort(), ['increment', 'unusedEffect', 'unusedPure', 'value']);
assert.deepEqual(Object.keys(reexport).sort(), ['forwarded', 'unusedForwarded']);
assert.equal(typeof Object.getOwnPropertyDescriptor(dep, 'unusedPure').get, 'function');
assert.equal(typeof Object.getOwnPropertyDescriptor(reexport, 'unusedForwarded').get, 'function');
assert.equal(dep.unusedPure, 42);
assert.equal(reexport.unusedForwarded, 42);
assert.deepEqual(Array.from(context.values), [2, 2]);
assert.equal(dep.value, 2);
assert.equal(reexport.forwarded, 2);
dep.increment();
assert.equal(dep.value, 3);
assert.equal(reexport.forwarded, 3);
assert.deepEqual(Array.from(context.effects), ['dep', 'reexport', 'entry']);
