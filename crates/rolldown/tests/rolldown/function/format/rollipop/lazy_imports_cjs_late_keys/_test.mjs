import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = await globalThis.__rollipop_late_keys_result;

// Keys added synchronously after load are visible to later importers.
assert.equal(result.client, 'ready');
assert.equal(result.added, 'added');
assert.equal(result.original, 'original');
assert.equal(result.nodeClient, 'ready');
assert.equal(result.sameNamespace, true);
assert.deepEqual(result.clientKeys, ['client', 'default', 'init']);
assert.deepEqual(result.earlyNodeKeys, ['client', 'default', 'init']);
