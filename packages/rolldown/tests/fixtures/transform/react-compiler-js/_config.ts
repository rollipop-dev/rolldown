import { defineTest } from 'rolldown-tests';
import { getOutputChunk } from 'rolldown-tests/utils';
import { expect } from 'vitest';

export default defineTest({
  config: {
    input: 'main.js',
    external: ['react', 'react/compiler-runtime'],
    transform: {
      reactCompiler: {},
    },
  },
  afterTest(output) {
    const code = getOutputChunk(output)[0].code;
    expect(code).toContain('from "react/compiler-runtime"');
    expect(code).toContain('c(');
  },
});
