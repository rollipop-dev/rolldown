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

use rustc_hash::FxHashMap;
use swc_common::comments::SingleThreadedComments;
use swc_common::source_map::SourceMapGenConfig;
use swc_common::sync::Lrc;
use swc_common::{FileName, GLOBALS, Globals, Mark};
use swc_ecma_ast::{EsVersion, Expr, Pass, Program};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::{Emitter, Node};
use swc_ecma_compat_es2015::{block_scoping, classes, destructuring, parameters};
use swc_ecma_compat_es2017::async_to_generator;
use swc_ecma_compat_es2018::object_rest_spread;
use swc_ecma_compat_es2022::class_properties::{self, class_properties};
use swc_ecma_compat_es2022::private_in_object;
use swc_ecma_parser::{
  EsSyntax, FlowSyntax, Syntax, TsSyntax, parse_file_as_expr, parse_file_as_program,
};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::helpers::{self, Helpers, inject_helpers};
use swc_ecma_transforms_base::hygiene::hygiene;
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_module::{common_js, import_analysis, path::Resolver};
use swc_ecma_transforms_optimization::inline_globals;
use swc_ecma_transforms_react::jsx::{Options as JsxOptions, Runtime as JsxRuntime, jsx};
use swc_ecma_transforms_typescript::{
  Config as TsStripConfig, TsImportExportAssignConfig, typescript,
};
use swc_ecma_utils::NodeIgnoringSpan;
use swc_ecma_visit::VisitMutWith;
use swc_react_native::{CodegenOptions, CodegenVisitor, WorkletsOptions, WorkletsVisitor};

use crate::visitors::RemoveFlowTypeOnlyFields;

pub use swc_react_native::WorkletsOptions as SwcWorkletsOptions;

/// SWC wasm plugin entry — path on disk plus the JSON config to pass to it.
///
/// Builds without the `wasm_plugins` feature still expose this type for API
/// stability, but constructing a [`Transformer`] with a non-empty plugins
/// vec will return an error.
#[derive(Debug, Clone)]
pub struct SwcWasmPlugin {
  pub path: String,
  pub config: serde_json::Value,
}

/// JSX runtime selection for the React transform pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ReactRuntime {
  /// Leave JSX untouched. Default — paired with a downstream bundler
  /// that owns JSX handling (e.g. rolldown itself).
  #[default]
  Preserve,
  /// Compile to `_jsx` / `_jsxs` / `Fragment` imports from
  /// `react/jsx-runtime` (or `react/jsx-dev-runtime` in development).
  Automatic,
  /// Compile to `React.createElement` calls.
  Classic,
}

/// React transform pass options. Modeled on Babel's
/// `@babel/plugin-transform-react-jsx`, minus the dev-server-only
/// fast-refresh knobs (a bundler concern, not a test/precompile concern).
#[derive(Debug, Default, Clone)]
pub struct ReactConfig {
  pub runtime: ReactRuntime,
  /// Import source for the automatic runtime. Defaults to `"react"`.
  pub import_source: Option<String>,
  /// `pragma` for the classic runtime. Defaults to `"React.createElement"`.
  pub pragma: Option<String>,
  /// `pragmaFrag` for the classic runtime. Defaults to `"React.Fragment"`.
  pub pragma_frag: Option<String>,
  /// Throw when an XML namespace prefix is encountered (e.g. `<svg:path>`).
  pub throw_if_namespace: Option<bool>,
  /// When `true`, emits the development runtime (`__source` / `__self`
  /// debug props for automatic, `react/jsx-dev-runtime` import).
  pub development: bool,
}

/// Module transform mode for the SWC module stage.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SwcModuleType {
  /// Preserve the input module shape. ESM stays ESM, and CommonJS stays as
  /// ordinary JS expressions.
  #[default]
  Unambiguous,
  /// Transform ES module syntax to CommonJS, matching SWC's
  /// `module: { type: "commonjs" }` defaults.
  CommonJs,
}

/// SWC module transform configuration.
#[derive(Debug, Default, Clone)]
pub struct ModuleConfig {
  /// Defaults to [`SwcModuleType::Unambiguous`].
  pub r#type: SwcModuleType,
}

