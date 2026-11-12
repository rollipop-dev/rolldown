import { originalPositionFor, TraceMap } from '@jridgewell/trace-mapping';
import { rolldown } from '@rollipop/rolldown';
import {
  rollipopReactNativePlugin,
  RollipopReactNativeTransformer,
} from '@rollipop/rolldown/experimental';
import { describe, expect, test } from 'vitest';

async function compile(mode: 'sync' | 'async' | 'bundle', filename: string, source: string) {
  if (mode !== 'bundle') {
    const transformer = new RollipopReactNativeTransformer();
    return mode === 'sync'
      ? transformer.transformSync(filename, source)
      : transformer.transform(filename, source);
  }
  const bundle = await rolldown({
    input: filename,
    treeshake: false,
    external: (id) => id.startsWith('react-native'),
    plugins: [
      {
        name: 'codegen-fixture',
        resolveId: (id) => (id === filename ? id : null),
        load: (id) => (id === filename ? source : null),
      },
      rollipopReactNativePlugin(),
    ],
  });
  try {
    const { output } = await bundle.generate({ format: 'esm', sourcemap: true });
    const chunk = output.find((item) => item.type === 'chunk')!;
    return { code: chunk.code, map: chunk.map!.toString() };
  } finally {
    await bundle.close();
  }
}

describe.each([
  {
    filename: 'ProbeNativeComponent.ts',
    unresolved: 'interface NativeProps extends ExternalProps { enabled?: boolean; }',
    local:
      'interface SharedProps { enabled?: boolean; }\ninterface NativeProps extends ViewProps, SharedProps {}',
  },
  {
    filename: 'ProbeNativeComponent.js',
    unresolved: 'type NativeProps = $ReadOnly<{ ...ExternalProps, enabled?: boolean }>;',
    local:
      'type SharedProps = $ReadOnly<{ enabled?: boolean }>;\ntype NativeProps = $ReadOnly<{ ...ViewProps, ...SharedProps }>;',
  },
])('native codegen: $filename', ({ filename, unresolved, local }) => {
  const imports = `import type { ViewProps } from 'react-native';
import codegenNativeComponent from 'react-native/Libraries/Utilities/codegenNativeComponent';`;
  const component = "export default codegenNativeComponent<NativeProps>('CodegenProbe');";

  describe.each(['sync', 'async', 'bundle'] as const)('%s', (mode) => {
    test('rejects unresolved inherited props', async () => {
      const source = `${imports}\n${unresolved}\n${component}`;
      await expect(compile(mode, filename, source)).rejects.toThrow(
        'Unable to handle prop spread: ExternalProps',
      );
    });

    test('rejects invalid Commands exports', async () => {
      const source = `${imports}\n${local}\nexport const Commands = {};\n${component}`;
      await expect(compile(mode, filename, source)).rejects.toThrow(
        "'Commands' is a reserved export",
      );
    });

    test('preserves original sources and positions around generated view configs', async () => {
      const source = `${imports}
export function before() { const label = "한글😀"; console.log("before", label); }
${local}
${component}
export function after() { const label = "한글😀"; console.log("after", label); }
`;
      const output = await compile(mode, filename, source);
      expect(output.code).toContain('__INTERNAL_VIEW_CONFIG');
      expect(output.code).toContain('enabled');
      expect(output.code).not.toContain('codegenNativeComponent(');
      const trace = new TraceMap(output.map);
      expect(trace.sources.some((name) => name?.includes('<generated>'))).toBe(false);
      const sourceIndex = trace.sources.findIndex((name) => name?.endsWith(filename));
      expect(sourceIndex).toBeGreaterThanOrEqual(0);
      expect(trace.sourcesContent?.[sourceIndex]).toBe(source);

      const generated = output.code.split('\n');
      const original = source.split('\n');
      for (const marker of ['before', 'after']) {
        const needle = `console.log("${marker}"`;
        const line = generated.findIndex((text) => text.includes(needle));
        const originalLine = original.findIndex((text) => text.includes(needle));
        expect(line, marker).toBeGreaterThanOrEqual(0);
        const position = originalPositionFor(trace, {
          line: line + 1,
          column: generated[line].indexOf(needle),
        });
        expect(position.source?.endsWith(filename), marker).toBe(true);
        expect(position.line, marker).toBe(originalLine + 1);
        expect(position.column, marker).toBe(original[originalLine].indexOf(needle));
      }
    });
  });
});
