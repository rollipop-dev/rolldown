use std::collections::HashMap;

use rolldown_plugin_rollipop_react_native::{
  FlowConfig, ModuleConfig, ReactConfig, ReactRuntime, RollipopReactNativePlugin, RuntimeTarget,
  SwcConfig, SwcModuleType, SwcWasmPlugin, WorkletsConfig,
};
use rollipop_react_native_transform::TransformerOptions;
use rustc_hash::FxBuildHasher;

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativePluginConfig {
  /// The name of the `env` to use when loading configs and plugins. Defaults
  /// to the value of `SWC_ENV`, or else `NODE_ENV`, or else `"development"`.
  pub env_name: Option<String>,
  /// Selects the compat-pass preset. Defaults to `"HermesV1"` when omitted.
  /// `"Hermes"` adds `transform-classes` and `transform-async-to-generator`
  /// on top of the V1 baseline for older Hermes / JSC fallbacks.
  pub runtime_target: Option<BindingRollipopReactNativeRuntimeTarget>,
  /// Flow handling configuration. Mirrors Babel's
  /// `@babel/plugin-transform-flow-strip-types` semantics. When omitted,
  /// every JS module is parsed as Flow (Metro / Babel default).
  pub flow: Option<BindingRollipopReactNativeFlowConfig>,
  /// `react-native-worklets` transform configuration. Visitor is skipped
  /// entirely when omitted.
  pub worklets: Option<BindingRollipopReactNativeWorkletsConfig>,
  /// SWC pipeline configuration — wasm plugins and helper emission.
  pub swc: Option<BindingRollipopReactNativeSwcConfig>,
}

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeSwcConfig {
  /// SWC `.wasm` plugins to load.
  pub plugins: Option<Vec<BindingRollipopReactNativeSwcPlugin>>,
  /// When `true`, runtime helpers are emitted as imports of `@swc/helpers`
  /// so a downstream bundler can deduplicate them. When `false` (default),
  /// helpers are inlined into each transformed file.
  pub external_helpers: Option<bool>,
  /// React (JSX) transform configuration. Skipped entirely when `runtime`
  /// is `"Preserve"` (the default).
  pub react: Option<BindingRollipopReactNativeReactConfig>,
  /// Module transform configuration. Defaults to `type: "unambiguous"`.
  pub module: Option<BindingRollipopReactNativeModuleConfig>,
  /// Global expression replacements, matching SWC's
  /// `jsc.transform.optimizer.globals.vars` behavior.
  pub globals: Option<HashMap<String, String, FxBuildHasher>>,
}

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeSwcPlugin {
  pub path: String,
  /// JSON-serialized plugin config
  pub config: String,
}

#[napi_derive::napi(string_enum)]
#[derive(Debug)]
pub enum BindingRollipopReactNativeReactRuntime {
  Preserve,
  Automatic,
  Classic,
}

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeModuleConfig {
  /// Module transform type. `"unambiguous"` preserves the input module shape;
  /// `"commonjs"` matches SWC's `module: { type: "commonjs" }` defaults.
  pub r#type: Option<BindingRollipopReactNativeModuleType>,
}

#[napi_derive::napi(string_enum)]
#[derive(Debug)]
pub enum BindingRollipopReactNativeModuleType {
  #[napi(value = "unambiguous")]
  Unambiguous,
  #[napi(value = "commonjs")]
  CommonJs,
}

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeReactConfig {
  /// JSX runtime. Defaults to `"Preserve"` (no transform — bundler owns JSX).
  pub runtime: Option<BindingRollipopReactNativeReactRuntime>,
  /// Import source for the automatic runtime. Defaults to `"react"`.
  pub import_source: Option<String>,
  /// `pragma` for the classic runtime. Defaults to `"React.createElement"`.
  pub pragma: Option<String>,
  /// `pragmaFrag` for the classic runtime. Defaults to `"React.Fragment"`.
  pub pragma_frag: Option<String>,
  /// Throw when an XML namespace prefix is encountered (e.g. `<svg:path>`).
  pub throw_if_namespace: Option<bool>,
  /// When `true`, emits the development runtime (`__source` / `__self` debug
  /// props for automatic, `react/jsx-dev-runtime` import).
  pub development: Option<bool>,
}

