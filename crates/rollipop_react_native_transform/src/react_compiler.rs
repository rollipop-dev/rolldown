use std::fmt::Debug;
use std::sync::Arc;

use react_compiler::entrypoint::compile_result::CompileResult;
use swc_common::comments::SingleThreadedComments;
use swc_ecma_ast::Program;
use swc_ecma_react_compiler::diagnostics::{Severity, compile_result_to_diagnostics};
use swc_ecma_react_compiler::{CompilerTarget, PluginOptions, SourceType, fast_check};

use crate::{ReactCompilerDynamicGating, ReactCompilerGating, TransformInput};

/// Reusable React Compiler configuration for the native SWC pipeline.
#[derive(Debug, Default, Clone)]
pub struct ReactCompilerOptions {
  /// Host-side filename filter, evaluated before compiler analysis.
  pub filter: Option<Arc<dyn ReactCompilerFilter>>,

  /// Which functions to compile.
  ///
  /// @default 'infer'
  pub compilation_mode: Option<String>,

  /// What to do when a function cannot be compiled.
  ///
  /// @default 'none'
  pub panic_threshold: Option<String>,

  /// React runtime version target. `17` and `18` require the
  /// `react-compiler-runtime` package; `19` ships the runtime in `react`.
  ///
  /// @default '19'
  pub target: Option<String>,

  /// Analyze and report diagnostics only; emit no transformed code.
  ///
  /// @default false
  pub no_emit: Option<bool>,

  /// Compiler output mode.
  ///
  /// @default undefined
  pub output_mode: Option<String>,

  /// Compile even functions marked with the `"use no memo"` / `"use no forget"`
  /// opt-out directives.
  ///
  /// @default false
  pub ignore_use_no_forget: Option<bool>,

  /// Treat Flow suppression comments as opt-outs.
  ///
  /// @default true
  pub flow_suppressions: Option<bool>,

  /// Enable `react-native-reanimated` support.
  ///
  /// @default false
  pub enable_reanimated: Option<bool>,

  /// Development mode (extra validation / instrumentation).
  ///
  /// @default false
  pub is_dev: Option<bool>,

  /// Source file name, used for the fast-refresh hash and in diagnostics.
  pub filename: Option<String>,

  /// ESLint rules whose suppressions opt a function out of compilation.
  pub eslint_suppression_rules: Option<Vec<String>>,

  /// Extra directives that opt a function out of compilation.
  pub custom_opt_out_directives: Option<Vec<String>>,

  /// Also emit a gated (feature-flagged) version of each compiled function.
  pub gating: Option<ReactCompilerGating>,

  /// Dynamically-gated compilation.
  pub dynamic_gating: Option<ReactCompilerDynamicGating>,
}

/// Host-side predicate evaluated before any compiler analysis or AST conversion.
pub trait ReactCompilerFilter: Debug + Send + Sync {
  fn matches(&self, filename: &str, cwd: &str) -> bool;
}

pub fn transform(
  program: &mut Program,
  comments: &SingleThreadedComments,
  input: &TransformInput<'_>,
  is_typescript: bool,
  config: &ReactCompilerOptions,
) -> anyhow::Result<Vec<String>> {
  let compilation_mode = config
    .compilation_mode
    .as_deref()
    .filter(|mode| matches!(*mode, "infer" | "syntax" | "annotation" | "all"))
    .unwrap_or("infer");
  if config.filter.as_ref().is_some_and(|filter| !filter.matches(input.filename, input.cwd))
    || (matches!(compilation_mode, "infer" | "annotation") && !fast_check::is_required(program))
  {
    return Ok(Vec::new());
  }

  // Allocate SWC's owned options only after the filename and AST gates pass.
  // See internal-docs/react-native-compiler/implementation.md.
  let options = PluginOptions {
    should_compile: true,
    compilation_mode: compilation_mode.into(),
    panic_threshold: config
      .panic_threshold
      .as_deref()
      .filter(|threshold| matches!(*threshold, "none" | "critical_errors" | "all_errors"))
      .unwrap_or("none")
      .into(),
    target: CompilerTarget::Version(config.target.as_deref().unwrap_or("19").into()),
    no_emit: config.no_emit.unwrap_or(false),
    output_mode: config
      .output_mode
      .as_deref()
      .filter(|mode| matches!(*mode, "client" | "ssr" | "lint"))
      .map(str::to_owned),
    ignore_use_no_forget: config.ignore_use_no_forget.unwrap_or(false),
    flow_suppressions: config.flow_suppressions.unwrap_or(true),
    enable_reanimated: config.enable_reanimated.unwrap_or(false),
    is_dev: config.is_dev.unwrap_or(false),
    filename: Some(config.filename.as_deref().unwrap_or(input.filename).to_owned()),
    eslint_suppression_rules: config.eslint_suppression_rules.clone(),
    custom_opt_out_directives: config.custom_opt_out_directives.clone(),
    gating: config.gating.clone(),
    dynamic_gating: config.dynamic_gating.clone(),
    #[expect(clippy::default_trait_access, reason = "SWC does not re-export the environment type")]
    environment: Default::default(),
    source_code: None,
    profiling: false,
    debug: false,
  };
  transform_with_options(program, comments, input, is_typescript, options)
}

