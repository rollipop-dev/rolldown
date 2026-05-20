import {
  BindingRollipopReactNativeTransformer,
  type BindingRollipopReactNativeTransformResult,
} from '../binding.cjs';
import type { RollipopReactNativePluginConfig } from '../builtin-plugin/rollipop-react-native-plugin';

export type RollipopReactNativeTransformerConfig = RollipopReactNativePluginConfig;
export type RollipopReactNativeTransformResult = BindingRollipopReactNativeTransformResult;

/**
 * Standalone React Native SWC transform pipeline.
 *
 * Mirrors the behavior of {@link rollipopReactNativePlugin} but can be
 * invoked directly outside of a rolldown build (e.g. from Metro adapters,
 * test harnesses, or precompile steps). SWC `.wasm` plugins listed in the
 * config are read from disk and compiled exactly once at construction
 * time — call `transform` / `transformSync` repeatedly without reloading.
 */
export class RollipopReactNativeTransformer {
  private inner: BindingRollipopReactNativeTransformer;

  constructor(config?: RollipopReactNativeTransformerConfig) {
    const plugins = config?.plugins?.map(([path, pluginConfig]) => ({
      path,
      config: JSON.stringify(pluginConfig ?? {}),
    }));
    this.inner = new BindingRollipopReactNativeTransformer({
      runtimeTarget: config?.runtimeTarget,
      envName: config?.envName,
      flow: config?.flow,
      worklets: config?.worklets,
      plugins,
    });
  }

  transform(filename: string, code: string): Promise<RollipopReactNativeTransformResult> {
    return this.inner.transform(filename, code);
  }

  transformSync(filename: string, code: string): RollipopReactNativeTransformResult {
    return this.inner.transformSync(filename, code);
  }
}