#[napi_derive::napi(string_enum)]
#[derive(Debug)]
pub enum BindingRollipopReactNativeRuntimeTarget {
  Hermes,
  HermesV1,
}

/// Mirrors `FlowConfig` from `rolldown_plugin_rollipop_react_native`.
#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeFlowConfig {
  /// When `true`, only files containing `@flow` or `@noflow` directive
  /// comments are parsed as Flow (Babel `requireDirective: true`). When
  /// `false` (default), every JS module is parsed as Flow regardless of
  /// directive — matches Metro / Babel default behavior.
  pub require_directive: Option<bool>,
}

/// Mirrors `WorkletsOptions` from `swc_react_native::worklets::options`,
/// minus the rolldown-managed fields (`filename`, `cwd`).
#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeWorkletsConfig {
  /// Identifiers treated as globals — never captured into worklet closures.
  pub globals: Option<Vec<String>>,
  /// When `true`, only the names listed in `globals` are considered safe.
  pub strict_global: Option<bool>,
  /// Omit native-only data (`init_data`) from the output. Useful for web
  /// builds.
  pub omit_native_only_data: Option<bool>,
  /// Disable source map generation for worklets.
  pub disable_source_maps: Option<bool>,
  /// Use paths relative to `cwd` for source locations.
  pub relative_source_location: Option<bool>,
  /// Disable Worklet Classes support.
  pub disable_worklet_classes: Option<bool>,
  /// Suppress the inline-shared-values warning.
  pub disable_inline_styles_warning: Option<bool>,
  /// Enable Bundle Mode.
  pub bundle_mode: Option<bool>,
  /// Release builds skip debug info such as stack details, version, and
  /// location.
  pub is_release: Option<bool>,
  /// Version string emitted as `__pluginVersion`. Callers should supply the
  /// installed `react-native-worklets` package version.
  pub plugin_version: Option<String>,
}

impl From<BindingRollipopReactNativeRuntimeTarget> for RuntimeTarget {
  fn from(value: BindingRollipopReactNativeRuntimeTarget) -> Self {
    match value {
      BindingRollipopReactNativeRuntimeTarget::Hermes => Self::Hermes,
      BindingRollipopReactNativeRuntimeTarget::HermesV1 => Self::HermesV1,
    }
  }
}

impl From<BindingRollipopReactNativeFlowConfig> for FlowConfig {
  fn from(value: BindingRollipopReactNativeFlowConfig) -> Self {
    Self { require_directive: value.require_directive.unwrap_or(false) }
  }
}

impl From<BindingRollipopReactNativeWorkletsConfig> for WorkletsConfig {
  fn from(value: BindingRollipopReactNativeWorkletsConfig) -> Self {
    let mut config = WorkletsConfig::default();
    let opts = &mut config.options;
    if let Some(globals) = value.globals {
      opts.globals = globals;
    }
    if let Some(v) = value.strict_global {
      opts.strict_global = v;
    }
    if let Some(v) = value.omit_native_only_data {
      opts.omit_native_only_data = v;
    }
    if let Some(v) = value.disable_source_maps {
      opts.disable_source_maps = v;
    }
    if let Some(v) = value.relative_source_location {
      opts.relative_source_location = v;
    }
    if let Some(v) = value.disable_worklet_classes {
      opts.disable_worklet_classes = v;
    }
    if let Some(v) = value.disable_inline_styles_warning {
      opts.disable_inline_styles_warning = v;
    }
    if let Some(v) = value.bundle_mode {
      opts.bundle_mode = v;
    }
    if let Some(v) = value.is_release {
      opts.is_release = v;
    }
    if let Some(v) = value.plugin_version {
      opts.plugin_version = v;
    }
    config
  }
}

