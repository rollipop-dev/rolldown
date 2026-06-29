import { defineTest } from 'rolldown-tests';
import { getOutputChunk } from 'rolldown-tests/utils';
import { expect } from 'vitest';

export default defineTest({
  config: {
    input: 'main.tsx',
    external: ['react', 'react/compiler-runtime'],
    transform: {
      jsx: 'preserve',
      reactCompiler: {},
    },
  },
  afterTest(output) {
    const code = getOutputChunk(output)[0].code;
    expect(code).toContain('from "react/compiler-runtime"');
    expect(code).toContain('const $ = c(2);');
  },
});
