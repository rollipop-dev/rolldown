import { copyFileSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runInNewContext } from 'node:vm';
import { rolldown } from '@rollipop/rolldown';
import {
  rollipopReactNativePlugin,
  RollipopReactNativeTransformer,
} from '@rollipop/rolldown/experimental';
import type { Options, transformSync as transformWithSwc } from '@swc/core';
import { isWasiTest } from 'rolldown-tests/utils';
import { afterAll, beforeAll, describe, expect, test } from 'vitest';

const require = createRequire(import.meta.url);
// WASI and Windows ARM64 bindings are built without the wasm_plugins feature.
const unsupportedWasmPlugins =
  isWasiTest || (process.platform === 'win32' && process.arch === 'arm64');
let transformSync: typeof transformWithSwc;
let plugins: [string, Record<string, unknown>][];
let cacheRoot: string | undefined;

beforeAll(async () => {
  if (unsupportedWasmPlugins) return;
  ({ transformSync } = await import('@swc/core'));
  plugins = [[require.resolve('@swc/plugin-remove-console'), { exclude: ['error'] }]];
  cacheRoot = mkdtempSync(join(tmpdir(), 'rolldown-swc-'));
});
afterAll(() => {
  if (cacheRoot) rmSync(cacheRoot, { recursive: true, force: true });
});

const fixtures = [
  { syntax: 'ecmascript', filename: 'input.js', annotation: '' },
  {
    syntax: 'typescript',
    filename: 'input.ts',
    annotation: ': { log(value: string): void }',
  },
  {
    syntax: 'flow',
    filename: 'input.flow.js',
    annotation: ': { log: (value: string) => void }',
  },
] as const;

const orderingCode = `
  console.log('global-log');
  console!.log('non-null-log');
  record('retained-side-effect');
`;

describe.skipIf(unsupportedWasmPlugins)('native SWC compiled module cache', () => {
  test('reuses loaded modules across transformers for the process lifetime, like SWC', () => {
    const path = join(cacheRoot!, 'replaceable.wasm');
    copyFileSync(plugins[0][0], path);
    const options = { swc: { plugins: [[path, { exclude: ['error'] }]] as typeof plugins } };
    const first = new RollipopReactNativeTransformer(options);
    const code = `console.log('log'); console.error('error');`;
    expect(evaluate(first.transformSync('first.js', code).code)).toEqual(['error:error']);

    writeFileSync(path, 'invalid WASM');
    const cached = new RollipopReactNativeTransformer(options);
    expect(evaluate(cached.transformSync('cached.js', code).code)).toEqual(['error:error']);
    rmSync(path);
    const deleted = new RollipopReactNativeTransformer(options);
    expect(evaluate(deleted.transformSync('deleted.js', code).code)).toEqual(['error:error']);
  });

  test('rejects an invalid uncached plugin and can retry after it is fixed', () => {
    const path = join(cacheRoot!, 'invalid.wasm');
    const options = { swc: { plugins: [[path, { exclude: ['error'] }]] as typeof plugins } };
    writeFileSync(path, 'invalid WASM');
    expect(() => new RollipopReactNativeTransformer(options)).toThrow(/Failed to load wasm plugin/);
    copyFileSync(plugins[0][0], path);
    const restored = new RollipopReactNativeTransformer(options);
    const code = `console.log('log'); console.error('error');`;
    expect(evaluate(restored.transformSync('restored.js', code).code)).toEqual(['error:error']);
  });

  test('keeps configurations isolated across transformers sharing a WASM module', async () => {
    const path = plugins[0][0];
    const keepErrors = new RollipopReactNativeTransformer({
      swc: { plugins: [[path, { exclude: ['error'] }]] },
    });
    const keepWarnings = new RollipopReactNativeTransformer({
      swc: { plugins: [[path, { exclude: ['warn'] }]] },
    });
    const code = `console.error('error'); console.warn('warn'); console.log('log');`;
    const results = await Promise.all([
      keepErrors.transform('first.js', code),
      keepWarnings.transform('second.js', code),
      keepErrors.transform('third.js', code),
    ]);
    expect(results.map((result) => evaluate(result.code))).toEqual([
      ['error:error'],
      ['warn:warn'],
      ['error:error'],
    ]);
  });
});

