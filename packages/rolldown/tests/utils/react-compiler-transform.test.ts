import path from 'node:path';
import { rolldown } from '@rollipop/rolldown';
import { transform } from '@rollipop/rolldown/utils';
import { expect, describe, it } from 'vitest';

describe('enhanced transform react compiler', () => {
  const bailoutCode = `import { useRef } from 'react';
    export function Component() { const ref = useRef(0); return <Text>{ref.current}</Text>; }`;

  it.each(['none'] as const)(
    'keeps nonfatal compiler diagnostics as warnings (%s)',
    async (panicThreshold) => {
      const result = await transform('Component.tsx', bailoutCode, {
        jsx: { compiler: { panicThreshold } },
      });
      expect(result.errors).toHaveLength(0);
      expect(result.warnings.length).toBeGreaterThan(0);
      expect(result.code).toContain('ref.current');
    },
  );

  it.each(['critical_errors', 'all_errors'] as const)(
    'preserves fatal compiler diagnostics (%s)',
    async (panicThreshold) => {
      const result = await transform('Component.tsx', bailoutCode, {
        jsx: { compiler: { panicThreshold } },
      });
      expect(result.errors.length).toBeGreaterThan(0);
    },
  );

  it.each(['none', 'all_errors'] as const)(
    'respects compiler fatality when bundling (%s)',
    async (panicThreshold) => {
      const warnings: string[] = [];
      const bundle = await rolldown({
        input: 'Component.tsx',
        external: ['react', 'react/jsx-runtime'],
        transform: { jsx: { compiler: { panicThreshold } } },
        onwarn: (warning) => warnings.push(warning.message),
        plugins: [
          {
            name: 'fixture',
            resolveId: (id) => (id === 'Component.tsx' ? id : null),
            load: (id) => (id === 'Component.tsx' ? bailoutCode : null),
          },
        ],
      });
      try {
        if (panicThreshold === 'all_errors') {
          await expect(bundle.generate({ format: 'esm' })).rejects.toThrow(
            'Cannot access refs during render',
          );
        } else {
          const result = await bundle.generate({ format: 'esm' });
          expect(result.output[0].code).toContain('ref.current');
          expect(
            warnings.some((warning) => warning.includes('Cannot access refs during render')),
          ).toBe(true);
        }
      } finally {
        await bundle.close();
      }
    },
  );

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
      jsx: {
        compiler: {},
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).toContain('from "react/compiler-runtime"');
    expect(result.code).toContain('_c(2)');
  });

  it('should apply React Compiler options', async () => {
    const result = await transform('Counter.tsx', code, {
      jsx: {
        compiler: {
          target: '17',
        },
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).toContain('from "react-compiler-runtime"');
  });

  it('should not exclude node_modules by default', async () => {
    const result = await transform(path.join('node_modules', 'pkg', 'Counter.tsx'), code, {
      jsx: {
        compiler: {},
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).toContain('from "react/compiler-runtime"');
    expect(result.code).toContain('_c(2)');
  });

  it('should skip React Compiler for excluded files', async () => {
    const result = await transform(path.join('node_modules', 'pkg', 'Counter.tsx'), code, {
      jsx: {
        compiler: {
          exclude: [/(^|[/\\])node_modules[/\\]/],
        },
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).not.toContain('react/compiler-runtime');
    expect(result.code).not.toContain('_c(');
  });

  it('should only run React Compiler for included files', async () => {
    const result = await transform('Counter.tsx', code, {
      jsx: {
        compiler: {
          include: ['**/src/**'],
        },
      },
    });
    expect(result.errors).toHaveLength(0);
    expect(result.code).not.toContain('react/compiler-runtime');
    expect(result.code).not.toContain('_c(');
  });
});
