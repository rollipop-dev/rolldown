import { originalPositionFor, TraceMap } from '@jridgewell/trace-mapping';
import { rolldown } from '@rollipop/rolldown';
import {
  rollipopReactNativePlugin,
  RollipopReactNativeTransformer,
} from '@rollipop/rolldown/experimental';
import { describe, expect, test } from 'vitest';

const source = `import { useEffect, useState } from "react";
const prefix = "한글😀";
export function useCounter(initial: number) {
  const [value, setValue] = useState(initial);
  console.log("hook-body", value);
  useEffect(() => {
    console.log("hook-effect", value);
  }, [value]);
  const increment = () => {
    const next = value + 1;
    console.log("hook-callback", next);
    setValue(next);
  };
  return { value, increment };
}
export function Counter({ initial }: { initial: number }) {
  const counter = useCounter(initial);
  const label = "한글😀"; console.log("component-body", counter.value, label);
  return <View title={prefix} value={counter.value} onPress={counter.increment} />;
}
`;
const mappings = [
  ...['hook-body', 'hook-effect', 'hook-callback', 'component-body'].map((marker) => {
    const needle = `console.log("${marker}"`;
    return [needle, needle];
  }),
  ['setValue(next)', 'setValue(next)'],
  ['View,', 'View title'],
];

function expectBodyMappings(code: string, map: string | { toString(): string }) {
  const trace = new TraceMap(typeof map === 'string' ? map : map.toString());
  if (code.includes('react/compiler-runtime')) {
    // Both the component and custom hook must actually be compiled.
    expect(code.match(/\b_?c\)?\(\d+\)/g)).toHaveLength(2);
  }
  const generated = code.split('\n');
  const original = source.split('\n');
  for (const [needle, originalNeedle] of mappings) {
    const line = generated.findIndex((line) => line.includes(needle));
    const originalLine = original.findIndex((line) => line.includes(originalNeedle));
    expect(line, needle).toBeGreaterThanOrEqual(0);
    const position = originalPositionFor(trace, {
      line: line + 1,
      column: generated[line].indexOf(needle),
    });
    expect(position.source, needle).toEqual(expect.stringMatching(/Counter\.(tsx|jsx)$/));
    expect(position.line, needle).toBe(originalLine + 1);
    expect(position.column, needle).toBe(original[originalLine].indexOf(originalNeedle));
  }
}

describe.each(['Counter.tsx', 'Counter.jsx'])('Compiler body source maps: %s', (filename) => {
  describe.each(['hermes', 'hermes-v1'] as const)('%s', (runtimeTarget) => {
    test.each(['sync', 'async'] as const)(
      'preserves component and hook positions in %s output',
      async (mode) => {
        for (const enabled of [false, true]) {
          const transformer = new RollipopReactNativeTransformer({
            runtimeTarget,
            swc: { react: { runtime: 'Automatic', compiler: enabled ? {} : undefined } },
          });
          const output =
            mode === 'sync'
              ? transformer.transformSync(filename, source)
              : await transformer.transform(filename, source);
          expect(output.code.includes('react/compiler-runtime')).toBe(enabled);
          expect(output.warnings).toEqual([]);
          expectBodyMappings(output.code, output.map);
        }
      },
    );

    test.each([
      { format: 'esm', refresh: false },
      { format: 'esm', refresh: true },
      { format: 'cjs', refresh: false },
      { format: 'cjs', refresh: true },
    ] as const)(
      'preserves body mappings through the bundler ($format, refresh: $refresh)',
      async ({ format, refresh }) => {
        const bundle = await rolldown({
          input: filename,
          external: (id) => id.startsWith('react'),
          transform: { jsx: { runtime: 'automatic', refresh } },
          plugins: [
            {
              name: 'source-map-fixture',
              resolveId: (id) => (id === filename ? id : null),
              load: (id) => (id === filename ? source : null),
            },
            rollipopReactNativePlugin({ runtimeTarget, swc: { react: { compiler: {} } } }),
          ],
        });
        try {
          const { output } = await bundle.generate({ format, sourcemap: true });
          const chunk = output[0];
          expect(chunk.code).toContain('react/compiler-runtime');
          if (refresh) expect(chunk.code).toContain('$RefreshReg$');
          expectBodyMappings(chunk.code, chunk.map!);
        } finally {
          await bundle.close();
        }
      },
    );
  });
});
