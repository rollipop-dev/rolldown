import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// The lazy loader is passed as the fifth factory parameter.
// Lazy imports use getter holders, not Proxy facades.
assert.match(code, /get a\(\)/);
assert.doesNotMatch(code, /__rollipop_lazy_(resolve|namespaces)/);
assert.match(code, /:function\(\w+,\w+,\w+,\w+,(\w+)\)\{[^}]*\b\1\(\d+,0\)/);

await import('./dist/main.js');
const result = globalThis.__rollipop_lazy_no_proxy_result;
assert.deepEqual(result.log, ['main', 'dep', 'cjs']);
assert.equal(result.value, 'dep-value');
assert.equal(result.cjs, 'cjs-named');
assert.equal(result.named, 'cjs-named');
