mod flow;
mod visitors;
#[cfg(feature = "wasm_plugins")]
mod wasm_plugins;

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use rolldown_common::ModuleType;
use rolldown_plugin::{
  HookTransformArgs, HookTransformOutput, HookTransformReturn, HookUsage, Plugin,
  SharedTransformPluginContext,
};
use rolldown_sourcemap::SourceMap;
use swc_common::comments::SingleThreadedComments;
use swc_common::source_map::SourceMapGenConfig;
use swc_common::sync::Lrc;
use swc_common::{FileName, GLOBALS, Globals, Mark};
use swc_ecma_ast::{EsVersion, Pass};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::{Emitter, Node};
use swc_ecma_compat_es2015::{block_scoping, classes};
use swc_ecma_compat_es2017::async_to_generator;
use swc_ecma_compat_es2022::class_properties::{self, class_properties};
use swc_ecma_compat_es2022::private_in_object;
use swc_ecma_parser::{EsSyntax, FlowSyntax, Syntax, TsSyntax, parse_file_as_program};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::helpers::{self, Helpers, inject_helpers};
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_typescript::{Config as TsStripConfig, typescript};
use swc_ecma_visit::VisitMutWith;
use swc_react_native::{CodegenOptions, CodegenVisitor, WorkletsOptions, WorkletsVisitor};

use crate::visitors::RemoveFlowTypeOnlyFields;

/// SWC wasm plugin entry — path on disk plus the JSON config to pass to it.
///
/// Builds without the `wasm_plugins` feature still expose this type for API
/// stability, but constructing a [`RollipopReactNativePlugin`] with a
/// non-empty plugins vec will return an error.
#[derive(Debug)]
pub struct SwcWasmPlugin {
  pub path: String,
  pub config: serde_json::Value,
}

/// Compat-pass preset selecting which Hermes generation we target.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTarget {
  /// Older Hermes.
  Hermes,
  /// Modern Hermes (the default).
  #[default]
  HermesV1,
}

/// Configuration for the react-native-worklets transform. Wraps
/// [`WorkletsOptions`] and is forwarded to the visitor as-is.
#[derive(Debug, Default, Clone)]
pub struct WorkletsConfig {
  pub options: WorkletsOptions,
}

/// Flow handling configuration. Mirrors Babel's `@babel/plugin-transform-flow-strip-types` semantics.
#[derive(Debug, Default, Clone, Copy)]
pub struct FlowConfig {
  /// When `true`, only files containing `@flow` or `@noflow` directive
  /// comments are parsed as Flow (Babel `requireDirective: true`).
  /// When `false` (default — matches Metro / Babel default), every JS
  /// module is parsed as Flow regardless of directive.
  pub require_directive: bool,
}

pub struct RollipopReactNativePlugin {
  runtime_target: RuntimeTarget,
  worklets: Option<WorkletsConfig>,
  flow: FlowConfig,
  #[cfg(feature = "wasm_plugins")]
  wasm_plugins: wasm_plugins::WasmPlugins,
}

impl RollipopReactNativePlugin {
  pub fn new(
    plugins: Vec<SwcWasmPlugin>,
    env_name: Option<String>,
    runtime_target: RuntimeTarget,
    worklets: Option<WorkletsConfig>,
    flow: Option<FlowConfig>,
  ) -> Result<Self, anyhow::Error> {
    #[cfg(not(feature = "wasm_plugins"))]
    {
      if !plugins.is_empty() {
        return Err(anyhow::anyhow!(
          "SWC wasm plugins are not supported in this build (the `wasm_plugins` feature is disabled — typically the `aarch64-pc-windows-msvc` target)",
        ));
      }
      drop(plugins);
      drop(env_name);
    }

    Ok(Self {
      runtime_target,
      worklets,
      flow: flow.unwrap_or_default(),
      #[cfg(feature = "wasm_plugins")]
      wasm_plugins: wasm_plugins::WasmPlugins::new(plugins, env_name)?,
    })
  }

  /// Whether to parse `args` as Flow. Only Js/Jsx inputs qualify; Ts/Tsx
  /// and other types are never Flow. Within Js/Jsx, the directive policy
  /// (`flow.require_directive`) decides whether a `@flow`/`@noflow` marker
  /// is required.
  fn is_flow_module(&self, args: &HookTransformArgs<'_>) -> bool {
    match args.module_type {
      ModuleType::Js | ModuleType::Jsx => {
        !self.flow.require_directive || flow::has_directive(args.code)
      }
      _ => false,
    }
  }
}

fn is_codegen_required(args: &HookTransformArgs<'_>, is_flow: bool) -> bool {
  let typed = is_flow || matches!(args.module_type, ModuleType::Ts | ModuleType::Tsx);
  typed && memchr::memmem::find(args.code.as_bytes(), b"codegenNativeComponent<").is_some()
}

impl fmt::Debug for RollipopReactNativePlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut s = f.debug_struct("RollipopReactNativePlugin");
    #[cfg(feature = "wasm_plugins")]
    s.field("plugins_count", &self.wasm_plugins.len());
    s.finish_non_exhaustive()
  }
}

struct SourceMapConfig;

impl SourceMapGenConfig for SourceMapConfig {
  fn file_name_to_source(&self, f: &FileName) -> String {
    f.to_string()
  }

