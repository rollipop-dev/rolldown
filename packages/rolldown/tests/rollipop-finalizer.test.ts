import { runInNewContext } from 'node:vm';
import {
  rolldown,
  type InputOptions,
  type OutputOptions,
  type SourceDescription,
} from '@rollipop/rolldown';
import { dev } from '@rollipop/rolldown/experimental';
import { expect, test } from 'vitest';

async function generate(
  sources: Record<string, string | SourceDescription>,
  outputOptions: OutputOptions = {},
  inputOptions: InputOptions = {},
) {
  const bundle = await rolldown({
    input: 'entry.js',
    treeshake: false,
    ...inputOptions,
    plugins: [{ name: 'fixture', resolveId: (id) => id, load: (id) => sources[id] }],
  });
  try {
    const { output } = await bundle.generate({
      format: 'rollipop',
      codeSplitting: false,
      ...outputOptions,
    });
    return output.find((item) => item.type === 'chunk')!.code;
  } finally {
    await bundle.close();
  }
}

function expectAsciiCode(code: string) {
  for (const line of code.split('\n')) {
    if (line.trimStart().startsWith('//')) continue;
    for (let index = 0; index < line.length; index++) {
      expect(line.charCodeAt(index)).toBeLessThanOrEqual(0x7f);
    }
  }
}

test.each([
  { charset: 'ascii', profilerNames: false },
  { charset: 'ascii', profilerNames: true },
  { charset: 'utf8', profilerNames: false },
  { charset: 'utf8', profilerNames: true },
] as const)(
  'respects output.charset: $charset in Rollipop factories with profilerNames=$profilerNames',
  async ({ charset, profilerNames }) => {
    const value = '\u4f60\u597d \u{1f600}';
    const code = await generate(
      {
        'entry.js': `import { 'a-b' as value } from 'dep-\u03c0-\u{1f600}.js'; globalThis.result = value;`,
        'dep-\u03c0-\u{1f600}.js': `const \u03c0 = ${JSON.stringify(value)}; export { \u03c0 as 'a-b' };`,
      },
      { charset, minify: false, generatedCode: { profilerNames } },
    );
    if (charset === 'ascii') {
      expectAsciiCode(code);
    } else {
      expect(code).toContain(value);
    }
    const context = { result: undefined };
    runInNewContext(code, context);
    expect(context.result).toBe(value);
  },
);

test.each(['ascii', 'utf8'] as const)(
  'respects output.charset: %s in Rollipop dev graph metadata',
  async (charset) => {
    const sources: Record<string, string> = {
      '\0entry.js': 'import { value } from "dep-\u03c0.js"; globalThis.result = value;',
      '\0dep-\u03c0.js': 'export const value = 42;',
    };
    let code: string | undefined;
    const engine = await dev(
      {
        input: 'entry.js',
        treeshake: false,
        experimental: { devMode: true },
        plugins: [{ name: 'fixture', resolveId: (id) => `\0${id}`, load: (id) => sources[id] }],
      },
      { format: 'rollipop', charset, minify: false, generatedCode: { profilerNames: true } },
      {
        watch: { enabled: false, skipWrite: true },
        onOutput(output) {
          if (output instanceof Error) throw output;
          code = output.output.find((item) => item.type === 'chunk')!.code;
        },
      },
    );
    try {
      await engine.run();
      await engine.ensureCurrentBuildFinish();
      const graph = code!.split('\n').find((line) => line.includes('.registerGraph('));
      expect(graph).toBeDefined();
      if (charset === 'ascii') {
        expectAsciiCode(graph!);
      } else {
        expect(graph).toContain('dep-\u03c0.js');
      }
      let entries: string[] | undefined;
      runInNewContext(graph!, {
        __rolldown_runtime__: {
          registerGraph(delta: { ids: string[]; entries?: number[] }) {
            entries = delta.entries?.map((index) => delta.ids[index]);
          },
        },
      });
      expect(entries).toEqual(['\\0entry.js']);
    } finally {
      await engine.close();
    }
  },
);

test.each([
  'import { x } from "dep.js"; globalThis.requested = x;',
  'import { x } from "dep.js";',
  'import * as dep from "dep.js"; globalThis.requested = dep.x;',
  'export { x as value } from "dep.js";',
])('exposes shimmed exports on their original namespace: %s', async (request) => {
  const warnings: string[] = [];
  const code = await generate(
    {
      'entry.js': `${request}
        import * as ns from 'dep.js';
        import * as barrel from 'barrel.js';
        globalThis.result = [Object.keys(ns).sort(), Object.keys(barrel).sort(), ns.x, 'x' in ns];`,
      'dep.js': 'export const value = 1;',
      'barrel.js': 'export * from "dep.js";',
    },
    {},
    {
      treeshake: true,
      shimMissingExports: true,
      onwarn: (warning) => warnings.push(warning.code!),
    },
  );
  const context = { result: undefined };
  runInNewContext(code, context);
  expect(context.result).toEqual([['value', 'x'], ['value', 'x'], undefined, true]);
  expect(warnings).toEqual(['SHIMMED_EXPORT']);
});

