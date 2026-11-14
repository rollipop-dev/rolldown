import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// The generated declarations of one module factory, found by its `// <file>` comment.
const factory = (file) => code.split(`// ${file}\n`)[1].split(/\n\t\t\d+: function/)[0];
// Named-only CommonJS bindings read `module.exports` directly.
assert.match(code, /var import_live_\w+ = __rollipop_require__\(\d+\);/);
assert.match(code, /var import_fn_\w+ = __rollipop_require__\(\d+\);/);
// `default` and namespace values keep the cached interop namespace.
assert.match(code, /var import_esmodule_\w+ = __rollipop_require__\.i\(\d+\);/);
assert.match(code, /var import_live_\w+ = __rollipop_require__\.i\(\d+\);/);

// So do namespace imports that are only read through named members.
assert.match(factory('member_ns.js'), /= __rollipop_require__\(\d+\);/);
// `default` members keep the namespace.
assert.match(factory('default_member_ns.js'), /= __rollipop_require__\.i\(\d+\);/);
assert.match(factory('computed_default_ns.js'), /= __rollipop_require__\.i\(\d+\);/);
// `typeof ns.key` and `!ns.key` are named reads.
assert.match(factory('typeof_ns.js'), /= __rollipop_require__\(\d+\);/);

await import('./dist/main.js');
const result = globalThis.__rollipop_cjs_direct_reads_result;

assert.equal(result.countBefore, 0);
assert.equal(result.countAfter, 1);
// Keys added after the module loaded are visible to named imports.
assert.equal(result.addedBefore, undefined);
assert.equal(result.addedAfter, 'added-later');
assert.equal(result.helper, 'helper');
assert.equal(result.named, 'esmodule-named');
assert.equal(result.esmoduleDefault, 'esmodule-default');
assert.equal(result.fnDefault, 'fn');
assert.equal(result.namespaceCount, 1);
assert.equal(result.namespaceDefaultIsExports, true);
assert.equal(result.reexported, 'esmodule-named');
assert.equal(result.nodeNamed, 'esmodule-named');
// While a CommonJS module is still loading, later exports are not there yet,
// but named reads see them once it has loaded.
assert.deepEqual(result.cycle, { early: 'early', lateDuringLoad: undefined });
assert.equal(result.cycleLate, 'late');
assert.equal(result.memberHelper, 'helper');
assert.equal(result.computedHelper, 'helper');
assert.equal(result.defaultMember, 'esmodule-default');
assert.deepEqual(result.typeofHelper, ['string', false]);
assert.equal(result.computedDefault, 'plain');
assert.equal(result.plainDefault, 'plain');