function evaluate(code: string): string[] {
  const events: string[] = [];
  runInNewContext(code, {
    console: {
      log: (value: string) => events.push(`log:${value}`),
      warn: (value: string) => events.push(`warn:${value}`),
      error: (value: string) => events.push(`error:${value}`),
    },
    record: (value: string) => events.push(value),
  });
  return events;
}

describe.skipIf(unsupportedWasmPlugins).each([undefined, false, true])(
  'native SWC runPluginFirst=%s',
  (runPluginFirst) => {
    // Before stripping, the plugin cannot match console wrapped in TsNonNullExpr.
    const orderedEvents = runPluginFirst
      ? ['log:non-null-log', 'retained-side-effect']
      : ['retained-side-effect'];

    test.each(fixtures)(
      '$syntax matches @swc/core in sync and async transforms',
      async (fixture) => {
        const { syntax, filename, annotation } = fixture;
        const code = `
      ${syntax === 'flow' ? '// @flow' : ''}
      console.log('global-log');
      console.warn('global-warn');
      console.error('global-error');
      function logLocally(console${annotation}) {
        console.log('local-log');
      }
      logLocally({ log: (value) => record(value) });
      record('retained-side-effect');
    `;
        const options: Options = {
          filename,
          configFile: false,
          swcrc: false,
          jsc: { parser: { syntax }, target: 'esnext' },
        };
        const baseline = transformSync(code, options);
        expect(evaluate(baseline.code)).toEqual([
          'log:global-log',
          'warn:global-warn',
          'error:global-error',
          'local-log',
          'retained-side-effect',
        ]);

        const reference = transformSync(code, {
          ...options,
          jsc: { ...options.jsc, experimental: { plugins, runPluginFirst, cacheRoot } },
        });
        const expected = ['error:global-error', 'local-log', 'retained-side-effect'];
        expect(evaluate(reference.code)).toEqual(expected);

        const transformer = new RollipopReactNativeTransformer({
          flow: { requireDirective: true },
          swc: { plugins, runPluginFirst },
        });
        expect(evaluate(transformer.transformSync(filename, code).code)).toEqual(expected);
        expect(evaluate((await transformer.transform(filename, code)).code)).toEqual(expected);
      },
    );

    test('matches @swc/core plugin ordering around TypeScript stripping', async () => {
      const filename = 'ordering.ts';
      const reference = transformSync(orderingCode, {
        filename,
        configFile: false,
        swcrc: false,
        jsc: {
          parser: { syntax: 'typescript' },
          target: 'esnext',
          experimental: { plugins, runPluginFirst, cacheRoot },
        },
      });
      expect(evaluate(reference.code)).toEqual(orderedEvents);

      const transformer = new RollipopReactNativeTransformer({ swc: { plugins, runPluginFirst } });
      const syncResult = transformer.transformSync(filename, orderingCode);
      const asyncResult = await transformer.transform(filename, orderingCode);
      for (const result of [syncResult, asyncResult]) {
        expect(evaluate(result.code)).toEqual(orderedEvents);
        expect(result.code).not.toMatch(/console\s*!/);
      }
    });

    test('passes plugin ordering through the builtin plugin configuration', async () => {
      const entry = '\0ordering.ts';
      const bundle = await rolldown({
        input: entry,
        plugins: [
          {
            name: 'ordering-fixture',
            resolveId(id) {
              if (id === entry) return entry;
            },
            load(id) {
              if (id === entry) return { code: orderingCode, moduleType: 'ts' };
            },
          },
          rollipopReactNativePlugin({ swc: { plugins, runPluginFirst } }),
        ],
      });
      try {
        const { output } = await bundle.generate({ format: 'iife' });
        expect(evaluate(output[0].code)).toEqual(orderedEvents);
        expect(output[0].code).not.toMatch(/console\s*!/);
      } finally {
        await bundle.close();
      }
    });
  },
);
