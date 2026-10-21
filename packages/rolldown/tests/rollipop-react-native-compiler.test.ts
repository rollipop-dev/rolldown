import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runInNewContext } from 'node:vm';
import { rolldown } from '@rollipop/rolldown';
import {
  rollipopReactNativePlugin,
  RollipopReactNativeTransformer,
  type RollipopReactNativePluginConfig,
  type RollipopReactNativeReactConfig,
} from '@rollipop/rolldown/experimental';
import { describe, expect, test } from 'vitest';

type CompilerOptions = NonNullable<RollipopReactNativeReactConfig['compiler']>;

const component = `
  export function Probe({ value }) {
    const doubled = value * 2;
    return <probe value={doubled}/>;
  }
`;
const varComponent = component.replace('const doubled', 'var doubled');
const invalidHook = `
  import { useState } from 'react';
  export function Probe({ value }) {
    if (value > 2) useState(0);
    return <probe value={value}/>;
  }
`;

async function compile(
  source: string,
  {
    compiler = {},
    native = {},
    filename = 'Probe.jsx',
    cwd,
    refresh = false,
  }: {
    compiler?: CompilerOptions | null;
    native?: RollipopReactNativePluginConfig;
    filename?: string;
    cwd?: string;
    refresh?: boolean;
  } = {},
) {
  const warnings: string[] = [];
  const bundle = await rolldown({
    input: filename,
    cwd,
    external: (id) => id.startsWith('react'),
    transform: { jsx: { runtime: 'automatic', refresh } },
    onwarn: (warning) => warnings.push(warning.message),
    plugins: [
      {
        name: 'fixture',
        resolveId: (id) => (id === filename ? id : null),
        load: (id) => (id === filename ? source : null),
      },
      rollipopReactNativePlugin({
        ...native,
        swc: {
          ...native.swc,
          react: { compiler: compiler ?? undefined, ...native.swc?.react },
        },
      }),
    ],
  });
  try {
    const output = await bundle.generate({ format: 'cjs', sourcemap: true });
    const chunk = output.output[0];
    return { code: chunk.code, map: chunk.map, warnings };
  } finally {
    await bundle.close();
  }
}

function execute(code: string, modules: Record<string, unknown> = {}) {
  const caches: unknown[][] = [];
  let cursor = 0;
  const compilerRuntime = {
    c(size: number) {
      return (caches[cursor++] ??= Array(size).fill(Symbol.for('react.memo_cache_sentinel')));
    },
  };
  const jsxRuntime = { jsx: (type: string, props: unknown) => ({ type, props }) };
  const imports: Record<string, unknown> = {
    'react/compiler-runtime': compilerRuntime,
    'react-compiler-runtime': compilerRuntime,
    'react/jsx-runtime': jsxRuntime,
    react: { useState: (value: number) => [value, () => {}] },
    ...modules,
  };
  const module = { exports: {} as Record<string, unknown> };
  runInNewContext(code, {
    module,
    exports: module.exports,
    global: { Error },
    __DEV__: true,
    _WORKLET: false,
    $RefreshReg$: () => {},
    $RefreshSig$: () => (component: unknown) => component,
    require(id: string) {
      expect(Object.hasOwn(imports, id), `unexpected require: ${id}`).toBe(true);
      return imports[id];
    },
  });
  return {
    exports: module.exports,
    render(name: string, argument: unknown): unknown {
      cursor = 0;
      return (module.exports[name] as (argument: unknown) => unknown)(argument);
    },
  };
}

