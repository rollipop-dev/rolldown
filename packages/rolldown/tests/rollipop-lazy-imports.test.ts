import { runInNewContext } from 'node:vm';
import { rolldown, type OutputOptions } from '@rollipop/rolldown';
import { expect, test } from 'vitest';

const sources: Record<string, string> = {
  'entry.js': `
    import { registry } from 'registry.js';
    import { pluginName } from 'setup/register.js';
    import { lazyValue } from 'lazy.js';
    const beforeRead = registry.get('plugin');
    const lazyLoadedEarly = globalThis.log.includes('lazy');
    globalThis.result = [beforeRead, lazyLoadedEarly, pluginName, lazyValue];
  `,
  'registry.js': 'export const registry = new Map();',
  'setup/register.js': `
    import { registry } from 'registry.js';
    registry.set('plugin', 'registered');
    export const pluginName = 'plugin';
  `,
  'lazy.js': `globalThis.log.push('lazy'); export const lazyValue = 'lazy-value';`,
};

async function run(outputOptions: OutputOptions) {
  const bundle = await rolldown({
    input: 'entry.js',
    treeshake: false,
    plugins: [{ name: 'fixture', resolveId: (id) => id, load: (id) => sources[id] }],
  });
  try {
    const { output } = await bundle.generate({
      format: 'rollipop',
      codeSplitting: false,
      ...outputOptions,
    });
    const code = output[0].code;
    const context: { log: string[]; result?: unknown } = { log: [] };
    runInNewContext(code, context);
    return { code, result: context.result };
  } finally {
    await bundle.close();
  }
}

test('lazyImports defers modules until a binding is read', async () => {
  const { code, result } = await run({ lazyImports: true });
  expect(code).toContain('__rollipop_lazy__(');
  // The registering module has not run when the registry is read first.
  expect(result).toEqual([undefined, false, 'plugin', 'lazy-value']);
});

test.each([
  ['RegExp', { exclude: /(^|[\\/])setup[\\/]/ }],
  ['RegExp array', { exclude: [/register\.js$/] }],
  ['glob', { exclude: '**/setup/**' }],
])('lazyImports.exclude keeps matched modules eager (%s)', async (_, lazyImports) => {
  const { result } = await run({ lazyImports });
  expect(result).toEqual(['registered', false, 'plugin', 'lazy-value']);
});

test('lazyImports is off by default and with false', async () => {
  for (const lazyImports of [undefined, false]) {
    const { code, result } = await run({ lazyImports });
    expect(code).not.toContain('__rollipop_lazy__');
    expect(result).toEqual(['registered', true, 'plugin', 'lazy-value']);
  }
});
