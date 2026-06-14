import assert from 'node:assert';

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_basic_result, 'entry:1');
