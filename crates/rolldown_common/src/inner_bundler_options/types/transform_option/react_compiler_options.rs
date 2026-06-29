// MARK: - Rollipop

use rolldown_utils::{
  pattern_filter::{StringOrRegex, filter},
  url::clean_url,
};

/// Mirrors `oxc_react_compiler::PluginOptions`; keep compiler option fields aligned with Oxc when updating this type.
/// The file filter fields are Rollipop-side gates around that Oxc option shape.
#[derive(Debug, Default, Clone)]
pub struct ReactCompilerOptions {
  /// File patterns to compile. Empty means all files that enter the transform pipeline.
  pub include: Vec<StringOrRegex>,

  /// File patterns to skip.
  pub exclude: Vec<StringOrRegex>,

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

impl ReactCompilerOptions {
  pub fn should_transform(&self, id: &str, cwd: &str) -> bool {
    if self.include.is_empty() && self.exclude.is_empty() {
      return true;
    }

    let exclude = (!self.exclude.is_empty()).then_some(self.exclude.as_slice());
    let include = (!self.include.is_empty()).then_some(self.include.as_slice());

    if filter(exclude, include, id, cwd).inner() {
      return true;
    }

    let cleaned_id = clean_url(id);
    if cleaned_id != id {
      return filter(exclude, include, cleaned_id, cwd).inner();
    }

    false
  }
}

#[derive(Debug, Clone)]
pub struct ReactCompilerGating {
  /// Module the gating import comes from.
  pub source: String,
  /// Imported specifier used as the gate.
  pub import_specifier_name: String,
}

#[derive(Debug, Clone)]
pub struct ReactCompilerDynamicGating {
  /// Module the gating import comes from.
  pub source: String,
}

impl From<ReactCompilerOptions> for oxc_react_compiler::PluginOptions {
  fn from(value: ReactCompilerOptions) -> Self {
    let mut options = oxc_react_compiler::default_plugin_options();
    if let Some(compilation_mode) = value.compilation_mode {
      options.compilation_mode = compilation_mode;
    }
    if let Some(panic_threshold) = value.panic_threshold {
      options.panic_threshold = panic_threshold;
    }
    if let Some(target) = value.target {
      options.target = oxc_react_compiler::CompilerTarget::Version(target);
    }
    if let Some(no_emit) = value.no_emit {
      options.no_emit = no_emit;
    }
    if value.output_mode.is_some() {
      options.output_mode = value.output_mode;
    }
    if let Some(ignore_use_no_forget) = value.ignore_use_no_forget {
      options.ignore_use_no_forget = ignore_use_no_forget;
    }
    if let Some(flow_suppressions) = value.flow_suppressions {
      options.flow_suppressions = flow_suppressions;
    }
    if let Some(enable_reanimated) = value.enable_reanimated {
      options.enable_reanimated = enable_reanimated;
    }
    if let Some(is_dev) = value.is_dev {
      options.is_dev = is_dev;
    }
    if value.filename.is_some() {
      options.filename = value.filename;
    }
    if value.eslint_suppression_rules.is_some() {
      options.eslint_suppression_rules = value.eslint_suppression_rules;
    }
    if value.custom_opt_out_directives.is_some() {
      options.custom_opt_out_directives = value.custom_opt_out_directives;
    }
    if let Some(gating) = value.gating {
      options.gating = Some(oxc_react_compiler::GatingConfig {
        source: gating.source,
        import_specifier_name: gating.import_specifier_name,
      });
    }
    if let Some(dynamic_gating) = value.dynamic_gating {
      options.dynamic_gating =
        Some(oxc_react_compiler::DynamicGatingConfig { source: dynamic_gating.source });
    }
    options
  }
}
