import assert from 'node:assert/strict';

await import('./dist/main.js');
const result = await globalThis.__rollipop_escaped_barrel_result;

assert.equal(result.reexportValue, 'reexport');
assert.equal(result.later, 'function');
assert.deepEqual([...result.atLoad].sort(), ['esc', 'req', 'starns']);
assert.ok(
  result.afterDynamicImport.includes('dyn'),
  `dyn should have run: ${result.afterDynamicImport}`,
);
