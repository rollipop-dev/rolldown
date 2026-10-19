import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
assert.match(code, /\bdirect\s*:\s*\(\)\s*=>/);
assert.match(code, /\bnested\s*:\s*\(\)\s*=>/);
assert.match(code, /\bgroup\s*:\s*\(\)\s*=>/);
assert.match(code, /\bcjsValue\s*:\s*\(\)\s*=>/);
assert.match(code, /\bcjsDefault\s*:\s*\(\)\s*=>/);
assert.match(code, /\bunknownValue\s*:\s*\(\)\s*=>/);

const context = { seed: 7 };
vm.runInNewContext(code, context);
assert.deepEqual(Array.from(context.result), [42, 42, 7, 43, 44, 44, true, true, true]);
