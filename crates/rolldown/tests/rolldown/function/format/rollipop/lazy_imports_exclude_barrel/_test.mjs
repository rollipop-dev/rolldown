import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// `exclude` is decided per import edge. A side-effectful module reached through a
// package index only runs at load when the pattern matches the index as well.
assert.match(code, /var import_lib_\w+ = __rollipop_require__\(\d+\);/);
assert.match(code, /var import_register_\w+ = __rollipop_require__\(\d+\);/);

await import('./dist/main.js');
const result = globalThis.__rollipop_exclude_barrel_result;
assert.equal(result.beforeRead, 'registered-at-load');
assert.equal(result.pluginName, 'plugin');