fn transform_with_options(
  program: &mut Program,
  comments: &SingleThreadedComments,
  input: &TransformInput<'_>,
  is_typescript: bool,
  mut options: PluginOptions,
) -> anyhow::Result<Vec<String>> {
  // SWC gates lint diagnostics on no_emit even when the core selects lint via output_mode.
  options.no_emit |= options.output_mode.as_deref() == Some("lint");
  let no_emit = options.no_emit;
  let result = swc_ecma_react_compiler::transform(
    program,
    SourceType::from_program(program).with_typescript(is_typescript),
    input.code,
    Some(comments),
    options,
  );

  // SWC 25 omits CompileResult's fatal bit.
  // A fatal result adds a root Error before its logger diagnostics;
  // successful noEmit lint can also report uncategorized pipeline errors.
  // Compare against the same logger conversion only on this error path, without reparsing or serializing compiler events.
  // See internal-docs/react-native-compiler/implementation.md.
  let has_root_error = result.diagnostics.iter().any(|diagnostic| {
    matches!(diagnostic.severity, Severity::Error) && diagnostic.category.is_none()
  }) && result.diagnostics.len()
    > compile_result_to_diagnostics(
      &CompileResult::Success {
        ast: None,
        events: result.events,
        ordered_log: Vec::new(),
        renames: Vec::new(),
        timing: Vec::new(),
      },
      no_emit,
    )
    .len();
  let warnings: Vec<_> = result
    .diagnostics
    .into_iter()
    .map(|diagnostic| format!("{}: {}", input.filename, diagnostic.message))
    .collect();
  anyhow::ensure!(!has_root_error, "{}", warnings.join("\n"));

  if let Some(compiled) = result.program {
    *program = compiled;
  }

  Ok(warnings)
}

#[cfg(test)]
mod tests {
  use swc_common::{FileName, GLOBALS, Globals, SourceMap};
  use swc_ecma_ast::EsVersion;
  use swc_ecma_parser::{EsSyntax, Syntax, parse_file_as_program};
  use swc_ecma_react_compiler::default_plugin_options;

  use super::*;

  fn compile_with_pipeline_error(panic_threshold: &str) -> anyhow::Result<Vec<String>> {
    GLOBALS.set(&Globals::new(), || {
      let input = TransformInput {
        cwd: "",
        filename: "App.jsx",
        code: "export function App({ value }) { const doubled = value * 2; return <View value={doubled}/>; }",
        module_kind: None,
      };
      let cm = SourceMap::default();
      let comments = SingleThreadedComments::default();
      let fm = cm.new_source_file(FileName::Custom(input.filename.into()).into(), input.code.to_string());
      let mut program = parse_file_as_program(
        &fm, Syntax::Es(EsSyntax { jsx: true, ..Default::default() }),
        EsVersion::latest(), Some(&comments), &mut Vec::new(),
      ).unwrap();
      let mut options = default_plugin_options();
      options.no_emit = true;
      options.panic_threshold = panic_threshold.into();
      options.environment.throw_unknown_exception_testonly = true;
      transform_with_options(&mut program, &comments, &input, false, options)
    })
  }

  #[test]
  fn no_emit_keeps_nonfatal_pipeline_errors_as_warnings() {
    let warnings = compile_with_pipeline_error("none").unwrap();
    assert!(warnings.iter().any(|warning| warning.contains("Pipeline error")), "{warnings:?}");
  }

  #[test]
  fn strict_no_emit_still_fails_on_pipeline_errors() {
    compile_with_pipeline_error("all_errors").unwrap_err();
  }
}
