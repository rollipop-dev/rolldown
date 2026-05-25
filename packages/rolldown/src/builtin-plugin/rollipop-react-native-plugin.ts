import { BuiltinPlugin } from './utils';

export type RollipopReactNativeRuntimeTarget = 'Hermes' | 'HermesV1';
export type RollipopReactNativeModuleType = 'unambiguous' | 'commonjs';

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

/**
 * React (JSX) transform configuration. Mirrors Babel's
 * `@babel/plugin-transform-react-jsx`, minus dev-server-only knobs
 * (fast refresh) — those belong to a bundler, not a precompile/test path.
 */
export interface RollipopReactNativeReactConfig {
  /**
   * JSX runtime.
   * - `"Preserve"` (default): leave JSX untouched — downstream bundler owns it.
   * - `"Automatic"`: compile to `_jsx` / `_jsxs` / `Fragment` imports from
   *   `react/jsx-runtime` (or `react/jsx-dev-runtime` when `development`).
   * - `"Classic"`: compile to `React.createElement` calls.
   */
  runtime?: 'Preserve' | 'Automatic' | 'Classic';
  /** Import source for the automatic runtime. Defaults to `"react"`. */
  importSource?: string;
  /** `pragma` for the classic runtime. Defaults to `"React.createElement"`. */
  pragma?: string;
  /** `pragmaFrag` for the classic runtime. Defaults to `"React.Fragment"`. */
  pragmaFrag?: string;
  /** Throw on XML namespace prefixes (e.g. `<svg:path>`). */
  throwIfNamespace?: boolean;
  /**
   * When `true`, emits the development runtime (`__source` / `__self` debug
   * props for automatic, `react/jsx-dev-runtime` import).
   */
  development?: boolean;
}

export interface RollipopReactNativeModuleConfig {
  /**
   * Module transform type.
   * - `"unambiguous"` (default): preserve the input module shape.
   * - `"commonjs"`: transform ESM syntax to CommonJS.
   */
  type?: RollipopReactNativeModuleType;
}

/**
 * SWC pipeline configuration — wasm plugins, helper emission, React transform.
 */
export interface RollipopReactNativeSwcConfig {
  /** SWC `.wasm` plugins to load. Each entry is `[pluginPath, pluginConfig]`. */
  plugins?: [string, Record<string, unknown>][];
  /**
   * When `true`, runtime helpers are emitted as imports of `@swc/helpers` so
   * a downstream bundler can deduplicate them. When `false` (default), helpers
   * are inlined into each transformed file — preferred when feeding the
   * output to a runtime (e.g. jest) without a bundle step in between.
   */
  externalHelpers?: boolean;
  /** React (JSX) transform configuration. Skipped when `runtime` is `"Preserve"`. */
  react?: RollipopReactNativeReactConfig;
  /** Module transform configuration. Defaults to `type: "unambiguous"`. */
  module?: RollipopReactNativeModuleConfig;
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
  /** Flow handling configuration. Defaults match Metro / Babel behavior. */
  flow?: RollipopReactNativeFlowConfig;
  /** SWC pipeline configuration. */
  swc?: RollipopReactNativeSwcConfig;
}

function lowerSwc(swc: RollipopReactNativeSwcConfig | undefined) {
  if (swc == null) return undefined;
  return {
    plugins: swc.plugins?.map(([path, pluginConfig]) => ({
      path,
      config: JSON.stringify(pluginConfig ?? {}),
    })),
    externalHelpers: swc.externalHelpers,
    react: swc.react,
    module: swc.module,
  };
}

export function rollipopReactNativePlugin(config?: RollipopReactNativePluginConfig): BuiltinPlugin {
  return new BuiltinPlugin('builtin:rollipop-react-native', {
    runtimeTarget: config?.runtimeTarget,
    envName: config?.envName,
    flow: config?.flow,
    worklets: config?.worklets,
    swc: lowerSwc(config?.swc),
  });
}
