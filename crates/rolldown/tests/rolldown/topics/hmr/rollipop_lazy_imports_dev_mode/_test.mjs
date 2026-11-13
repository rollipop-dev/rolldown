import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// Dev mode (HMR) ignores `lazyImports`: the HMR runtime manages module records itself.
const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');
assert.doesNotMatch(code, /__rollipop_lazy__|new Proxy\(/);
assert.match(code, /var import_dep_\w+ = __rollipop_require__\(\d+\);/);
