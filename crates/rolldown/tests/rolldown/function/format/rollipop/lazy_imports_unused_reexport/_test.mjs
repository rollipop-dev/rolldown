import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = globalThis.__rollipop_unused_reexport_result;
const { final } = result;

assert.deepEqual(result.values, [
  'a',
  'b',
  'c',
  'd',
  'm',
  'e',
  'g',
  'n',
  'cm',
  'function',
  'late',
  'deep',
]);
// A re-export that nothing reads through its barrel is kept only for its side
// effects, so its module runs when the barrel runs. This holds even when another
// module imports the same symbol directly (register_e/f/g/h).
for (const name of [
  'register_a',
  'register_b',
  'register_c',
  'register_d',
  'register_e',
  'register_f',
  'register_g',
  'register_h',
]) {
  assert.ok(final.includes(name), `${name} should have run: ${final}`);
}

const ranBefore = (name, marker) => final.indexOf(name) < final.indexOf(marker);
if (globalThis.__configName === 'no-tree-shaking') {
  // Without tree-shaking, whether a re-export is imported is unknown, so every
  // re-exported module stays eager.
  assert.ok(ranBefore('late', 'before late'), `late.js should run with its barrel: ${final}`);
  assert.ok(ranBefore('deep', 'before late'), `deep.js should run with its barrel: ${final}`);
} else {
  // Re-exports that are read stay lazy, also through two barrels.
  assert.ok(!ranBefore('late', 'before late'), `late.js should run on first read: ${final}`);
  assert.ok(!ranBefore('deep', 'before late'), `deep.js should run on first read: ${final}`);
}
