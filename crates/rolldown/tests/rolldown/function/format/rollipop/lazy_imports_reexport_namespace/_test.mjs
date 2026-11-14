import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = await globalThis.__rollipop_reexport_namespace_result;
const { final } = result;

assert.deepEqual(result.values, [
  'ns',
  'star-ns',
  'star',
  'require',
  'ns-late',
  'star-late',
  'dynamic',
]);
// Re-exports that are never read through a namespace, `export *`, `require()` or
// `import()` keep their module eager, as without lazy imports.
for (const name of [
  'ns_register',
  'star_ns_register',
  'star_register',
  'require_register',
  'dynamic_register',
]) {
  assert.ok(final.includes(name), `${name} should have run: ${final}`);
}
// A namespace member read and a read through `export *` stay lazy.
for (const name of ['ns_late', 'star_late']) {
  assert.ok(
    final.indexOf(name) > final.indexOf('before late'),
    `${name} should run on first read: ${final}`,
  );
}
