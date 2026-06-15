import assert from 'node:assert';

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_side_effect_import_result, 'entry:side-effect');