/// SWC-side configuration. Bundles knobs that affect the SWC pipeline itself
/// (wasm plugins, helper emission, React transform) so they can be passed
/// around as a unit.
#[derive(Debug, Default, Clone)]
pub struct SwcConfig {
  /// User-supplied SWC `.wasm` plugins to run before the built-in passes.
  pub plugins: Vec<SwcWasmPlugin>,
  /// When `true`, runtime helpers are emitted as `import` / `require` calls
  /// to `@swc/helpers` so a downstream bundler can deduplicate them. When
  /// `false` (default), helpers are inlined into each transformed file —
  /// safer for callers that hand the output straight to a runtime (e.g.
  /// jest) without a bundle step in between.
  pub external_helpers: bool,
  /// React (JSX) transform configuration. Skipped entirely when
  /// `react.runtime` is `Preserve`.
  pub react: ReactConfig,
  /// Module transform configuration. When omitted, defaults to
  /// `type: "unambiguous"`, preserving the input module shape.
  pub module: Option<ModuleConfig>,
  /// Global expression replacements such as `"import.meta.hot" => "undefined"`,
  /// matching SWC's `jsc.transform.optimizer.globals.vars` behavior.
  pub globals: FxHashMap<String, String>,
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

/// Transformer-level configuration. SWC-specific knobs (wasm plugins,
/// helper emission) live under [`SwcConfig`]; `env_name` is passed to
/// [`Transformer::new`] separately because it influences the eager wasm
/// plugin compilation step that happens once at construction.
#[derive(Debug, Default, Clone)]
pub struct TransformerOptions {
  pub runtime_target: RuntimeTarget,
  pub worklets: Option<WorkletsConfig>,
  pub flow: Option<FlowConfig>,
  pub swc: Option<SwcConfig>,
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
  module_type: SwcModuleType,
  #[cfg(feature = "wasm_plugins")]
  wasm_plugins: wasm_plugins::WasmPlugins,
}

impl Transformer {
  pub fn new(
    env_name: Option<String>,
    mut options: TransformerOptions,
  ) -> Result<Self, anyhow::Error> {
    let module_type = options
      .swc
      .as_ref()
      .and_then(|swc| swc.module.as_ref())
      .map_or(SwcModuleType::Unambiguous, |module| module.r#type);
    let plugins = options.swc.as_mut().map(|s| std::mem::take(&mut s.plugins)).unwrap_or_default();

    #[cfg(not(feature = "wasm_plugins"))]
    {
      if !plugins.is_empty() {
        return Err(anyhow::anyhow!(
          "SWC wasm plugins are not supported in this build (the `wasm_plugins` feature is disabled — typically the `aarch64-pc-windows-msvc` target)",
        ));
      }
      drop(env_name);
    }

    Ok(Self {
      options,
      module_type,
      #[cfg(feature = "wasm_plugins")]
      wasm_plugins: wasm_plugins::WasmPlugins::new(plugins, env_name)?,
    })
  }

  /// Returns `true` when `module_kind` should be parsed as Flow. Flow only
  /// applies to `Js`/`Jsx`; for those, the `require_directive` config
  /// decides whether `@flow`/`@noflow` must be present in the source.
  fn is_flow_module(&self, module_kind: ModuleKind, code: &str) -> bool {
    let require_directive = self.options.flow.is_some_and(|cfg| cfg.require_directive);
    match module_kind {
      ModuleKind::Js | ModuleKind::Jsx => !require_directive || flow::has_directive(code),
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

      helpers::HELPERS.set(
        &Helpers::new(self.options.swc.as_ref().is_some_and(|cfg| cfg.external_helpers)),
        || -> Result<(), anyhow::Error> {
          // Strip TS/Flow types first so the worklets visitor sees plain JS,
          // matching the babel pipeline order in react-native-reanimated.
          typescript(
            TsStripConfig {
              flow_syntax: is_flow,
              import_export_assign_config: match self.module_type {
                SwcModuleType::CommonJs => TsImportExportAssignConfig::Preserve,
                SwcModuleType::Unambiguous => TsImportExportAssignConfig::default(),
              },
              ..Default::default()
            },
            unresolved_mark,
            top_level_mark,
          )
          .process(&mut program);

          // Run the JSX pass before downstream class/worklet passes so they see the desugared call shape.
          if let Some(react) = self.options.swc.as_ref().map(|s| &s.react)
            && !matches!(react.runtime, ReactRuntime::Preserve)
          {
            jsx(
              Arc::clone(&cm),
              Some(&comments),
              to_jsx_options(react),
              top_level_mark,
              unresolved_mark,
            )
            .process(&mut program);
          }

          if let Some(worklets) = &self.options.worklets {
            let mut options = worklets.options.clone();
            options.filename = Some(input.filename.to_string());
            let mut visitor = WorkletsVisitor::new(options).with_source_map(Arc::clone(&cm));
            program.visit_mut_with(&mut visitor);
          }

          // SWC applies optimizer globals after plugin transforms and before compat/module passes.
          if let Some(swc) = self.options.swc.as_ref() {
            transform_globals(&mut program, &swc.globals, &cm)?;
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
              async_to_generator(async_to_generator::Config::default(), unresolved_mark),
              block_scoping(unresolved_mark),
            )
              .process(&mut program),
            RuntimeTarget::Hermes => (
              class_props,
              private_in_object(),
              async_to_generator(async_to_generator::Config::default(), unresolved_mark),
              object_rest_spread(object_rest_spread::Config::default()),
              parameters(parameters::Config::default(), unresolved_mark),
              destructuring(destructuring::Config::default()),
              classes(classes::Config::default()),
              block_scoping(unresolved_mark),
            )
              .process(&mut program),
          }

          finalize_program(&mut program, self.module_type, unresolved_mark, &comments);

          Ok(())
        },
      )?;

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

fn finalize_program(
  program: &mut Program,
  module_type: SwcModuleType,
  unresolved_mark: Mark,
  comments: &SingleThreadedComments,
) {
  match module_type {
    SwcModuleType::CommonJs => commonjs(unresolved_mark).process(program),
    SwcModuleType::Unambiguous => unambiguous(unresolved_mark).process(program),
  }

  (hygiene(), fixer(Some(comments))).process(program);
}

fn transform_globals(
  program: &mut Program,
  vars: &FxHashMap<String, String>,
  cm: &Lrc<swc_common::SourceMap>,
) -> Result<(), anyhow::Error> {
  if vars.is_empty() {
    return Ok(());
  }

  let mut globals = FxHashMap::default();
  let mut global_exprs = FxHashMap::default();

  for (key, value) in vars {
    let value = parse_globals_expr(cm, value)?;

    if key.contains('.') {
      global_exprs.insert(NodeIgnoringSpan::owned(parse_globals_expr(cm, key)?), value);
    } else {
      globals.insert(key.clone().into(), value);
    }
  }

  inline_globals(
    Lrc::new(FxHashMap::default()),
    Lrc::new(globals),
    Lrc::new(global_exprs),
    Lrc::new(FxHashMap::default()),
  )
  .process(program);

  Ok(())
}

fn parse_globals_expr(
  cm: &Lrc<swc_common::SourceMap>,
  source: &str,
) -> Result<Expr, anyhow::Error> {
  let fm = cm.new_source_file(Lrc::new(FileName::Anon), source.to_string());
  let mut errors = Vec::new();
  let expression = parse_file_as_expr(
    &fm,
    Syntax::Es(EsSyntax::default()),
    EsVersion::default(),
    None,
    &mut errors,
  )
  .map_err(|error| anyhow::anyhow!("Invalid `swc.globals` expression `{source}`: {error:?}"))?;

  if let Some(error) = errors.first() {
    return Err(anyhow::anyhow!("Invalid `swc.globals` expression `{source}`: {error:?}"));
  }

  Ok(*expression)
}

fn to_jsx_options(config: &ReactConfig) -> JsxOptions {
  let mut options = JsxOptions {
    runtime: Some(match config.runtime {
      ReactRuntime::Automatic => JsxRuntime::Automatic,
      ReactRuntime::Classic => JsxRuntime::Classic,
      ReactRuntime::Preserve => JsxRuntime::Preserve,
    }),
    development: Some(config.development),
    ..Default::default()
  };
  if let Some(s) = &config.import_source {
    options.import_source = Some(s.clone().into());
  }
  if let Some(s) = &config.pragma {
    options.pragma = Some(s.clone().into());
  }
  if let Some(s) = &config.pragma_frag {
    options.pragma_frag = Some(s.clone().into());
  }
  if let Some(v) = config.throw_if_namespace {
    options.throw_if_namespace = Some(v);
  }
  options
}

fn commonjs(unresolved_mark: Mark) -> impl Pass {
  let config = common_js::Config::default();
  let import_interop = config.import_interop();
  let ignore_dynamic = config.ignore_dynamic;

  (
    import_analysis::import_analyzer(import_interop, ignore_dynamic),
    inject_helpers(unresolved_mark),
    common_js::common_js(
      Resolver::default(),
      unresolved_mark,
      config,
      common_js::FeatureFlag { support_block_scoping: false, support_arrow: false },
    ),
  )
}

fn unambiguous(unresolved_mark: Mark) -> impl Pass {
  inject_helpers(unresolved_mark)
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
