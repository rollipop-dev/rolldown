import {
  defineConfig,
  type InputOptions,
  type OutputOptions,
  type Plugin as RolldownRawPlugin,
  type PreRenderedChunk,
  type RenderedChunk,
} from '@rollipop/rolldown';
import { rollipopReactNativePlugin, viteTransformPlugin } from '@rollipop/rolldown/experimental';
import { transformSync } from '@rollipop/rolldown/utils';
import type { Plugin as RollupPlugin } from 'rollup';
import { describe, expectTypeOf, test } from 'vitest';

describe('chunk type compatibility', () => {
  // https://github.com/rolldown/rolldown/issues/10804
  test('`RenderedChunk` is assignable to `PreRenderedChunk`', () => {
    const renderedChunk = undefined as unknown as RenderedChunk;
    expectTypeOf(renderedChunk).toExtend<PreRenderedChunk>();
  });
});

describe('plugin type compatibility', () => {
  type PluginsOption = InputOptions['plugins'];
  type OutputPluginsOption = OutputOptions['plugins'];

  test('can assign rollup plugins to `plugins` option', () => {
    const rollupPluginInstance = undefined as unknown as RollupPlugin;
    expectTypeOf(rollupPluginInstance).toExtend<PluginsOption>();

    // this should not error
    defineConfig({
      plugins: [rollupPluginInstance],
    });
  });

  test('can assign rollup plugins to `output.plugins` option', () => {
    const rollupPluginInstance = undefined as unknown as RollupPlugin;
    expectTypeOf(rollupPluginInstance).toExtend<OutputPluginsOption>();

    // this should not error
    defineConfig({
      output: {
        plugins: [rollupPluginInstance],
      },
    });
  });

  test('input suggestions for hooks works', () => {
    const plugin: PluginsOption = {
      name: 'test',
      // ^ input suggestion should work here
    };
    expectTypeOf(plugin).toEqualTypeOf<RolldownRawPlugin | { name: string }>();

    const buildS = undefined;
    defineConfig({
      plugins: [
        {
          name: 'test',
          // @ts-expect-error -- only known properties should be allowed
          buildS,
          //    ^ input suggestion should work here
        },
      ],
    });
  });
});

describe('transform option types', () => {
  test('accepts React Compiler only in the React Native plugin', () => {
    rollipopReactNativePlugin({
      swc: {
        react: {
          compiler: {
            target: '17',
          },
        },
      },
    });
  });

  test('does not accept React Compiler under `transform.jsx`', () => {
    defineConfig({
      transform: {
        jsx: {
          // @ts-expect-error -- configure the React Native plugin's `swc.react.compiler`
          compiler: {},
        },
      },
    });
  });

  test('does not accept React Compiler directly under `transform`', () => {
    defineConfig({
      transform: {
        // @ts-expect-error -- configure the React Native plugin's `swc.react.compiler`
        reactCompiler: {},
      },
    });
  });

  test('does not accept React Compiler in the generic transform utility', () => {
    transformSync('App.jsx', '', {
      jsx: {
        // @ts-expect-error -- configure the React Native transformer's `swc.react.compiler`
        compiler: {},
      },
    });
  });

  test('does not accept React Compiler in the Vite transform plugin', () => {
    viteTransformPlugin({
      root: process.cwd(),
      transformOptions: {
        jsx: {
          // @ts-expect-error -- configure the React Native plugin's `swc.react.compiler`
          compiler: {},
        },
      },
    });
  });
});