describe.each(['bundle', 'sync', 'async'] as const)(
  'native compiler binding renames: %s',
  (mode) => {
    describe.each(['hermes', 'hermes-v1'] as const)('%s', (runtimeTarget) => {
      const cases = [
        {
          name: 'successive callbacks with the same parameter name',
          body: 'const data = items.filter(item => item.value > 1).map(item => item.value);',
          value: 'data',
          first: [3],
          changed: [5],
        },
        {
          name: 'nested callbacks with the same parameter name',
          body: 'const data = items.map(item => item.children.map(item => item.value));',
          value: 'data',
          first: [[2], [4]],
          changed: [[6]],
        },
        {
          name: 'callbacks shadowing an outer data binding',
          body: 'const data = items.map(data => data.value); const more = items.map(data => data.value * 2);',
          value: '[data, more]',
          first: [
            [1, 3],
            [2, 6],
          ],
          changed: [[5], [10]],
        },
      ];
      describe.each(['jsx', 'tsx'])('%s', (extension) => {
        test.each(cases)('$name', async (fixture) => {
          const filename = `Probe.${extension}`;
          const source = `
            type Item = { value: number, children: Array<{ value: number }> };
            export function Probe({ items }: { items: Array<Item> }) {
              ${fixture.body}
              return <probe value={${fixture.value}} />;
            }
          `;
          for (const enabled of [false, true]) {
            let code: string;
            if (mode === 'bundle') {
              code = (
                await compile(source, {
                  filename,
                  compiler: enabled ? {} : null,
                  native: { runtimeTarget },
                })
              ).code;
            } else {
              const transformer = new RollipopReactNativeTransformer({
                runtimeTarget,
                swc: {
                  react: { runtime: 'Automatic', compiler: enabled ? {} : undefined },
                  module: { type: 'commonjs' },
                },
              });
              code = (
                mode === 'sync'
                  ? transformer.transformSync(filename, source)
                  : await transformer.transform(filename, source)
              ).code;
            }
            expect(code.includes('react/compiler-runtime')).toBe(enabled);
            const runtime = execute(code);
            const props = {
              items: [
                { value: 1, children: [{ value: 2 }] },
                { value: 3, children: [{ value: 4 }] },
              ],
            };
            const first = runtime.render('Probe', props);
            expect(first).toEqual({ type: 'probe', props: { value: fixture.first } });
            if (enabled) expect(runtime.render('Probe', props)).toBe(first);
            const changed = runtime.render('Probe', {
              items: [{ value: 5, children: [{ value: 6 }] }],
            });
            expect(changed).toEqual({ type: 'probe', props: { value: fixture.changed } });
            expect(changed).not.toBe(first);
          }
        });
      });
    });
  },
);

describe.each(['hermes', 'hermes-v1'] as const)('RN compiler integration: %s', (runtimeTarget) => {
  test.each([
    {
      filename: 'Probe.tsx',
      source: `type Props = { value: number }; ${component.replace('{ value })', '{ value }: Props)')}`,
    },
    {
      filename: 'Probe.jsx',
      source: `// @flow\ntype Props = $ReadOnly<{ value: number }>; ${component.replace('{ value })', '{ value }: Props)')}`,
    },
  ])('uses swc.react.compiler before lowering $filename', async ({ filename, source }) => {
    const result = await compile(source, { filename, native: { runtimeTarget } });
    expect(result.code).toContain('react/compiler-runtime');
    expect(result.code).not.toContain('type Props');
    expect(result.code).toMatch(/\bvar\b/);
    expect(result.warnings).toEqual([]);
    expect(result.map?.sourcesContent).toContain(source);
    const runtime = execute(result.code);
    const first = runtime.render('Probe', { value: 2 });
    expect(first).toEqual({ type: 'probe', props: { value: 4 } });
    expect(runtime.render('Probe', { value: 2 })).toBe(first);
    const changed = runtime.render('Probe', { value: 3 });
    expect(changed).not.toBe(first);
    expect(changed).toEqual({ type: 'probe', props: { value: 6 } });
  });
});

test('keeps the compiler disabled when no compiler configuration is supplied', async () => {
  const result = await compile(component, { compiler: null });
  expect(result.code).not.toContain('compiler-runtime');
  expect(result.warnings).toEqual([]);
  expect(execute(result.code).render('Probe', { value: 2 })).toEqual({
    type: 'probe',
    props: { value: 4 },
  });
});

