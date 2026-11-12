import { runInNewContext } from 'node:vm';
import { rolldown, type InputOptions, type OutputOptions } from '@rollipop/rolldown';
import { dev } from '@rollipop/rolldown/experimental';
import { expect, test } from 'vitest';

async function generate(
  sources: Record<string, string>,
  outputOptions: OutputOptions = {},
  devMode = false,
  treeshake = false,
) {
  const inputOptions: InputOptions = {
    input: 'entry.js',
    treeshake,
    plugins: [{ name: 'fixture', resolveId: (id) => id, load: (id) => sources[id] }],
  };
  const output: OutputOptions = { format: 'rollipop', codeSplitting: false, ...outputOptions };
  let code: string | undefined;
  if (devMode) {
    const engine = await dev(
      {
        ...inputOptions,
        experimental: { devMode: { implement: '', skipCommonRuntimeInjection: true } },
      },
      output,
      {
        watch: { enabled: false, skipWrite: true },
        onOutput(result) {
          if (result instanceof Error) throw result;
          code = result.output.find((item) => item.type === 'chunk')!.code;
        },
      },
    );
    try {
      await engine.run();
      await engine.ensureCurrentBuildFinish();
    } finally {
      await engine.close();
    }
  } else {
    const bundle = await rolldown(inputOptions);
    try {
      const result = await bundle.generate(output);
      code = result.output.find((item) => item.type === 'chunk')!.code;
    } finally {
      await bundle.close();
    }
  }

  return code!;
}

async function execute(
  sources: Record<string, string>,
  outputOptions: OutputOptions = {},
  devMode = false,
  treeshake = false,
) {
  const code = await generate(sources, outputOptions, devMode, treeshake);
  const context: { result?: unknown; complete?: Promise<unknown>; __rolldown_runtime__?: unknown } =
    {};
  if (devMode) {
    const { DevRuntime } = await import(
      import.meta.resolve('@rollipop/rolldown/experimental/runtime')
    );
    context.__rolldown_runtime__ = new DevRuntime('namespace-test');
  }
  runInNewContext(code, context);
  await context.complete;
  return context.result;
}

const variants = [false, true].flatMap((treeshake) =>
  [false, true].flatMap((devMode) =>
    [false, true].flatMap((minify) =>
      [false, true].map((profilerNames) => ({ treeshake, devMode, minify, profilerNames })),
    ),
  ),
);

const explicitMarkerSources = {
  'entry.js': `
    import * as ns from 'dep.js';
    import * as barrel from 'barrel.js';
    import { __esModule as marker } from 'dep.js';
    const required = require('dep.js');
    const before = [
      ns.__esModule, marker, barrel.__esModule,
      Object.keys(ns).includes('__esModule'),
      Object.getOwnPropertyDescriptor(ns, '__esModule').get !== undefined,
      required.__esModule, Object.keys(required).includes('__esModule'),
      required.value, required.default, required !== ns,
      Object.prototype.toString.call(ns), Object.prototype.toString.call(required)
    ];
    ns.update();
    globalThis.complete = import('dep.js').then(dynamic => {
      globalThis.result = [before, [
        ns.__esModule, marker, barrel.__esModule, required.__esModule, required.value,
        dynamic === ns, dynamic.__esModule
      ]];
    });
  `,
  'dep.js': `
    export let __esModule = false;
    export let value = 1;
    export default 'default';
    export function update() { __esModule = 'updated'; value = 2; }
  `,
  'barrel.js': 'export * from "dep.js";',
};

const explicitMarkerResult = [
  [
    false,
    false,
    false,
    true,
    true,
    true,
    false,
    1,
    'default',
    true,
    '[object Module]',
    '[object Object]',
  ],
  ['updated', 'updated', 'updated', true, 2, true, 'updated'],
];

test.each(variants)(
  'preserves an authored __esModule export with treeshake=$treeshake, dev=$devMode, minify=$minify and profilerNames=$profilerNames',
  async ({ treeshake, devMode, minify, profilerNames }) => {
    expect(
      await execute(
        explicitMarkerSources,
        { minify, generatedCode: { profilerNames } },
        devMode,
        treeshake,
      ),
    ).toEqual(explicitMarkerResult);
  },
);

test('matches IIFE namespace and CommonJS marker behavior', async () => {
  expect(await execute(explicitMarkerSources, { format: 'iife' })).toEqual(explicitMarkerResult);
});

