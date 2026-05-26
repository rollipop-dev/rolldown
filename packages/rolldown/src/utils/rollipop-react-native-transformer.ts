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
 *
 * All options pass through unchanged — `swc.externalHelpers` defaults to
 * `false` (helpers inlined) and `swc.react.runtime` to `"Preserve"`, both
 * matching the underlying Rust defaults. Set them explicitly when needed.
 */
export class RollipopReactNativeTransformer {
  private inner: BindingRollipopReactNativeTransformer;

  constructor(config?: RollipopReactNativeTransformerConfig) {
    const swc = config?.swc;
    const plugins = swc?.plugins?.map(([path, pluginConfig]) => ({
      path,
      config: JSON.stringify(pluginConfig ?? {}),
    }));
    this.inner = new BindingRollipopReactNativeTransformer({
      runtimeTarget: config?.runtimeTarget,
      envName: config?.envName,
      flow: config?.flow,
      worklets: config?.worklets,
      swc: {
        plugins,
        externalHelpers: swc?.externalHelpers,
        react: swc?.react,
        module: swc?.module,
        globals: swc?.globals,
      },
    });
  }

  transform(filename: string, code: string): Promise<RollipopReactNativeTransformResult> {
    return this.inner.transform(filename, code);
  }

  transformSync(filename: string, code: string): RollipopReactNativeTransformResult {
    return this.inner.transformSync(filename, code);
  }
}