describe.each(['sync', 'async'] as const)('standalone native compiler: %s', (mode) => {
  function compile(transformer: RollipopReactNativeTransformer, filename: string, source: string) {
    return mode === 'sync'
      ? transformer.transformSync(filename, source)
      : transformer.transform(filename, source);
  }

  test('opts in before type stripping and Hermes lowering', async () => {
    const source = `type Props = { value: number }; ${component.replace('{ value })', '{ value }: Props)')}`;
    for (const runtimeTarget of ['hermes', 'hermes-v1'] as const) {
      for (const enabled of [false, true]) {
        const transformer = new RollipopReactNativeTransformer({
          runtimeTarget,
          swc: {
            react: { runtime: 'Automatic', compiler: enabled ? {} : undefined },
            module: { type: 'commonjs' },
          },
        });
        const result = await compile(transformer, 'Probe.tsx', source);
        expect(result.code.includes('react/compiler-runtime')).toBe(enabled);
        expect(result.code).not.toContain('type Props');
        expect(result.warnings).toEqual([]);
        expect(JSON.parse(result.map).sourcesContent).toContain(source);
        const runtime = execute(result.code);
        const first = runtime.render('Probe', { value: 2 });
        expect(first).toEqual({ type: 'probe', props: { value: 4 } });
        expect(runtime.render('Probe', { value: 2 }) === first).toBe(enabled);
      }
    }
  });

  test('applies glob and regex filters to filenames without queries', async () => {
    for (const include of [['src/**'], [/Probe\.jsx$/]]) {
      const transformer = new RollipopReactNativeTransformer({
        swc: {
          react: {
            compiler: {
              include,
              exclude: [/[/\\]vendor[/\\]/],
              panicThreshold: 'all_errors',
            },
          },
        },
      });
      const included = join(process.cwd(), 'src', 'Probe.jsx');
      expect((await compile(transformer, `${included}?platform=ios`, component)).code).toContain(
        'react/compiler-runtime',
      );
      const excluded = await compile(
        transformer,
        join(process.cwd(), 'src', 'vendor', 'Probe.jsx'),
        `type Props = { value: number }; ${varComponent}`,
      );
      expect(excluded.code).not.toContain('compiler-runtime');
      expect(excluded.code).not.toContain('type Props');
      expect(excluded.warnings).toEqual([]);
      const unmatched = await compile(
        transformer,
        join(process.cwd(), 'lib', 'Other.jsx'),
        varComponent,
      );
      expect(unmatched.code).not.toContain('compiler-runtime');
      expect(unmatched.warnings).toEqual([]);
    }
  });

  test('returns nonfatal diagnostics and rejects fatal compiler errors', async () => {
    for (const compiler of [{ noEmit: true }, { outputMode: 'lint' as const }]) {
      const analysis = new RollipopReactNativeTransformer({
        swc: { react: { runtime: 'Automatic', compiler }, module: { type: 'commonjs' } },
      });
      const result = await compile(analysis, 'Probe.jsx', varComponent);
      expect(result.code).not.toContain('compiler-runtime');
      expect(result.warnings.join('\n')).toMatch(/var/);
      expect(execute(result.code).render('Probe', { value: 2 })).toEqual({
        type: 'probe',
        props: { value: 4 },
      });
    }

    const strict = new RollipopReactNativeTransformer({
      swc: { react: { compiler: { panicThreshold: 'all_errors' } } },
    });
    await expect(async () => compile(strict, 'Invalid.jsx', invalidHook)).rejects.toThrow(
      /Invalid\.jsx.*Hook/s,
    );
  });
});

test('keeps concurrent native compiler calls independent', async () => {
  const transformer = new RollipopReactNativeTransformer({
    swc: { react: { compiler: { panicThreshold: 'all_errors' } } },
  });
  const inputs = Array.from({ length: 8 }, (_, index) => ({
    filename: `Probe${index}.jsx`,
    source: component.replace('value * 2', `value * ${index + 1}`),
  }));
  const expected = inputs.map(({ filename, source }) =>
    transformer.transformSync(filename, source),
  );
  const actual = await Promise.all(
    inputs.map(({ filename, source }) => transformer.transform(filename, source)),
  );
  expect(actual).toEqual(expected);
  for (const result of actual) expect(result.code).toContain('react/compiler-runtime');
});

