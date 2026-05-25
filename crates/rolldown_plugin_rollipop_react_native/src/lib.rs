//! Rolldown plugin wrapper around [`rollipop_react_native_transform::Transformer`].
//!
//! All SWC work lives in the transform crate. This file only translates
//! between rolldown's `HookTransformArgs` / `HookTransformOutput` and the
//! transformer's input/output types, and owns the construction step that
//! preloads the SWC wasm plugins.

use std::borrow::Cow;

use rolldown_common::ModuleType;
use rolldown_plugin::{
  HookTransformArgs, HookTransformOutput, HookTransformReturn, HookUsage, Plugin,
  SharedTransformPluginContext,
};
use rolldown_sourcemap::SourceMap;
use rollipop_react_native_transform::{
  ModuleKind, TransformInput, Transformer, TransformerOptions,
};

pub use rollipop_react_native_transform::{
  FlowConfig, ModuleConfig, ReactConfig, ReactRuntime, RuntimeTarget, SwcConfig, SwcModuleType,
  SwcWasmPlugin, WorkletsConfig,
};

pub struct RollipopReactNativePlugin {
  transformer: Transformer,
}

impl RollipopReactNativePlugin {
  pub fn new(
    env_name: Option<String>,
    runtime_target: RuntimeTarget,
    worklets: Option<WorkletsConfig>,
    flow: Option<FlowConfig>,
    swc: Option<SwcConfig>,
  ) -> Result<Self, anyhow::Error> {
    let options = TransformerOptions { runtime_target, worklets, flow, swc };
    Ok(Self { transformer: Transformer::new(env_name, options)? })
  }
}

impl std::fmt::Debug for RollipopReactNativePlugin {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RollipopReactNativePlugin").field("transformer", &self.transformer).finish()
  }
}

fn module_type_to_kind(module_type: &ModuleType) -> Option<ModuleKind> {
  match module_type {
    ModuleType::Js => Some(ModuleKind::Js),
    ModuleType::Jsx => Some(ModuleKind::Jsx),
    ModuleType::Ts => Some(ModuleKind::Ts),
    ModuleType::Tsx => Some(ModuleKind::Tsx),
    _ => None,
  }
}

fn kind_to_module_type(kind: ModuleKind) -> ModuleType {
  match kind {
    ModuleKind::Js => ModuleType::Js,
    ModuleKind::Jsx => ModuleType::Jsx,
    ModuleKind::Ts => ModuleType::Ts,
    ModuleKind::Tsx => ModuleType::Tsx,
  }
}

impl Plugin for RollipopReactNativePlugin {
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed("builtin:rollipop-react-native")
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::Transform
  }

  async fn transform(
    &self,
    _ctx: SharedTransformPluginContext,
    args: &HookTransformArgs<'_>,
  ) -> HookTransformReturn {
    let Some(module_kind) = module_type_to_kind(args.module_type) else {
      return Ok(None);
    };

    let output = self.transformer.transform(TransformInput {
      filename: args.id,
      code: args.code.as_str(),
      module_kind: Some(module_kind),
    })?;

    let map = SourceMap::from_json_string(&output.map_json)
      .map_err(|e| anyhow::anyhow!("Failed to parse source map: {e}"))?;

    Ok(Some(HookTransformOutput {
      code: Some(output.code),
      map: Some(map),
      module_type: output.output_module_kind.map(kind_to_module_type),
      ..Default::default()
    }))
  }

  fn transform_meta(&self) -> Option<rolldown_plugin::PluginHookMeta> {
    Some(rolldown_plugin::PluginHookMeta { order: Some(rolldown_plugin::PluginOrder::Pre) })
  }
}
