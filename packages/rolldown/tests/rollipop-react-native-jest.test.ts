import { createRequire } from 'node:module';
import { runInNewContext } from 'node:vm';
import { originalPositionFor, TraceMap } from '@jridgewell/trace-mapping';
import { rolldown } from '@rollipop/rolldown';
import {
  rollipopReactNativePlugin,
  RollipopReactNativeTransformer,
} from '@rollipop/rolldown/experimental';
import type { TransformConfig } from '@swc/core';
import { isWasiTest } from 'rolldown-tests/utils';
import { beforeAll, describe, expect, test } from 'vitest';

const importCode = `
  import { value } from './dependency';
  jest.mock('./dependency', () => ({ value: 'mocked' }));
  record(value);
`;

function evaluate(code: string): string[] {
  const events: string[] = [];
  const factories = new Map<string, () => unknown>();
  type Jest = Record<string, (name?: string, factory?: () => unknown) => Jest>;
  const jest: Jest = {};
  for (const method of [
    'mock',
    'unmock',
    'deepUnmock',
    'enableAutomock',
    'disableAutomock',
    'doMock',
  ]) {
    jest[method] = (name, factory) => {
      events.push(name === undefined ? method : `${method}:${name}`);
      if (name && factory) factories.set(name, factory);
      return jest;
    };
  }
  runInNewContext(`(function(exports, require) {${code}\n})(exports, require);`, {
    exports: {},
    jest,
    require: (name: string) => {
      events.push(`require:${name}`);
      return factories.get(name)?.() ?? { value: 'real', jest };
    },
    record: (value: string) => events.push(value),
    React: {
      createElement: (tag: string, _props: unknown, value: string) => `${tag}:${value}`,
    },
  });
  return events;
}

