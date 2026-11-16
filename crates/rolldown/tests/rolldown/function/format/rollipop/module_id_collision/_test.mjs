import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
// The colliding module that sorts first keeps the hash; the other takes the next slot.
assert.match(code, /15276222: function/);
assert.match(code, /15276223: function/);

await import('./dist/main.js');
assert.deepEqual(await globalThis.__rollipop_module_id_collision, {
  a: 'a',
  b: 'b',
  required: 'b',
  dynamic: 'a',
});
