import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// CommonJS importees go through the cached namespace helper.
assert.match(code, /__rollipop_require__\.i\(\d+\)/);
assert.match(code, /__rollipop_require__\.i\(\d+, 1\)/);
assert.doesNotMatch(code, /__rollipop_require__\.t\(__rollipop_require__\(/);

await import('./dist/main.js');
const result = globalThis.__rollipop_cjs_interop_result;

assert.equal(result.sameNamespace, true);
assert.equal(await result.dynamicSameNamespace, true);
assert.deepEqual(result.plainDefault, { named: 'plain-named', other: 'plain-other' });
assert.equal(result.named, 'plain-named');
assert.deepEqual(result.plainKeys, ['default', 'named', 'other']);
assert.equal(result.esmoduleDefault, 'esmodule-default');
assert.equal(result.esmoduleNamed, 'esmodule-named');
// Node interop exposes `module.exports` as default, even with `__esModule`.
assert.equal(result.nodeDefaultIsExports, 'object');
assert.equal(result.nodeDistinctFromBabel, true);
assert.equal(result.reassigned, 'reassigned');
assert.equal(result.extra, 'extra');
// A namespace created while the CommonJS module is still loading is not cached.
assert.deepEqual(result.cycleKeysDuringLoad, ['default', 'early']);
assert.deepEqual(result.cycleKeys, ['default', 'early', 'late']);
