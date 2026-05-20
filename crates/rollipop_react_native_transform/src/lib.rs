//! Standalone SWC transform pipeline for React Native modules.
//!
//! Mirrors the Babel preset used by Metro: Flow strip → codegen native
//! component → TS strip → react-native-worklets → Hermes compat passes.
//! Optional user-supplied SWC `.wasm` plugins run first (under the
//! `wasm_plugins` feature).
//!
//! This crate has no rolldown dependencies. It powers both the rolldown
//! plugin (which forwards `HookTransformArgs` into [`Transformer`]) and a
//! NAPI binding exposed via `rolldown_binding`.

mod flow;
mod visitors;
#[cfg(feature = "wasm_plugins")]
mod wasm_plugins;

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

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

pub use swc_react_native::WorkletsOptions as SwcWorkletsOptions;

/// SWC wasm plugin entry — path on disk plus the JSON config to pass to it.
///
/// Builds without the `wasm_plugins` feature still expose this type for API
/// stability, but constructing a [`Transformer`] with a non-empty plugins
/// vec will return an error.
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

/// Module kind of the input source. The transformer uses this to pick the
/// initial SWC parser syntax and to gate Flow handling (only `Js`/`Jsx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
  Js,
  Jsx,
  Ts,
  Tsx,
}

/// Transformer-level configuration. The wasm plugin list and `env_name` are
/// passed to [`Transformer::new`] separately because they require eager
/// compilation/initialization that happens once at construction.
#[derive(Debug, Default, Clone)]
pub struct TransformerOptions {
  pub runtime_target: RuntimeTarget,
  pub worklets: Option<WorkletsConfig>,
  pub flow: FlowConfig,
}

/// Input to a single [`Transformer::transform`] call.
///
/// `module_kind` is optional — when `None`, it is inferred from
/// `filename` via [`infer_module_kind_from_filename`]. Callers that
/// already know the kind (e.g. the rolldown plugin reads it from
/// `HookTransformArgs::module_type`) should pass `Some` to avoid the
/// extra extension scan and to keep behavior consistent with
/// bundler-driven typing.
#[derive(Debug, Clone, Copy)]
pub struct TransformInput<'a> {
  pub filename: &'a str,
  pub code: &'a str,
  pub module_kind: Option<ModuleKind>,
}

/// Infer the [`ModuleKind`] from `filename` using rolldown's default
/// extension table (`js`/`mjs`/`cjs` → `Js`, `jsx` → `Jsx`,
/// `ts`/`mts`/`cts` → `Ts`, `tsx` → `Tsx`).
///
/// Matches rolldown's behavior in `get_module_loader_from_file_extension`:
/// scan `.` positions left-to-right and take the first suffix that maps to
/// a known kind. This handles compound extensions correctly (`foo.d.ts` →
/// `Ts`, `foo.test.tsx` → `Tsx`). Falls back to `Js` when no suffix matches.
pub fn infer_module_kind_from_filename(filename: &str) -> ModuleKind {
  let bytes = filename.as_bytes();
  for i in memchr::memchr_iter(b'.', bytes) {
    let suffix = &filename[i + 1..];
    match suffix {
      "tsx" => return ModuleKind::Tsx,
      "ts" | "mts" | "cts" => return ModuleKind::Ts,
      "jsx" => return ModuleKind::Jsx,
      "js" | "mjs" | "cjs" => return ModuleKind::Js,
      _ => {}
    }
  }
  ModuleKind::Js
}

/// Output of [`Transformer::transform`].
#[derive(Debug)]
pub struct TransformOutput {
  pub code: String,
  /// JSON-encoded source map (sourcemap V3). Kept as a string to avoid
  /// leaking SWC-internal types in the public API; downstream callers can
  /// parse it into whatever representation they need.
  pub map_json: String,
  /// `Some(ModuleKind::Jsx)` when the input was Flow — Flow inputs are
  /// downgraded to JSX after pragma stripping because downstream parsers
  /// (oxc, swc Es) reject `@flow` annotations. `None` when the effective
  /// module kind is unchanged from the input.
  pub output_module_kind: Option<ModuleKind>,
}

