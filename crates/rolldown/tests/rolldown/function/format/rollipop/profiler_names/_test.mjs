import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');

assert.match(code, /var __rollipop_modules__ = __rollipop_require__\.m = \{/);
assert.match(code, /"main\.js": function\(/);
assert.match(code, /__rollipop_require__\("dep\.js"\)/);

await import('./dist/main.js');

const ids = globalThis.__rollipop_profiler_names_ids;

assert.equal(typeof ids.main, 'string');
assert.equal(typeof ids.dep, 'string');
assert.equal(ids.main, 'main.js');
assert.equal(ids.dep, 'dep.js');
