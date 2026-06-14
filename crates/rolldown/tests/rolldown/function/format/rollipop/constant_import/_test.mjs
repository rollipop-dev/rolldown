import assert from 'node:assert';

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_constant_import_result, 'ios');
