import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
assert.match(code, /__rollipop_lazy__\(\d+\)/);
assert.match(code, /__rollipop_lazy__\(\d+, 0\)/);
// Reads of lazy bindings go through the holder, including re-export getters.
assert.match(code, /\(0, import_log_\w+\.a\.log\)\("main"\)/);
assert.match(code, /reexportedLazy: \(\) => import_lazy_\w+\.a\.lazyValue/);
// Eager bindings are read directly.
assert.match(
  code,
  /var import_star_\w+ = __rollipop_require__\(\d+\);\n\s*__rollipop_require__\.re\(__rollipop_exports__, import_star_\w+\);/,
);

await import('./dist/main.js');
const result = globalThis.__rollipop_lazy_imports_result;

// Side-effect and namespace imports stay eager; an import kept only for side
// effects (its binding is never read) stays eager too.
assert.deepEqual(result.beforeAccess, ['side', 'effect', 'namespace', 'main']);
// `export *` is eager inside reexport.js, which main imports lazily.
assert.match(code, /var import_star_\w+ = __rollipop_require__\(\d+\);/);
assert.equal(result.lazyValue, 'lazy-value');
assert.equal(result.lazyDefault, 'lazy-default');
assert.deepEqual(result.afterLazy, ['side', 'effect', 'namespace', 'main', 'lazy']);
assert.equal(result.namespace, 'namespace-value');
assert.equal(result.fromStar, 'star-value');
assert.equal(result.reexportedLazy, 'lazy-value');
assert.equal(result.cjsDefault, 'cjs-named');
assert.equal(result.cjsNamed, 'cjs-named');
assert.equal(result.cycle, 'b');
// Every read of a failed module rethrows its initialization error.
assert.deepEqual(result.errors, ['boom', 'boom']);
assert.deepEqual(result.final, [
  'side',
  'effect',
  'namespace',
  'main',
  'lazy',
  'star',
  'cjs',
  'cycle_a',
  'cycle_b',
  'throws',
]);
// Never-read bindings never evaluate their module.
assert.ok(!result.final.includes('never'));
