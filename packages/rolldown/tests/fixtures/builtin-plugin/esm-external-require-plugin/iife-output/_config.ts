import { defineTest } from 'rolldown-tests';
import { esmExternalRequirePlugin } from '@rollipop/rolldown/plugins';
import { expect } from 'vitest';

export default defineTest({
  config: {
    output: {
      format: 'iife',
    },
    plugins: [esmExternalRequirePlugin({ external: ['ext'] })],
  },
  async afterTest(output) {
    const code = output.output[0].code;
    expect(code).toContain('function(ext)');
    expect(code).toContain('module.exports = { ...ext }');
  },
});
