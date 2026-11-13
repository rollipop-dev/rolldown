import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = globalThis.__rollipop_merged_side_effect_result;

assert.equal(result.polyfill, 'function');
assert.equal(result.registered, true);
assert.equal(result.barrelValue, 'barrel');
// barrel.js ran when `barrelValue` was read, and its side-effect import ran with it.
assert.equal(globalThis.__rollipop_merged_setup, true);
