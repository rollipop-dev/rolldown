use rolldown_plugin_rollipop_react_native::{
  RollipopReactNativePlugin, RuntimeTarget, SwcWasmPlugin, WorkletsConfig,
};

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativePluginConfig {
  pub plugins: Option<Vec<BindingRollipopReactNativeSwcPlugin>>,
  /// The name of the `env` to use when loading configs and plugins. Defaults
  /// to the value of `SWC_ENV`, or else `NODE_ENV`, or else `"development"`.
  pub env_name: Option<String>,
  /// Selects the compat-pass preset. Defaults to `"HermesV1"` when omitted.
  /// `"Hermes"` adds `transform-classes` and `transform-async-to-generator`
  /// on top of the V1 baseline for older Hermes / JSC fallbacks.
  pub runtime_target: Option<BindingRollipopReactNativeRuntimeTarget>,
  /// `react-native-worklets` transform configuration. Visitor is skipped
  /// entirely when omitted.
  pub worklets: Option<BindingRollipopReactNativeWorkletsConfig>,
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
pub enum BindingRollipopReactNativeRuntimeTarget {
  Hermes,
  HermesV1,
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

impl TryFrom<BindingRollipopReactNativePluginConfig> for RollipopReactNativePlugin {
  type Error = anyhow::Error;

  fn try_from(value: BindingRollipopReactNativePluginConfig) -> Result<Self, Self::Error> {
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
    let runtime_target = value.runtime_target.map(RuntimeTarget::from).unwrap_or_default();
    let worklets = value.worklets.map(WorkletsConfig::from);
    RollipopReactNativePlugin::new(plugins, value.env_name, runtime_target, worklets)
  }
}
