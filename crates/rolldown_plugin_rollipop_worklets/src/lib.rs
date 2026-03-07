use std::borrow::Cow;

use oxc::{
  allocator::Allocator,
  codegen::{Codegen, CodegenOptions, CodegenReturn},
  diagnostics::Severity,
  parser::Parser,
  span::SourceType,
};
use oxc_react_native_worklets::{WorkletsOptions, WorkletsVisitor, may_contain_worklets};
use rolldown_common::ModuleType;
use rolldown_plugin::{
  HookTransformArgs, HookTransformOutput, HookTransformReturn, HookUsage, Plugin, PluginHookMeta,
  PluginOrder, SharedTransformPluginContext,
};

#[derive(Debug)]
pub struct RollipopWorkletsPlugin {
  pub root: String,
  pub plugin_version: String,
  pub is_release: bool,
}

impl Plugin for RollipopWorkletsPlugin {
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed("builtin:rollipop-worklets")
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::Transform
  }

  async fn transform(
    &self,
    _ctx: SharedTransformPluginContext,
    args: &HookTransformArgs<'_>,
  ) -> HookTransformReturn {
    if !may_contain_worklets(args.code) {
      return Ok(None);
    }

    let allocator = Allocator::default();
    let source_type = match args.module_type {
      ModuleType::Js => SourceType::mjs(),
      ModuleType::Jsx => SourceType::jsx(),
      ModuleType::Ts => SourceType::ts(),
      ModuleType::Tsx => SourceType::tsx(),
      _ => unreachable!(),
    };

    let mut parser_ret = Parser::new(&allocator, args.code, source_type).parse();
    if parser_ret.panicked
      && let Some(err) = parser_ret.errors.iter().find(|e| e.severity == Severity::Error)
    {
      return Err(anyhow::anyhow!(format!(
        "Failed to parse code in '{}': {:?}",
        args.id, err.message
      )));
    }

    let mut visitor = WorkletsVisitor::new(
      &allocator,
      WorkletsOptions {
        cwd: Some(self.root.clone()),
        filename: Some(args.id.to_string()),
        plugin_version: self.plugin_version.clone(),
        is_release: self.is_release,
        ..Default::default()
      },
    );

    visitor
      .visit_program(&mut parser_ret.program)
      .map_err(|e| anyhow::anyhow!("Failed to transform worklets: {e}"))?;

    let CodegenReturn { code, map, .. } = Codegen::new()
      .with_options(CodegenOptions {
        source_map_path: Some(args.id.into()),
        ..CodegenOptions::default()
      })
      .build(&parser_ret.program);

    Ok(Some(HookTransformOutput { map, code: Some(code), ..Default::default() }))
  }

  fn transform_meta(&self) -> Option<PluginHookMeta> {
    Some(PluginHookMeta { order: Some(PluginOrder::Pre) })
  }
}
