import { defineTest } from 'rolldown-tests';
import { getOutputChunk } from 'rolldown-tests/utils';
import { expect } from 'vitest';

export default defineTest({
  config: {
    input: 'main.tsx',
    external: ['react', 'react/compiler-runtime'],
    transform: {
      jsx: 'preserve',
      reactCompiler: {
        exclude: [/(^|[/\\])node_modules[/\\]/],
      },
    },
  },
  afterTest(output) {
    const code = getOutputChunk(output)[0].code;
    expect(code).toContain('from "react/compiler-runtime"');
    expect(code.match(/c\(2\)/g)).toHaveLength(1);
  },
});
