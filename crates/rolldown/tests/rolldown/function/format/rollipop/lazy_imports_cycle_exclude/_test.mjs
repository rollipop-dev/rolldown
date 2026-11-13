import assert from 'node:assert/strict';

await import('./dist/main.js');
// Same result as eager evaluation.
assert.deepEqual(globalThis.__rollipop_cycle_exclude_result, { c: 'c:f', pthing: 'p:c:f' });
