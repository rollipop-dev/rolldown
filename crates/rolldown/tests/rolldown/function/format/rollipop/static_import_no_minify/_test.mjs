import assert from 'node:assert';

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_static_import_no_minify_result, 'dep:7');
