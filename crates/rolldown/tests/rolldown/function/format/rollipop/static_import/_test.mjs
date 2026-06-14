import assert from 'node:assert';

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_static_import_result, 'dep:7');