describe('native Jest hoisting', () => {
  test('registers mocks before lowered imports through sync and async NAPI', async () => {
    const transformer = new RollipopReactNativeTransformer({
      swc: { jest: true, module: { type: 'commonjs' } },
    });
    const results = [
      transformer.transformSync('input.js', importCode),
      await transformer.transform('input.js', importCode),
    ];
    for (const result of results) {
      expect(evaluate(result.code)).toEqual([
        'mock:./dependency',
        'require:./dependency',
        'mocked',
      ]);
    }
  });

  test.each([undefined, false])('does not hoist when jest=%s', (jest) => {
    const transformer = new RollipopReactNativeTransformer({
      swc: { jest, module: { type: 'commonjs' } },
    });
    expect(evaluate(transformer.transformSync('input.js', importCode).code)).toEqual([
      'require:./dependency',
      'mock:./dependency',
      'real',
    ]);
  });

  test.each(['hermes', 'hermes-v1'] as const)(
    'matches SWC name-based hoisting for local bindings on %s',
    (runtimeTarget) => {
      const transformer = new RollipopReactNativeTransformer({
        runtimeTarget,
        swc: { jest: true },
      });
      const parameter = transformer.transformSync(
        'parameter.js',
        `
        function run(jest) {
          record('parameter-before');
          jest.mock('parameter');
        }
        run({ mock(value) { record(value); } });
      `,
      );
      expect(evaluate(parameter.code)).toEqual(['parameter', 'parameter-before']);

      const variable = transformer.transformSync(
        'variable.js',
        `
        const jest = { mock(value) { record(value); } };
        jest.mock('variable');
      `,
      );
      // Deliberately preserve upstream behavior: the call moves before initialization.
      expect(() => evaluate(variable.code)).toThrow();
    },
  );

  test('does not mistake imported Jest for the global binding', () => {
    const transformer = new RollipopReactNativeTransformer({
      swc: { jest: true, module: { type: 'commonjs' } },
    });
    const result = transformer.transformSync(
      'input.js',
      `
      import { jest } from '@jest/globals';
      record('before');
      jest.mock('imported');
    `,
    );
    expect(evaluate(result.code)).toEqual(['require:@jest/globals', 'before', 'mock:imported']);
  });

  test('keeps hoisted calls and chains in their block and in source order', () => {
    const transformer = new RollipopReactNativeTransformer({ swc: { jest: true } });
    const result = transformer.transformSync(
      'input.js',
      `
      record('outer-before');
      {
        record('inner-before');
        jest.mock('first').unmock('second');
        jest.deepUnmock('third');
        jest.disableAutomock().enableAutomock();
        record('inner-after');
      }
      record('outer-after');
    `,
    );
    expect(evaluate(result.code)).toEqual([
      'outer-before',
      'mock:first',
      'unmock:second',
      'deepUnmock:third',
      'disableAutomock',
      'enableAutomock',
      'inner-before',
      'inner-after',
      'outer-after',
    ]);
  });

  test('preserves directive prologues when hoisting inside a function', () => {
    const transformer = new RollipopReactNativeTransformer({ swc: { jest: true } });
    const result = transformer.transformSync(
      'input.js',
      `
      function run() {
        'use strict';
        record(this === undefined ? 'strict' : 'loose');
        jest.mock('nested');
      }
      record('before');
      run();
    `,
    );
    expect(evaluate(result.code)).toEqual(['before', 'mock:nested', 'strict']);
  });

  test('matches SWC hoisting of mixed chains while leaving standalone doMock in place', () => {
    const transformer = new RollipopReactNativeTransformer({ swc: { jest: true } });
    const result = transformer.transformSync(
      'input.js',
      `
      record('before');
      jest.doMock('first');
      jest.doMock('second').mock('third');
      jest.mock('fourth').doMock('fifth');
      record('after');
    `,
    );
    expect(evaluate(result.code)).toEqual([
      'doMock:second',
      'mock:third',
      'before',
      'doMock:first',
      'mock:fourth',
      'doMock:fifth',
      'after',
    ]);
  });

  test.each([
    { filename: 'input.tsx', directive: '' },
    { filename: 'input.jsx', directive: '// @flow' },
  ])('composes with type stripping and JSX in $filename', ({ filename, directive }) => {
    const transformer = new RollipopReactNativeTransformer({
      swc: { jest: true, module: { type: 'commonjs' }, react: { runtime: 'Classic' } },
    });
    const result = transformer.transformSync(
      filename,
      `
      ${directive}
      import { value } from './dependency';
      jest.mock('./dependency', () => ({ value: 'mocked' }));
      const label: string = value;
      record(<span>{label}</span>);
    `,
    );
    expect(evaluate(result.code)).toEqual([
      'mock:./dependency',
      'require:./dependency',
      'span:mocked',
    ]);
  });

  test.each([false, true])('passes jest=%s through the builtin plugin', async (jest) => {
    const entry = '\0jest.ts';
    const bundle = await rolldown({
      input: entry,
      plugins: [
        {
          name: 'jest-fixture',
          resolveId: (id) => (id === entry ? entry : null),
          load: (id) =>
            id === entry
              ? {
                  code: `record('before'); jest.mock('fixture'); record('after');`,
                  moduleType: 'ts',
                }
              : null,
        },
        rollipopReactNativePlugin({ swc: { jest } }),
      ],
    });
    try {
      const { output } = await bundle.generate({ format: 'iife' });
      expect(evaluate(output[0].code)).toEqual(
        jest ? ['mock:fixture', 'before', 'after'] : ['before', 'mock:fixture', 'after'],
      );
    } finally {
      await bundle.close();
    }
  });
});

// WASI and Windows ARM64 bindings are built without the wasm_plugins feature.
const unsupportedWasmPlugins =
  isWasiTest || (process.platform === 'win32' && process.arch === 'arm64');

