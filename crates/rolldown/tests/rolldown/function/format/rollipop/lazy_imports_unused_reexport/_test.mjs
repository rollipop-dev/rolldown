import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = globalThis.__rollipop_unused_reexport_result;

assert.deepEqual(result.values, ['a', 'b', 'c', 'd', 'm', 'late']);
// A re-export whose binding was tree-shaken is kept only for its side effects,
// so its module runs when the re-exporting module runs.
for (const name of ['register_a', 'register_b', 'register_c', 'register_d']) {
  assert.ok(result.final.includes(name), `${name} should have run: ${result.final}`);
}
// A re-export that is read is still lazy.
assert.ok(
  result.final.indexOf('late') > result.final.indexOf('before late'),
  `late.js should run on first read: ${result.final}`,
);
