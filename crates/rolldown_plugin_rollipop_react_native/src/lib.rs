mod visitors;

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use rolldown_sourcemap::SourceMap;
use swc_common::comments::SingleThreadedComments;
use swc_common::plugin::metadata::TransformPluginMetadataContext;
use swc_common::plugin::serialized::{PluginSerializedBytes, VersionedSerializable};
use swc_common::source_map::SourceMapGenConfig;
use swc_common::sync::Lrc;
use swc_common::{FileName, GLOBALS, Globals, Mark};
use swc_ecma_ast::{EsVersion, Module, Pass, Program};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::{Emitter, Node};
use swc_ecma_compat_es2015::{block_scoping, classes};
use swc_ecma_compat_es2017::async_to_generator;
use swc_ecma_compat_es2022::class_properties::{self, class_properties};
use swc_ecma_compat_es2022::private_in_object;
use swc_ecma_parser::{
  EsSyntax, FlowSyntax, Syntax, TsSyntax, parse_file_as_module, parse_file_as_program,
};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::helpers::{self, Helpers, inject_helpers};
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_typescript::{Config as TsStripConfig, typescript};
use swc_ecma_visit::VisitMutWith;
use swc_plugin_runner::create_plugin_transform_executor;
use swc_plugin_runner::plugin_module_bytes::{CompiledPluginModuleBytes, RawPluginModuleBytes};
use swc_react_native::{CodegenOptions, CodegenVisitor, WorkletsOptions, WorkletsVisitor};

use rolldown_common::ModuleType;
use rolldown_plugin::{
  HookTransformArgs, HookTransformOutput, HookTransformReturn, HookUsage, Plugin,
  SharedTransformPluginContext,
};

use crate::visitors::RemoveFlowTypeOnlyFields;

/// SWC wasm plugin entry — path on disk plus the JSON config to pass to it.
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

struct PreloadedSwcPlugin {
  compiled: CompiledPluginModuleBytes,
  config: Arc<serde_json::Value>,
}

pub struct RollipopReactNativePlugin {
  runtime_target: RuntimeTarget,
  runtime: Arc<dyn swc_plugin_runner::runtime::Runtime>,
  env_name: String,
  worklets: Option<WorkletsConfig>,
  plugins: Vec<PreloadedSwcPlugin>,
}

/// Resolve the active env name the same way `@swc/core` does:
/// `SWC_ENV` → `NODE_ENV` → `"development"`.
fn default_env_name() -> String {
  std::env::var("SWC_ENV")
    .or_else(|_| std::env::var("NODE_ENV"))
    .unwrap_or_else(|_| "development".into())
}

impl RollipopReactNativePlugin {
  pub fn new(
    plugins: Vec<SwcWasmPlugin>,
    env_name: Option<String>,
    runtime_target: RuntimeTarget,
    worklets: Option<WorkletsConfig>,
  ) -> Result<Self, anyhow::Error> {
    let runtime: Arc<dyn swc_plugin_runner::runtime::Runtime> =
      Arc::new(swc_plugin_backend_wasmer::WasmerRuntime);

    let preloaded = plugins
      .into_iter()
      .map(|p| {
        let wasm_bytes = std::fs::read(&p.path)
          .map_err(|e| anyhow::anyhow!("Failed to read wasm plugin '{}': {e}", p.path))?;
        let raw = RawPluginModuleBytes::new(p.path, wasm_bytes);
        let compiled = CompiledPluginModuleBytes::from_raw_module(&*runtime, raw);
        Ok(PreloadedSwcPlugin { compiled, config: Arc::new(p.config) })
      })
      .collect::<Result<Vec<_>, anyhow::Error>>()?;

    Ok(Self {
      plugins: preloaded,
      runtime,
      env_name: env_name.unwrap_or_else(default_env_name),
      runtime_target,
      worklets,
    })
  }

  fn has_flow_directive(code: &str) -> bool {
    memchr::memmem::find(code.as_bytes(), b"@flow").is_some()
  }

  fn should_parse_flow(args: &HookTransformArgs<'_>) -> bool {
    matches!(args.module_type, ModuleType::Js | ModuleType::Jsx)
      && Self::has_flow_directive(args.code)
  }

  fn needs_codegen_visit(code: &str) -> bool {
    memchr::memmem::find(code.as_bytes(), b"codegenNativeComponent<").is_some()
  }

  fn is_codegen_required(args: &HookTransformArgs<'_>, is_flow: bool) -> bool {
    let typed = is_flow || matches!(args.module_type, ModuleType::Ts | ModuleType::Tsx);
    typed && Self::needs_codegen_visit(args.code)
  }