test('supports native Automatic JSX and CommonJS configuration', async () => {
  const result = await compile(component, {
    native: { swc: { react: { runtime: 'Automatic' }, module: { type: 'commonjs' } } },
  });
  expect(result.code).toContain('react/compiler-runtime');
  const runtime = execute(result.code);
  const first = runtime.render('Probe', { value: 2 });
  expect(first).toEqual({ type: 'probe', props: { value: 4 } });
  expect(runtime.render('Probe', { value: 2 })).toBe(first);
});

test('keeps unsupported var input buildable with the default panic threshold', async () => {
  const result = await compile(varComponent);
  expect(result.code).not.toContain('compiler-runtime');
  expect(execute(result.code).render('Probe', { value: 2 })).toEqual({
    type: 'probe',
    props: { value: 4 },
  });
});

test('honors strict all_errors for unsupported var input', async () => {
  await expect(
    compile(varComponent, { compiler: { panicThreshold: 'all_errors' } }),
  ).rejects.toThrow(/Probe\.jsx.*var/s);
});

test.each([{ noEmit: true }, { outputMode: 'lint' as const }])(
  'retains executable uncompiled output for analysis options %j',
  async (compiler) => {
    const result = await compile(component, { compiler });
    expect(result.code).not.toContain('compiler-runtime');
    expect(execute(result.code).render('Probe', { value: 2 })).toEqual({
      type: 'probe',
      props: { value: 4 },
    });
    const bailout = await compile(varComponent, { compiler });
    expect(bailout.code).not.toContain('compiler-runtime');
    expect(bailout.warnings.join('\n')).toMatch(/var/);
  },
);

test('keeps invalid hooks nonfatal by default and rejects them in strict modes', async () => {
  expect((await compile(invalidHook)).code).not.toContain('compiler-runtime');
  for (const panicThreshold of ['critical_errors', 'all_errors'] as const) {
    await expect(compile(invalidHook, { compiler: { panicThreshold } })).rejects.toThrow(/Hook/);
  }
});

test('annotation mode compiles opt-in functions without JSX', async () => {
  const result = await compile(
    `export function annotated(value) { 'use memo'; return { value }; }
     export function unmarked(value) { return { value }; }`,
    { filename: 'utility.js', compiler: { compilationMode: 'annotation' } },
  );
  const runtime = execute(result.code);
  const first = runtime.render('annotated', 2);
  expect(runtime.render('annotated', 2)).toBe(first);
  const unmarked = runtime.render('unmarked', 2);
  expect(runtime.render('unmarked', 2)).not.toBe(unmarked);
});

test('all mode compiles ordinary functions without infer-mode React patterns', async () => {
  const result = await compile('export function compute(value) { return { value }; }', {
    filename: 'utility.js',
    compiler: { compilationMode: 'all' },
  });
  expect(result.code).toContain('react/compiler-runtime');
  const runtime = execute(result.code);
  const first = runtime.render('compute', 2);
  expect(runtime.render('compute', 2)).toBe(first);
  expect(runtime.render('compute', 3)).toEqual({ value: 3 });
});

test('respects standard and custom opt-outs and the explicit override', async () => {
  const cases: [string, CompilerOptions][] = [
    ['use no memo', {}],
    ['skip compiler', { customOptOutDirectives: ['skip compiler'] }],
  ];
  for (const [directive, compiler] of cases) {
    const source = component.replace('const doubled', `'${directive}'; const doubled`);
    expect((await compile(source, { compiler })).code).not.toContain('compiler-runtime');
    const result = await compile(source, { compiler: { ...compiler, ignoreUseNoForget: true } });
    expect(result.code).toContain('react/compiler-runtime');
  }
});

