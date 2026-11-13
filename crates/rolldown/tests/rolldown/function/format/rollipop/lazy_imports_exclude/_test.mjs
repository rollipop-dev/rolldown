import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// The excluded module is imported eagerly; others stay lazy.
assert.match(code, /var import_register_\w+ = __rollipop_require__\(\d+\);/);
assert.match(code, /var import_lazy_\w+ = __rollipop_lazy__\(\d+\);/);

await import('./dist/main.js');
const result = globalThis.__rollipop_lazy_exclude_result;
assert.equal(result.beforeRead, 'registered-at-load');
assert.equal(result.lazyLoadedEarly, false);
assert.equal(result.pluginName, 'plugin');
assert.equal(result.lazyValue, 'lazy-value');
