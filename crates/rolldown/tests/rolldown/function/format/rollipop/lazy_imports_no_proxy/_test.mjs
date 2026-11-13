import assert from 'node:assert/strict';

// Lazy holders don't need Proxy, so imports stay lazy without it.
const OriginalProxy = globalThis.Proxy;
globalThis.Proxy = undefined;
try {
  await import('./dist/main.js');
} finally {
  globalThis.Proxy = OriginalProxy;
}
const result = globalThis.__rollipop_lazy_no_proxy_result;
assert.deepEqual(result.log, ['main', 'dep', 'cjs']);
assert.equal(result.value, 'dep-value');
assert.equal(result.cjs, 'cjs-named');
assert.equal(result.named, 'cjs-named');