  fn inline_sources_content(&self, _f: &FileName) -> bool {
    true
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
    let is_flow = self.is_flow_module(args);
    let syntax = if is_flow {
      Syntax::Flow(FlowSyntax {
        jsx: true,
        enums: true,
        components: true,
        decorators: true,
        pattern_matching: true,
        require_directive: false,
        all: true,
      })
    } else {
      match args.module_type {
        ModuleType::Js => Syntax::Es(EsSyntax::default()),
        ModuleType::Jsx => Syntax::Es(EsSyntax { jsx: true, ..Default::default() }),
        ModuleType::Ts => Syntax::Typescript(TsSyntax::default()),
        ModuleType::Tsx => Syntax::Typescript(TsSyntax { tsx: true, ..Default::default() }),
        _ => return Ok(None),
      }
    };

    let cm: Lrc<swc_common::SourceMap> = Arc::default();
    let comments = SingleThreadedComments::default();

    GLOBALS.set(&Globals::new(), || {
      let fm =
        cm.new_source_file(Lrc::new(FileName::Real(PathBuf::from(args.id))), args.code.clone());

      let mut errors = Vec::new();
      let mut program =
        parse_file_as_program(&fm, syntax, EsVersion::latest(), Some(&comments), &mut errors)
          .map_err(|e| anyhow::anyhow!("Parse error in '{}': {:?}", args.id, e))?;
      let unresolved_mark = Mark::new();
      let top_level_mark = Mark::new();

      #[cfg(feature = "wasm_plugins")]
      self.wasm_plugins.run(&cm, unresolved_mark, &comments, args.id, &mut program)?;

      resolver(unresolved_mark, top_level_mark, false).process(&mut program);

      // Codegen must run before TS strip — it relies on the type annotations.
      if is_codegen_required(args, is_flow) {
        program.visit_mut_with(&mut CodegenVisitor::new(
          Arc::clone(&cm),
          CodegenOptions { filename: args.id.to_string() },
        ));
      }

      if is_flow {
        program.visit_mut_with(&mut RemoveFlowTypeOnlyFields {});
      }

      helpers::HELPERS.set(&Helpers::new(true), || {
        // Strip TS/Flow types first so the worklets visitor sees plain JS,
        // matching the babel pipeline order in react-native-reanimated.
        typescript(
          TsStripConfig { flow_syntax: is_flow, ..Default::default() },
          unresolved_mark,
          top_level_mark,
        )
        .process(&mut program);

        if let Some(worklets) = &self.worklets {
          let mut options = worklets.options.clone();
          options.filename = Some(args.id.to_string());
          let mut visitor = WorkletsVisitor::new(options).with_source_map(Arc::clone(&cm));
          program.visit_mut_with(&mut visitor);
        }

        let class_props = class_properties(
          class_properties::Config {
            set_public_fields: true,
            private_as_properties: true,
            ..Default::default()
          },
          unresolved_mark,
        );

        match self.runtime_target {
          RuntimeTarget::HermesV1 => (
            class_props,
            private_in_object(),
            block_scoping(unresolved_mark),
            inject_helpers(unresolved_mark),
            fixer(Some(&comments)),
          )
            .process(&mut program),
          RuntimeTarget::Hermes => (
            class_props,
            private_in_object(),
            async_to_generator(async_to_generator::Config::default(), unresolved_mark),
            classes(classes::Config::default()),
            block_scoping(unresolved_mark),
            inject_helpers(unresolved_mark),
            fixer(Some(&comments)),
          )
            .process(&mut program),
        }
      });

      // Strip Flow pragmas after the SWC pipeline so they don't leak into
      // rolldown's downstream oxc parse — a stray `@flow` makes oxc reject the otherwise plain JS output.
      if is_flow {
        flow::strip_pragma_comments(&comments);
      }

      let mut buf = Vec::new();
      let mut src_map_buf = Vec::new();
      {
        let wr = JsWriter::new(Arc::clone(&cm), "\n", &mut buf, Some(&mut src_map_buf));
        let mut emitter = Emitter {
          cfg: swc_ecma_codegen::Config::default().with_target(EsVersion::Es5),
          cm: Arc::clone(&cm),
          comments: Some(&comments),
          wr,
        };
        program.emit_with(&mut emitter).map_err(|e| anyhow::anyhow!("Codegen error: {e}"))?;
      }

      let code =
        String::from_utf8(buf).map_err(|e| anyhow::anyhow!("Invalid UTF-8 in output: {e}"))?;

      let swc_source_map = cm.build_source_map(&src_map_buf, None, SourceMapConfig);
      let mut map_json = Vec::new();
      swc_source_map
        .to_writer(&mut map_json)
        .map_err(|e| anyhow::anyhow!("Failed to serialize source map: {e}"))?;
      let map_str =
        String::from_utf8(map_json).expect("swc source map JSON output is always valid UTF-8");
      let map = SourceMap::from_json_string(&map_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse source map: {e}"))?;

      Ok(Some(HookTransformOutput {
        code: Some(code),
        map: Some(map),
        module_type: if is_flow { Some(ModuleType::Jsx) } else { None },
        ..Default::default()
      }))
    })
  }

  fn transform_meta(&self) -> Option<rolldown_plugin::PluginHookMeta> {
    Some(rolldown_plugin::PluginHookMeta { order: Some(rolldown_plugin::PluginOrder::Pre) })
  }
}
