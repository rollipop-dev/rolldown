import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = await globalThis.__rollipop_frozen_namespace_result;

assert.equal(result.client, 'ready');
// The frozen namespace keeps its keys; later importers get a namespace with the new key.
assert.equal(result.stillFrozen, true);
assert.deepEqual(result.frozenKeys, ['default', 'init']);
assert.deepEqual(result.readerKeys, ['client', 'default', 'init']);
