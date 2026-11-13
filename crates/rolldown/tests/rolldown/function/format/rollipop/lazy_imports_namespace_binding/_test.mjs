import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = globalThis.__rollipop_lazy_namespace_binding_result;

// A binding to a whole namespace is enumerable before any property read.
assert.deepEqual(result.keys, ['other', 'value']);
assert.deepEqual(result.forIn, ['other', 'value']);
assert.deepEqual(result.spread, ['other', 'value']);
assert.equal(result.value, 'x-value');
assert.equal(result.same, true);
