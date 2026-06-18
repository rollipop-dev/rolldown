import assert from 'node:assert/strict';
import { defineConfig } from '@rollipop/rolldown';

export default defineConfig((args) => {
  assert.strictEqual(args.customArg, 'customValue');
  return {
    input: './index.js',
  };
});
