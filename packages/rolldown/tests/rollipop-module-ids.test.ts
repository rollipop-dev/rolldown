// See internal-docs/rollipop-module-ids/design.md
import { runInNewContext } from 'node:vm';
import { rolldown, type OutputOptions } from '@rollipop/rolldown';
import { expect, test } from 'vitest';

const MODULE_COUNT = 120;

// A tree of modules, so the loader discovers importees from many modules at once.
function createSources(extraModule = false): Record<string, string> {
  const sources: Record<string, string> = {};
  for (let i = 0; i < MODULE_COUNT; i++) {
    const children = [2 * i + 1, 2 * i + 2].filter((child) => child < MODULE_COUNT);
    sources[`src/m${i}.js`] = [
      ...children.map((child) => `import { value as v${child} } from 'src/m${child}.js';`),
      `export const value = ${[String(i), ...children.map((child) => `v${child}`)].join(' + ')};`,
    ].join('\n');
  }
  if (extraModule) {
    sources['src/m0.js'] =
      `import { extra } from 'src/extra.js';\n${sources['src/m0.js'].replace(/;$/, ' + extra;')}`;
    sources['src/extra.js'] = 'export const extra = 0;';
  }
  // A removed barrel resolves one import record to two modules named `util`.
  sources['src/barrel.js'] =
    `export { a } from 'src/x/util.js'; export { b } from 'src/y/util.js';`;
  sources['src/x/util.js'] = 'export const a = 1;';
  sources['src/y/util.js'] = 'export const b = 2;';
  sources['entry.js'] = `
    import { value } from 'src/m0.js';
    import { a, b } from 'src/barrel.js';
    globalThis.result = value + a + b;
  `;
  return sources;
}

async function build(sources: Record<string, string>, outputOptions: OutputOptions = {}) {
  const bundle = await rolldown({
    input: 'entry.js',
    treeshake: { moduleSideEffects: (id) => id !== 'src/barrel.js' },
    plugins: [
      {
        name: 'fixture',
        resolveId: (id) => id,
        // Random load latency shuffles loader discovery order, and with it `ModuleIdx`.
        load: async (id) => {
          await new Promise((resolve) => setTimeout(resolve, Math.random() * 5));
          return sources[id];
        },
      },
    ],
  });
  try {
    const { output } = await bundle.generate({
      format: 'rollipop',
      codeSplitting: false,
      minify: false,
      ...outputOptions,
    });
    return output[0].code;
  } finally {
    await bundle.close();
  }
}

// `<id>: function(...) {` followed by the `// <stable id>` region comment.
function moduleIds(code: string) {
  const ids = new Map<string, number>();
  for (const [, id, stableId] of code.matchAll(/^\t\t(\d+): function\(.*\n\s*\/\/ (\S+)$/gm)) {
    ids.set(stableId, Number(id));
  }
  return ids;
}

const expectedResult = (MODULE_COUNT * (MODULE_COUNT - 1)) / 2 + 3;
// `src/m*.js`, the two `util.js` modules and the entry; the barrel is removed.
const expectedModuleCount = MODULE_COUNT + 3;

test('identical inputs produce identical rollipop bundles', async () => {
  const sources = createSources();
  const first = await build(sources);
  const context = { result: undefined };
  runInNewContext(first, context);
  expect(context.result).toBe(expectedResult);
  expect(moduleIds(first).size).toBe(expectedModuleCount);
  // Same-named bindings are told apart by source order, not by `ModuleIdx`.
  expect(first).toMatch(/var import_util_1 = __rollipop_require__\(\d+\);/);
  expect(first).toMatch(/var import_util_1\$1 = __rollipop_require__\(\d+\);/);
  for (let i = 0; i < 4; i++) {
    expect(await build(sources)).toBe(first);
  }
});

test('identical inputs produce identical minified rollipop bundles', async () => {
  const sources = createSources();
  const first = await build(sources, { minify: true });
  for (let i = 0; i < 2; i++) {
    expect(await build(sources, { minify: true })).toBe(first);
  }
});

test('adding a module keeps the ids of the other modules', async () => {
  const before = moduleIds(await build(createSources()));
  const afterCode = await build(createSources(true));
  const after = moduleIds(afterCode);
  const context = { result: undefined };
  runInNewContext(afterCode, context);
  expect(context.result).toBe(expectedResult);

  expect(after.size).toBe(before.size + 1);
  expect(after.has('src/extra.js')).toBe(true);
  for (const [stableId, id] of before) {
    expect(after.get(stableId), stableId).toBe(id);
  }
  // Ids are unique and small enough to stay cheap number literals.
  expect(new Set(after.values()).size).toBe(after.size);
  for (const id of after.values()) expect(id).toBeLessThan(2 ** 24);
});
