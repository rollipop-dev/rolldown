import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = globalThis.__rollipop_interop_failure_result;

// A lazy import stays unresolved until interop construction succeeds, so every
// read rethrows instead of later reads returning `undefined`.
assert.deepEqual(result.immediate, ['threw:esmodule-flag', 'threw:esmodule-flag']);
assert.deepEqual(result.afterDescriptor, ['threw:descriptor-second', 'threw:descriptor-second']);
