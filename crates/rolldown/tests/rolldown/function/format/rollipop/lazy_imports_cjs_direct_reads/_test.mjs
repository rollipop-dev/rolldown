import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// Named-only CommonJS bindings omit the interop mode: the holder resolves to `module.exports`.
assert.match(code, /var import_live_\w+ = __rollipop_lazy__\(\d+\);/);
assert.match(code, /\(0, import_live_\w+\.a\.increment\)\(\)/);
// `default` reads keep the interop namespace mode.
assert.match(code, /var import_esmodule_\w+ = __rollipop_lazy__\(\d+, 0\);/);

await import('./dist/main.js');
const result = globalThis.__rollipop_lazy_direct_result;

// Only the side-effect import ran before any binding was read.
assert.deepEqual(result.beforeAccess, ['hot']);
assert.equal(result.countBefore, 0);
assert.equal(result.countAfter, 1);
assert.equal(result.addedBefore, undefined);
assert.equal(result.addedAfter, 'added-later');
assert.deepEqual(result.afterLive, ['hot', 'live']);
assert.equal(result.hot, 'hot-value');
assert.equal(result.fromGetter, 'getter-value');
assert.equal(result.esmoduleDefault, 'esmodule-default');
// Every read of a failed module rethrows its initialization error.
assert.deepEqual(result.errors, ['boom', 'boom']);
// Accessor exports run with `module.exports` as `this`: one memoized instance,
// and the identity holds.
assert.match(code, /var import_single_\w+ = __rollipop_lazy__\(\d+\);/);
assert.equal(result.version, 1);
assert.equal(result.sameClient, true);
assert.deepEqual(result.isExports, [true, true]);
// Proxy exports are read through the Proxy itself.
assert.deepEqual(result.fromProxy, ['proxy-value', 'proxy-value']);
