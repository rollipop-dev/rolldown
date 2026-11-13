import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = globalThis.__rollipop_side_effect_barrel_result;

assert.equal(result.later, 'function');
// Both re-exported modules ran with the side-effect import, also through `sub.js`.
assert.deepEqual([...result.atLoad].sort(), ['nested', 'register']);
