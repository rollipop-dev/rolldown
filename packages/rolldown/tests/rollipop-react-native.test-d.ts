import type { RollipopReactNativeWorkletsConfig } from '@rollipop/rolldown/experimental';
import { expectTypeOf, test } from 'vitest';

test('native worklet configuration requires an explicit runtime', () => {
  expectTypeOf<{ pluginVersion: string }>().toExtend<RollipopReactNativeWorkletsConfig>();
  expectTypeOf<{ reanimatedVersion: string }>().toExtend<RollipopReactNativeWorkletsConfig>();
  expectTypeOf<{}>().not.toExtend<RollipopReactNativeWorkletsConfig>();
  expectTypeOf<{
    pluginVersion: string;
    reanimatedVersion: string;
  }>().not.toExtend<RollipopReactNativeWorkletsConfig>();
});

test('native worklet configuration exposes each runtime option', () => {
  expectTypeOf<{
    pluginVersion: string;
    strictGlobal: boolean;
    bundleMode: boolean;
    globals: string[];
    isRelease: boolean;
  }>().toExtend<RollipopReactNativeWorkletsConfig>();
  expectTypeOf<{
    reanimatedVersion: string;
    processNestedWorklets: boolean;
    globals: string[];
    isRelease: boolean;
  }>().toExtend<RollipopReactNativeWorkletsConfig>();
});