  fn is_script_like(args: &HookTransformArgs<'_>) -> bool {
    args.id.ends_with(".cjs")
  }

  fn strip_flow_pragma_comments(comments: &SingleThreadedComments) {
    let (mut leading, mut trailing) = comments.borrow_all_mut();
    let retain = |c: &swc_common::comments::Comment| !Self::is_flow_pragma(&c.text);
    for list in leading.values_mut() {
      list.retain(retain);
    }
    for list in trailing.values_mut() {
      list.retain(retain);
    }
  }

  fn is_flow_pragma(text: &str) -> bool {
    text
      .lines()
      .any(|line| line.trim_start().trim_start_matches('*').trim_start().starts_with("@flow"))
  }

  fn run_wasm_plugins(
    &self,
    cm: &Lrc<swc_common::SourceMap>,
    unresolved_mark: Mark,
    comments: &SingleThreadedComments,
    filename: &str,
    program: &mut Program,
  ) -> Result<(), anyhow::Error> {
    if self.plugins.is_empty() {
      return Ok(());
    }

    let owned_program = std::mem::replace(program, Program::Module(Module::default()));
    let initial = PluginSerializedBytes::try_serialize(&VersionedSerializable::new(owned_program))?;

    let final_bytes = swc_plugin_proxy::COMMENTS.set(
      &swc_plugin_proxy::HostCommentsStorage { inner: Some(comments.clone()) },
      || -> Result<PluginSerializedBytes, anyhow::Error> {
        let mut serialized = initial;
        for plugin in &self.plugins {
          let cloned = plugin.compiled.clone_module(&*self.runtime);
          let metadata_context = Arc::new(TransformPluginMetadataContext::new(
            Some(filename.to_string()),
            self.env_name.clone(),
            None,
          ));
          let mut executor = create_plugin_transform_executor(
            cm,
            &unresolved_mark,
            &metadata_context,
            None,
            Box::new(cloned),
            Some((*plugin.config).clone()),
            Arc::clone(&self.runtime),
          );
          serialized = executor.transform(&serialized, Some(true))?;
        }
        Ok(serialized)
      },
    )?;

    *program = final_bytes.deserialize()?.into_inner();
    Ok(())
  }
}

impl fmt::Debug for RollipopReactNativePlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("RollipopReactNativePlugin")
      .field("plugins_count", &self.plugins.len())
      .field("env_name", &self.env_name)
      .finish_non_exhaustive()
  }
}

struct SourceMapConfig;

impl SourceMapGenConfig for SourceMapConfig {
  fn file_name_to_source(&self, f: &FileName) -> String {
    f.to_string()
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
    let is_flow = Self::should_parse_flow(args);
    let syntax = match args.module_type {
      ModuleType::Js if is_flow => Syntax::Flow(FlowSyntax {
        jsx: true,
        enums: true,
        components: true,
        decorators: true,
        pattern_matching: true,
        require_directive: false,
        all: true,
      }),
      ModuleType::Js => Syntax::Es(EsSyntax::default()),
      ModuleType::Jsx => Syntax::Es(EsSyntax { jsx: true, ..Default::default() }),
      ModuleType::Ts => Syntax::Typescript(TsSyntax::default()),
      ModuleType::Tsx => Syntax::Typescript(TsSyntax { tsx: true, ..Default::default() }),
      _ => return Ok(None),
    };

    let cm: Lrc<swc_common::SourceMap> = Arc::default();
    let comments = SingleThreadedComments::default();

    GLOBALS.set(&Globals::new(), || {
      let fm =
        cm.new_source_file(Lrc::new(FileName::Real(PathBuf::from(args.id))), args.code.clone());

      let mut errors = Vec::new();
      let mut program = if Self::is_script_like(args) {
        parse_file_as_program(&fm, syntax, EsVersion::latest(), Some(&comments), &mut errors)
          .map_err(|e| anyhow::anyhow!("Parse error in '{}': {:?}", args.id, e))?
      } else {
        let module =
          parse_file_as_module(&fm, syntax, EsVersion::latest(), Some(&comments), &mut errors)
            .map_err(|e| anyhow::anyhow!("Parse error in '{}': {:?}", args.id, e))?;
        Program::Module(module)
      };
      let unresolved_mark = Mark::new();
      let top_level_mark = Mark::new();

      self.run_wasm_plugins(&cm, unresolved_mark, &comments, args.id, &mut program)?;

      resolver(unresolved_mark, top_level_mark, false).process(&mut program);

      // Codegen must run before TS strip — it relies on the type annotations.
      if Self::is_codegen_required(args, is_flow) {
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
        Self::strip_flow_pragma_comments(&comments);
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