test('preserves real star exports beside shimmed exports', async () => {
  const code = await generate(
    {
      'entry.js': `import { missing } from 'empty.js';
        import * as empty from 'empty.js';
        import * as barrel from 'barrel.js';
        globalThis.result = [missing, Object.keys(empty), Object.keys(barrel), barrel.missing];`,
      'empty.js': 'export {};',
      'real.js': 'export const missing = 42;',
      'barrel.js': 'export * from "empty.js"; export * from "real.js";',
    },
    {},
    { treeshake: true, shimMissingExports: true, onwarn: () => {} },
  );
  const context = { result: undefined };
  runInNewContext(code, context);
  expect(context.result).toEqual([undefined, ['missing'], ['missing'], 42]);
});

test('exposes implicit empty-module shims through star re-exports without warnings', async () => {
  const warnings: string[] = [];
  const code = await generate(
    {
      'entry.js': `import { missing } from 'empty.js';
        import * as empty from 'empty.js';
        import * as barrel from 'barrel.js';
        globalThis.result = [missing, Object.keys(empty), Object.keys(barrel)];`,
      'empty.js': { code: '', moduleType: 'empty' },
      'barrel.js': 'export * from "empty.js";',
    },
    {},
    { treeshake: true, onwarn: (warning) => warnings.push(warning.code!) },
  );
  const context = { result: undefined };
  runInNewContext(code, context);
  expect(context.result).toEqual([undefined, ['missing'], ['missing']]);
  expect(warnings).toEqual([]);
});

test.each([
  { strictExecutionOrder: false, onDemandWrapping: false },
  { strictExecutionOrder: false, onDemandWrapping: true },
  { strictExecutionOrder: true, onDemandWrapping: false },
  { strictExecutionOrder: true, onDemandWrapping: true },
])(
  'finalizes retained runtime code with strict=$strictExecutionOrder and onDemand=$onDemandWrapping',
  async ({ strictExecutionOrder, onDemandWrapping }) => {
    const code = await generate(
      {
        'entry.js': 'import { check } from "barrel.js"; globalThis.result = check();',
        'barrel.js': 'export { check } from "dep.js";',
        'dep.js': 'export function check() { return 42; }',
      },
      { strictExecutionOrder },
      { treeshake: { moduleSideEffects: false }, experimental: { onDemandWrapping } },
    );
    expect(code).not.toMatch(/^\s*export\s/m);
    const context = { result: undefined };
    runInNewContext(code, context);
    expect(context.result).toBe(42);
  },
);

test.each([
  'exports = { replacement: true };',
  'var exports = { replacement: true };',
  'module.exports = { replacement: true };',
])('keeps CommonJS top-level this bound to the initial exports: %s', async (replacement) => {
  const code = await generate({
    'entry.js': `import check from 'dep.cjs'; globalThis.result = check();`,
    'dep.cjs': `
        'use strict';
        const initial = this;
        ${replacement}
        module.exports = () => [this === initial, this !== exports, this !== module.exports];
      `,
  });
  const context = { result: undefined };
  runInNewContext(code, context);
  expect(context.result).toEqual([true, !replacement.startsWith('module.'), true]);
});

test('keeps generated import bindings off nested source bindings', async () => {
  const sources = {
    'entry.js': `import { value } from 'dep.js'; globalThis.result = value();`,
    'dep.js': `export function value() { return 42; }`,
  };
  const initial = await generate(sources);
  const generatedName = /var (import_dep_\d+_\d+_\d+)/.exec(initial)![1];
  sources['entry.js'] = `
    import { value } from 'dep.js';
    function read() {
      const ${generatedName} = { value: () => 'shadowed' };
      globalThis.authored = ${generatedName}.value();
      return value();
    }
    globalThis.result = read();
  `;
  const context = { result: undefined, authored: undefined };
  runInNewContext(await generate(sources), context);
  expect(context.authored).toBe('shadowed');
  expect(context.result).toBe(42);
});

test('keeps lowered dynamic imports off a source Promise binding', async () => {
  const code = await generate({
    'entry.js': `
      function read(Promise) { return import('dep.js'); }
      globalThis.result = read({ resolve() { throw new Error('shadowed Promise'); } });
    `,
    'dep.js': `export const value = 42;`,
  });
  const context = { result: undefined };
  runInNewContext(code, context);
  await expect(context.result).resolves.toMatchObject({ value: 42 });
});

test.each([
  { call: 'check()', name: 'a-b' },
  { call: 'check`value`', name: 'a-b' },
  { call: 'check()', name: '\u{1f600}' },
  { call: 'check`value`', name: '\u{1f600}' },
])(
  'supports arbitrary import names without binding a namespace as this: $name $call',
  async ({ call, name }) => {
    const quotedName = JSON.stringify(name);
    const code = await generate({
      'entry.js': `import { ${quotedName} as check } from 'barrel.js'; globalThis.result = ${call};`,
      'barrel.js': `export { ${quotedName} } from 'dep.js';`,
      'dep.js': `'use strict'; function check() { return this === undefined; } export { check as ${quotedName} };`,
    });
    const context = { result: undefined };
    runInNewContext(code, context);
    expect(context.result).toBe(true);
  },
);
