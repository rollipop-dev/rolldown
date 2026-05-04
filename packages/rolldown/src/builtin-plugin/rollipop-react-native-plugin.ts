import { BuiltinPlugin } from './utils';

export type RollipopReactNativeRuntimeTarget = 'Hermes' | 'HermesV1';

/**
 * `react-native-worklets` transform configuration. Field semantics mirror
 * the upstream Babel plugin's `WorkletsOptions`, minus the rolldown-managed
 * fields (`filename`, `cwd`). Pass `worklets: {}` to opt in with defaults;
 * omit the field entirely to skip the visitor.
 */
export interface RollipopReactNativeWorkletsConfig {
  /** Identifiers treated as globals — never captured into worklet closures. */
  globals?: string[];
  /** When `true`, only the names listed in `globals` are considered safe. */
  strictGlobal?: boolean;
  /** Omit native-only data (`init_data`) from the output. Useful for web builds. */
  omitNativeOnlyData?: boolean;
  /** Disable source map generation for worklets. */
  disableSourceMaps?: boolean;
  /** Use paths relative to `cwd` for source locations. */
  relativeSourceLocation?: boolean;
  /** Disable Worklet Classes support. */
  disableWorkletClasses?: boolean;
  /** Suppress the inline-shared-values warning. */
  disableInlineStylesWarning?: boolean;
  /** Enable Bundle Mode. */
  bundleMode?: boolean;
  /** Release builds skip debug info such as stack details, version, and location. */
  isRelease?: boolean;
  /** Version string emitted as `__pluginVersion` (the installed `react-native-worklets` version). */
  pluginVersion?: string;
}

/**
 * Flow handling configuration. Mirrors Babel's
 * `@babel/plugin-transform-flow-strip-types` semantics.
 */
export interface RollipopReactNativeFlowConfig {
  /**
   * When `true`, only files containing `@flow` or `@noflow` directive comments are parsed as Flow (Babel `requireDirective: true`).
   * When `false` (default), every JS module is parsed as Flow regardless of directive — matches Metro / Babel default behavior.
   */
  requireDirective?: boolean;
}

export interface RollipopReactNativePluginConfig {
  runtimeTarget?: RollipopReactNativeRuntimeTarget;
  /**
   * The name of the `env` to use when loading configs and plugins. Defaults
   * to the value of `SWC_ENV`, or else `NODE_ENV`, or else `"development"`.
   */
  envName?: string;
  /** `react-native-worklets` transform. Visitor is skipped when omitted. */
  worklets?: RollipopReactNativeWorkletsConfig;
  /** SWC plugins to load. */
  plugins?: [string, Record<string, unknown>][];
  /** Flow handling configuration. Defaults match Metro / Babel behavior. */
  flow?: RollipopReactNativeFlowConfig;
}

export function rollipopReactNativePlugin(config?: RollipopReactNativePluginConfig): BuiltinPlugin {
  const plugins = config?.plugins?.map(([path, pluginConfig]) => ({
    path,
    config: JSON.stringify(pluginConfig ?? {}),
  }));
  return new BuiltinPlugin('builtin:rollipop-react-native', {
    runtimeTarget: config?.runtimeTarget,
    envName: config?.envName,
    flow: config?.flow,
    worklets: config?.worklets,
    plugins,
  });
}