/// Pre-compiled, reusable transformer.
///
/// Construction does the expensive work (wasm plugin compilation, runtime
/// init); subsequent [`Self::transform`] calls only reuse what was
/// preloaded. Hold the instance for the lifetime of the consumer (the
/// rolldown plugin instance, or a long-lived NAPI handle).
pub struct Transformer {
  options: TransformerOptions,
  #[cfg(feature = "wasm_plugins")]
  wasm_plugins: wasm_plugins::WasmPlugins,
}

impl Transformer {
  pub fn new(
    plugins: Vec<SwcWasmPlugin>,
    env_name: Option<String>,
    options: TransformerOptions,
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
      options,
      #[cfg(feature = "wasm_plugins")]
      wasm_plugins: wasm_plugins::WasmPlugins::new(plugins, env_name)?,
    })
  }

  /// Returns `true` when `module_kind` should be parsed as Flow. Flow only
  /// applies to `Js`/`Jsx`; for those, the `require_directive` config
  /// decides whether `@flow`/`@noflow` must be present in the source.
  fn is_flow_module(&self, module_kind: ModuleKind, code: &str) -> bool {
    match module_kind {
      ModuleKind::Js | ModuleKind::Jsx => {
        !self.options.flow.require_directive || flow::has_directive(code)
      }
      ModuleKind::Ts | ModuleKind::Tsx => false,
    }
  }

  pub fn transform(&self, input: TransformInput<'_>) -> Result<TransformOutput, anyhow::Error> {
    let module_kind =
      input.module_kind.unwrap_or_else(|| infer_module_kind_from_filename(input.filename));
    let is_flow = self.is_flow_module(module_kind, input.code);
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
      match module_kind {
        ModuleKind::Js => Syntax::Es(EsSyntax::default()),
        ModuleKind::Jsx => Syntax::Es(EsSyntax { jsx: true, ..Default::default() }),
        ModuleKind::Ts => Syntax::Typescript(TsSyntax::default()),
        ModuleKind::Tsx => Syntax::Typescript(TsSyntax { tsx: true, ..Default::default() }),
      }
    };

    let cm: Lrc<swc_common::SourceMap> = Arc::default();
    let comments = SingleThreadedComments::default();

    GLOBALS.set(&Globals::new(), || {
      let fm = cm.new_source_file(
        Lrc::new(FileName::Real(PathBuf::from(input.filename))),
        input.code.to_string(),
      );

      let mut errors = Vec::new();
      let mut program =
        parse_file_as_program(&fm, syntax, EsVersion::latest(), Some(&comments), &mut errors)
          .map_err(|e| anyhow::anyhow!("Parse error in '{}': {:?}", input.filename, e))?;
      let unresolved_mark = Mark::new();
      let top_level_mark = Mark::new();

      #[cfg(feature = "wasm_plugins")]
      self.wasm_plugins.run(&cm, unresolved_mark, &comments, input.filename, &mut program)?;

      resolver(unresolved_mark, top_level_mark, false).process(&mut program);

      // Codegen must run before TS strip — it relies on the type annotations.
      if is_codegen_required(module_kind, input.code, is_flow) {
        program.visit_mut_with(&mut CodegenVisitor::new(
          Arc::clone(&cm),
          CodegenOptions { filename: input.filename.to_string() },
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

        if let Some(worklets) = &self.options.worklets {
          let mut options = worklets.options.clone();
          options.filename = Some(input.filename.to_string());
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

        match self.options.runtime_target {
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
      // the downstream parser — a stray `@flow` makes oxc reject the otherwise plain JS output.
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
      let map_json =
        String::from_utf8(map_json).expect("swc source map JSON output is always valid UTF-8");

      Ok(TransformOutput {
        code,
        map_json,
        output_module_kind: if is_flow { Some(ModuleKind::Jsx) } else { None },
      })
    })
  }
}

fn is_codegen_required(module_kind: ModuleKind, code: &str, is_flow: bool) -> bool {
  let typed = is_flow || matches!(module_kind, ModuleKind::Ts | ModuleKind::Tsx);
  typed && memchr::memmem::find(code.as_bytes(), b"codegenNativeComponent<").is_some()
}

impl fmt::Debug for Transformer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut s = f.debug_struct("Transformer");
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