describe.skipIf(unsupportedWasmPlugins)('SWC Jest compatibility', () => {
  test.each([
    { name: 'lowered imports', code: importCode },
    {
      name: 'parameter shadowing',
      code: `
        function run(jest) {
          record('before');
          jest.mock('local');
        }
        run({ mock(value) { record(value); } });
      `,
    },
    {
      name: 'mixed chains',
      code: `
        record('before');
        jest.doMock('first').mock('second');
        jest.mock('third').doMock('fourth');
      `,
    },
    {
      name: 'imported Jest',
      code: `
        import { jest } from '@jest/globals';
        record('before');
        jest.mock('imported');
      `,
    },
  ])('matches @swc/core for $name', async ({ code }) => {
    const { transformSync } = await import('@swc/core');
    // @swc/jest uses this internal option, which @swc/core omits from its public types.
    const transform: TransformConfig & { hidden: { jest: boolean } } = {
      hidden: { jest: true },
    };
    const reference = transformSync(code, {
      filename: 'input.js',
      jsc: { target: 'es2015', transform },
      module: { type: 'commonjs' },
    });
    const transformer = new RollipopReactNativeTransformer({
      swc: { jest: true, module: { type: 'commonjs' } },
    });
    const result = transformer.transformSync('input.js', code);
    expect(evaluate(result.code)).toEqual(evaluate(reference.code));
  });
});

describe.skipIf(unsupportedWasmPlugins)('native Jest coverage instrumentation', () => {
  let transformer: RollipopReactNativeTransformer;

  beforeAll(() => {
    transformer = new RollipopReactNativeTransformer({
      swc: {
        jest: true,
        module: { type: 'commonjs' },
        plugins: [
          [
            createRequire(import.meta.url).resolve('swc-plugin-coverage-instrument'),
            { coverageVariable: '__nativeCoverage__' },
          ],
        ],
      },
    });
  }, 120_000);

  test.each(['sync', 'async'] as const)(
    'preserves hoisting, counters, and source locations through %s NAPI',
    async (mode) => {
      const filename = 'native-coverage.ts';
      const source = [
        `import { value } from './dependency';`,
        `jest.mock('./dependency', () => ({ value: 42 }));`,
        `export function calculate(flag: boolean) {`,
        `  if (flag) return value;`,
        `  return 0;`,
        `}`,
        `globalThis.results = [calculate(true), calculate(false)];`,
        `export function fail() {`,
        `  throw new Error('coverage-source');`,
        `}`,
      ].join('\n');
      const result =
        mode === 'sync'
          ? transformer.transformSync(filename, source)
          : await transformer.transform(filename, source);
      type FileCoverage = {
        s: Record<string, number>;
        f: Record<string, number>;
        b: Record<string, number[]>;
        fnMap: Record<string, { name: string }>;
        statementMap: Record<string, { start: { line: number } }>;
      };
      const factories = new Map<string, () => unknown>();
      const context = {
        exports: {},
        jest: { mock: (name: string, factory: () => unknown) => factories.set(name, factory) },
        require: (name: string) => factories.get(name)?.() ?? { value: 'real' },
        results: undefined as unknown,
        __nativeCoverage__: {} as Record<string, FileCoverage>,
      };
      runInNewContext(result.code, context);
      expect(context.results).toEqual([42, 0]);
      expect(context).not.toHaveProperty('__coverage__');
      const files = Object.values(context.__nativeCoverage__);
      expect(files).toHaveLength(1);
      const coverage = files[0];
      const calls = Object.fromEntries(
        Object.entries(coverage.fnMap).map(([id, { name }]) => [name, coverage.f[id]]),
      );
      expect(calls).toMatchObject({ calculate: 2, fail: 0 });
      const statements = Object.fromEntries(
        Object.entries(coverage.statementMap).map(([id, { start }]) => [
          start.line,
          coverage.s[id],
        ]),
      );
      expect(statements).toMatchObject({ 7: 1, 9: 0 });
      const branches = Object.values(coverage.b).flat();
      expect(branches.length).toBeGreaterThan(0);
      expect(branches.every((count) => count === 1)).toBe(true);

      expect(result.map).toBeDefined();
      const map = new TraceMap(result.map!);
      const lines = result.code.split('\n');
      for (const [marker, sourceLine] of [
        ['jest.mock', 2],
        ['throw new Error', 9],
      ] as const) {
        const line = lines.findIndex((line) => line.trimStart().startsWith(marker));
        expect(line).toBeGreaterThanOrEqual(0);
        const original = originalPositionFor(map, {
          line: line + 1,
          column: lines[line].indexOf(marker),
        });
        expect(original.source).toMatch(/native-coverage\.ts$/);
        expect(original.line).toBe(sourceLine);
      }
    },
  );
});