test.each(variants)(
  'keeps synthetic markers off ESM namespaces with treeshake=$treeshake, dev=$devMode, minify=$minify and profilerNames=$profilerNames',
  async ({ treeshake, devMode, minify, profilerNames }) => {
    const result = await execute(
      {
        'entry.js': `
          import * as ns from 'dep.js';
          import * as other from 'dep.js';
          const required = require('dep.js');
          globalThis.complete = import('dep.js').then(dynamic => {
            globalThis.result = [
              Object.prototype.hasOwnProperty.call(ns, '__esModule'),
              Object.keys(ns), ns === other, ns === dynamic,
              required !== ns, required.__esModule, required.value,
              Object.keys(required), Object.getOwnPropertyDescriptor(required, '__esModule').enumerable
            ];
          });
        `,
        'dep.js': 'export const value = 42;',
      },
      { minify, generatedCode: { profilerNames } },
      devMode,
      treeshake,
    );
    expect(result).toEqual([false, ['value'], true, true, true, true, 42, ['value'], false]);
  },
);

test.each([false, true])('honors the module.exports ESM export with minify=%s', async (minify) => {
  const result = await execute(
    {
      'entry.js': `
        import * as ns from 'dep.js';
        const required = require('dep.js');
        globalThis.result = [required === ns['module.exports'], required.value, ns.__esModule, ns.default];
      `,
      'dep.js': `
        const replacement = { value: 42 };
        export { replacement as 'module.exports' };
        export const __esModule = false;
        export default 'default';
      `,
    },
    { minify },
  );
  expect(result).toEqual([true, 42, false, 'default']);
});

test.each([false, true])(
  'keeps require facades live through ESM cycles with minify=%s',
  async (minify) => {
    const result = await execute(
      {
        'entry.js': `
        import * as ns from 'dep.js';
        const before = ns.read();
        ns.update();
        globalThis.result = [ns.__esModule, before, ns.read()];
      `,
        'dep.js': `
        export const __esModule = false;
        export let value = 1;
        const reader = require('reader.cjs');
        export function read() { return reader.read(); }
        export function update() { value = 2; }
      `,
        'reader.cjs': `
        const required = require('dep.js');
        exports.read = () => [required.__esModule, required.value];
      `,
      },
      { minify },
    );
    expect(result).toEqual([false, [true, 1], [true, 2]]);
  },
);

test.each([false, true])(
  'does not capture source __toCommonJS bindings with minify=%s',
  async (minify) => {
    const result = await execute(
      {
        'entry.js': `
        const __toCommonJS = () => 'outer';
        const __toCommonJS$1 = () => 'reserved';
        function read() {
          const __toCommonJS = () => 'inner';
          const required = require('dep.js');
          return [__toCommonJS(), required.value, required.__esModule];
        }
        globalThis.result = [__toCommonJS(), __toCommonJS$1(), typeof __toCommonJS$2, read()];
      `,
        'dep.js': 'export const value = 42;',
      },
      { minify },
    );
    expect(result).toEqual(['outer', 'reserved', 'undefined', ['inner', 42, true]]);
  },
);

test('calls the existing __toCommonJS helper without a Rollipop adapter', async () => {
  const code = await generate({
    'entry.js': 'globalThis.result = require("dep.js").value;',
    'dep.js': 'export const value = 42;',
  });
  expect(code).toContain('__toCommonJS(__rollipop_require__(');
  expect(code).not.toContain('__rollipop_require__.n');
  const context = { result: undefined };
  runInNewContext(code, context);
  expect(context.result).toBe(42);
});

test.each([false, true])(
  'preserves exported and destructured __toCommonJS bindings with minify=%s',
  async (minify) => {
    const result = await execute(
      {
        'entry.js': `
          import * as consumer from 'consumer.js';
          globalThis.result = [consumer.__toCommonJS(), consumer.read({ __toCommonJS: () => 'parameter' })];
        `,
        'consumer.js': `
          export const __toCommonJS = () => 'exported';
          export function read({ __toCommonJS }) {
            const required = require('dep.js');
            return [__toCommonJS(), required.value, required.__esModule];
          }
        `,
        'dep.js': 'export const value = 42;',
      },
      { minify },
    );
    expect(result).toEqual(['exported', ['parameter', 42, true]]);
  },
);

test('executes unused require(ESM) without creating a CommonJS facade', async () => {
  const code = await generate({
    'entry.js': 'require("dep.js"); globalThis.result = globalThis.runs;',
    'dep.js': 'globalThis.runs++; export const value = 42;',
  });
  const context = { result: undefined, runs: 0 };
  runInNewContext(code, context);
  expect(context.result).toBe(1);
  expect(code).not.toContain('__toCommonJS(__rollipop_require__(');
});
