import assert from 'node:assert';

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_commonjs_result, 'cjs:42');
