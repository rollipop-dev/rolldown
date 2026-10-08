import { runInNewContext } from 'node:vm';
import { rolldown } from '@rollipop/rolldown';
import { expect, test } from 'vitest';

async function generate(sources: Record<string, string>) {
  const bundle = await rolldown({
    input: 'entry.js',
    treeshake: false,
    plugins: [{ name: 'fixture', resolveId: (id) => id, load: (id) => sources[id] }],
  });
  try {
    const { output } = await bundle.generate({ format: 'rollipop', codeSplitting: false });
    return output.find((item) => item.type === 'chunk')!.code;
  } finally {
    await bundle.close();
  }
}

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

test.each(['check()', 'check`value`'])(
  'supports arbitrary import names without binding a namespace as this: %s',
  async (call) => {
    const code = await generate({
      'entry.js': `import { 'a-b' as check } from 'barrel.js'; globalThis.result = ${call};`,
      'barrel.js': `export { 'a-b' } from 'dep.js';`,
      'dep.js': `'use strict'; function check() { return this === undefined; } export { check as 'a-b' };`,
    });
    const context = { result: undefined };
    runInNewContext(code, context);
    expect(context.result).toBe(true);
  },
);
