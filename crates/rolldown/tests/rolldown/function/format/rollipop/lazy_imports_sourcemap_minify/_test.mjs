import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { SourceMap } from 'node:module';

const dist = new URL('./dist/', import.meta.url);
const code = await readFile(new URL('main.js', dist), 'utf8');
const map = new SourceMap(JSON.parse(await readFile(new URL('main.js.map', dist), 'utf8')));
const generatedLines = code.split('\n');
// The lazy loader is in the bundle, so the mapped modules went through it.
assert.match(code, /new Proxy\(/);

function originalPosition(marker) {
  const line = generatedLines.findIndex((text) => text.includes(marker));
  assert.notEqual(line, -1, `${marker} is missing from the bundle`);
  // Point at the opening quote of the literal.
  const column = generatedLines[line].indexOf(marker) - 1;
  return map.findEntry(line, column);
}

async function assertMapsTo(marker, file, expectedLine) {
  const entry = originalPosition(marker);
  assert.ok(entry.originalSource?.endsWith(file), `${marker}: ${JSON.stringify(entry)}`);
  assert.equal(entry.originalLine, expectedLine, `${marker}: ${JSON.stringify(entry)}`);
  const source = (await readFile(new URL(file, import.meta.url), 'utf8')).split('\n');
  assert.equal(source[entry.originalLine].indexOf(marker) - 1, entry.originalColumn, marker);
}

await assertMapsTo('lazy-marker', 'lazy.js', 0);
await assertMapsTo('lazy-throw-marker', 'lazy.js', 3);
await assertMapsTo('cjs-marker', 'cjs.cjs', 0);
await assertMapsTo('side-marker', 'side.js', 0);
await assertMapsTo('main-marker', 'main.js', 5);

await import(new URL('main.js', dist));
const [lazyValue, cjsValue, , lazyThrow] = globalThis.__rollipop_sourcemap_run();
assert.equal(lazyValue, 'lazy-marker');
assert.equal(cjsValue, 'cjs-marker');
assert.throws(lazyThrow, /lazy-throw-marker/);