test.each(['17', '18', '19'] as const)('selects the React %s compiler runtime', async (target) => {
  const result = await compile(component, { compiler: { target } });
  expect(result.code).toContain(
    target === '19' ? 'react/compiler-runtime' : 'react-compiler-runtime',
  );
  const runtime = execute(result.code);
  const first = runtime.render('Probe', { value: 2 });
  expect(runtime.render('Probe', { value: 2 })).toBe(first);
});

test('applies glob and regex filters relative to cwd while retaining RN transforms', async () => {
  const cwd = join(tmpdir(), 'rolldown-compiler-filters');
  const compiler: CompilerOptions = {
    include: ['src/**'],
    exclude: [/[/\\]vendor[/\\]/],
    panicThreshold: 'all_errors',
  };
  const included = join(cwd, 'src', 'Probe.jsx');
  const result = await compile(component, { compiler, cwd, filename: `${included}?platform=ios` });
  expect(result.code).toContain('react/compiler-runtime');
  for (const filename of [join(cwd, 'src', 'vendor', 'Probe.jsx'), join(cwd, 'lib', 'Probe.jsx')]) {
    const excluded = await compile(`type Props = { value: number }; ${varComponent}`, {
      compiler,
      cwd,
      filename,
    });
    expect(excluded.code).not.toContain('compiler-runtime');
    expect(excluded.code).not.toContain('type Props');
    expect(excluded.warnings).toEqual([]);
  }
  const regex: CompilerOptions = { include: [/Probe\.jsx$/] };
  expect(
    (await compile(component, { compiler: regex, cwd, filename: `${included}?platform=ios` })).code,
  ).toContain('react/compiler-runtime');
  expect(
    (await compile(component, { compiler: regex, cwd, filename: join(cwd, 'src', 'Other.jsx') }))
      .code,
  ).not.toContain('compiler-runtime');
});

type Worklet = ((value?: number) => unknown) & {
  __closure: unknown;
  __initData: { code: string };
  __workletHash: number;
};

test.each(['hermes', 'hermes-v1'] as const)(
  'preserves captured and automatic Worklets alongside the compiler for %s',
  async (runtimeTarget) => {
    const result = await compile(
      `import { useAnimatedStyle } from 'react-native-reanimated';
       const offset = 40;
       export function compute(value) { 'worklet'; return offset + value; }
       export function Probe({ value }) {
         const style = useAnimatedStyle(() => ({ opacity: value / 10 }));
         return <probe value={value * 2} style={style}/>;
       }`,
      {
        compiler: { enableReanimated: true },
        native: {
          runtimeTarget,
          worklets: { pluginVersion: '0.7.4', disableSourceMaps: true },
        },
      },
    );
    expect(result.code).toContain('react/compiler-runtime');
    const runtime = execute(result.code, {
      'react-native-reanimated': { useAnimatedStyle: (callback: unknown) => callback },
    });
    const compute = runtime.exports.compute as Worklet;
    expect(compute(2)).toBe(42);
    expect(compute.__workletHash).toEqual(expect.any(Number));
    const serialized = runInNewContext(`(${compute.__initData.code})`);
    expect(serialized.call({ __closure: compute.__closure }, 2)).toBe(42);
    const rendered = runtime.render('Probe', { value: 2 }) as { props: { style: Worklet } };
    const style = rendered.props.style;
    expect(style()).toEqual({ opacity: 0.2 });
    expect(style.__workletHash).toEqual(expect.any(Number));
    const serializedStyle = runInNewContext(`(${style.__initData.code})`);
    expect(serializedStyle.call({ __closure: style.__closure })).toEqual({ opacity: 0.2 });
  },
);

test('keeps OXC Fast Refresh alongside the native compiler backend', async () => {
  const result = await compile(component, { refresh: true });
  expect(result.code).toContain('react/compiler-runtime');
  expect(result.code).toContain('$RefreshReg$');
  const runtime = execute(result.code);
  const first = runtime.render('Probe', { value: 2 });
  expect(first).toEqual({ type: 'probe', props: { value: 4 } });
  expect(runtime.render('Probe', { value: 2 })).toBe(first);
});
