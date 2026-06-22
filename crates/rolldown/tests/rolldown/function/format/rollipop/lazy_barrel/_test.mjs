import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');

assert.match(code, /barrel\/a\.js/);
assert.doesNotMatch(code, /barrel\/index\.js|lazy-b-remove-me|__rollipop_lazy_barrel_b_loaded/);

await import('./dist/main.js');

assert.equal(globalThis.__rollipop_lazy_barrel_result, 'lazy-a');
assert.equal(globalThis.__rollipop_lazy_barrel_namespace_result, 'lazy-a');
assert.equal(globalThis.__rollipop_lazy_barrel_b_loaded, undefined);