impl From<BindingRollipopReactNativeReactRuntime> for ReactRuntime {
  fn from(value: BindingRollipopReactNativeReactRuntime) -> Self {
    match value {
      BindingRollipopReactNativeReactRuntime::Preserve => Self::Preserve,
      BindingRollipopReactNativeReactRuntime::Automatic => Self::Automatic,
      BindingRollipopReactNativeReactRuntime::Classic => Self::Classic,
    }
  }
}

impl From<BindingRollipopReactNativeModuleType> for SwcModuleType {
  fn from(value: BindingRollipopReactNativeModuleType) -> Self {
    match value {
      BindingRollipopReactNativeModuleType::Unambiguous => Self::Unambiguous,
      BindingRollipopReactNativeModuleType::CommonJs => Self::CommonJs,
    }
  }
}

impl From<BindingRollipopReactNativeModuleConfig> for ModuleConfig {
  fn from(value: BindingRollipopReactNativeModuleConfig) -> Self {
    ModuleConfig { r#type: value.r#type.map(SwcModuleType::from).unwrap_or_default() }
  }
}

impl From<BindingRollipopReactNativeReactConfig> for ReactConfig {
  fn from(value: BindingRollipopReactNativeReactConfig) -> Self {
    ReactConfig {
      runtime: value.runtime.map(ReactRuntime::from).unwrap_or_default(),
      import_source: value.import_source,
      pragma: value.pragma,
      pragma_frag: value.pragma_frag,
      throw_if_namespace: value.throw_if_namespace,
      development: value.development.unwrap_or(false),
    }
  }
}

impl TryFrom<BindingRollipopReactNativeSwcConfig> for SwcConfig {
  type Error = anyhow::Error;

  fn try_from(value: BindingRollipopReactNativeSwcConfig) -> Result<Self, Self::Error> {
    let plugins = value
      .plugins
      .unwrap_or_default()
      .into_iter()
      .map(|p| -> Result<SwcWasmPlugin, anyhow::Error> {
        let config = serde_json::from_str(&p.config)
          .map_err(|e| anyhow::anyhow!("Failed to parse plugin config for '{}': {e}", p.path))?;
        Ok(SwcWasmPlugin { path: p.path, config })
      })
      .collect::<Result<Vec<_>, _>>()?;

    Ok(SwcConfig {
      plugins,
      external_helpers: value.external_helpers.unwrap_or(false),
      react: value.react.map(ReactConfig::from).unwrap_or_default(),
      module: value.module.map(ModuleConfig::from),
      globals: value.globals.unwrap_or_default(),
    })
  }
}

impl BindingRollipopReactNativePluginConfig {
  /// Lower the binding config into the parts needed to construct either the
  /// rolldown plugin or the standalone transformer. Centralizing this here
  /// keeps the two call sites in sync.
  pub fn into_parts(self) -> Result<(Option<String>, TransformerOptions), anyhow::Error> {
    let swc = self.swc.map(SwcConfig::try_from).transpose()?;
    let runtime_target = self.runtime_target.map(RuntimeTarget::from).unwrap_or_default();
    let worklets = self.worklets.map(WorkletsConfig::from);
    let flow = self.flow.map(FlowConfig::from);
    Ok((self.env_name, TransformerOptions { runtime_target, worklets, flow, swc }))
  }
}

impl TryFrom<BindingRollipopReactNativePluginConfig> for RollipopReactNativePlugin {
  type Error = anyhow::Error;

  fn try_from(value: BindingRollipopReactNativePluginConfig) -> Result<Self, Self::Error> {
    let (env_name, options) = value.into_parts()?;
    RollipopReactNativePlugin::new(
      env_name,
      options.runtime_target,
      options.worklets,
      options.flow,
      options.swc,
    )
  }
}
