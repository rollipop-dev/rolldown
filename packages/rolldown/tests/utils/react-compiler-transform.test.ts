import path from 'node:path';
import { transform } from '@rollipop/rolldown/utils';
import { expect, describe, it } from 'vitest';

describe('enhanced transform react compiler', () => {
  const code =
    'import * as React from "react"; export function Counter({ count }) { return <Text>{count}</Text>; }';

  it('should be disabled by default', async () => {
    const result = await transform('Counter.tsx', code, {
      jsx: 'preserve',
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).not.toContain('react/compiler-runtime');
    expect(result.code).not.toContain('_c(');
  });

  it('should enable React Compiler with default options', async () => {
    const result = await transform('Counter.tsx', code, {
      jsx: 'preserve',
      reactCompiler: {},
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).toContain('from "react/compiler-runtime"');
    expect(result.code).toContain('_c(2)');
  });

  it('should apply React Compiler options', async () => {
    const result = await transform('Counter.tsx', code, {
      jsx: 'preserve',
      reactCompiler: {
        target: '17',
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).toContain('from "react-compiler-runtime"');
  });

  it('should not exclude node_modules by default', async () => {
    const result = await transform(path.join('node_modules', 'pkg', 'Counter.tsx'), code, {
      jsx: 'preserve',
      reactCompiler: {},
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).toContain('from "react/compiler-runtime"');
    expect(result.code).toContain('_c(2)');
  });

  it('should skip React Compiler for excluded files', async () => {
    const result = await transform(path.join('node_modules', 'pkg', 'Counter.tsx'), code, {
      jsx: 'preserve',
      reactCompiler: {
        exclude: [/(^|[/\\])node_modules[/\\]/],
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).not.toContain('react/compiler-runtime');
    expect(result.code).not.toContain('_c(');
  });

  it('should only run React Compiler for included files', async () => {
    const result = await transform('Counter.tsx', code, {
      jsx: 'preserve',
      reactCompiler: {
        include: ['**/src/**'],
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).not.toContain('react/compiler-runtime');
    expect(result.code).not.toContain('_c(');
  });
});
