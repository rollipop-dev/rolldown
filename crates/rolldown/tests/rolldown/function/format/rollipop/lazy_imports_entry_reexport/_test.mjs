import assert from 'node:assert/strict';

await import('./dist/main.js');
// Entry re-exports run with the entry, as they do without lazy imports.
assert.deepEqual([...(globalThis.__rollipop_entry_reexport_log ?? [])].sort(), ['other', 'setup']);
