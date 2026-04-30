use std::{borrow::Cow, path::Path, sync::LazyLock};

use arcstr::ArcStr;
use oxc::codegen::{Codegen, CodegenOptions, CodegenReturn, CommentOptions};
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{
  JsxOptions, JsxRuntime, ReactRefreshOptions, TransformOptions, Transformer,
};
use regex::Regex;
use rolldown_error::{BatchedBuildDiagnostic, BuildDiagnostic, EventKind, Severity};
use rolldown_plugin::{HookTransformOutput, HookUsage, Plugin, SharedTransformPluginContext};
use rolldown_plugin_utils::to_string_literal;
use rolldown_sourcemap::{SourceMap, collapse_sourcemaps};
use rolldown_utils::pattern_filter::{FilterResult, StringOrRegex, filter};
use string_wizard::{MagicString, SourceMapOptions};

static REACT_COMP_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"extends\s+(?:React\.)?(?:Pure)?Component").unwrap());

#[derive(Debug)]
pub struct RollipopReactRefreshWrapperPluginOptions {
  pub cwd: String,
  pub include: Vec<StringOrRegex>,
  pub exclude: Vec<StringOrRegex>,
  pub jsx_import_source: Option<String>,
}

#[derive(Debug)]
pub struct RollipopReactRefreshWrapperPlugin {
  cwd: String,
  include: Vec<StringOrRegex>,
  exclude: Vec<StringOrRegex>,
  transform_options: TransformOptions,
}

impl RollipopReactRefreshWrapperPlugin {
  pub fn new(options: RollipopReactRefreshWrapperPluginOptions) -> Self {
    let transform_options = TransformOptions {
      jsx: JsxOptions {
        // `react-refresh` should work with development mode.
        development: true,
        runtime: JsxRuntime::Automatic,
        import_source: options.jsx_import_source,
        refresh: Some(ReactRefreshOptions {
          refresh_reg: "global.$RefreshReg$".to_string(),
          refresh_sig: "global.$RefreshSig$".to_string(),
          ..ReactRefreshOptions::default()
        }),
        ..JsxOptions::default()
      },
      ..TransformOptions::default()
    };

    Self { cwd: options.cwd, include: options.include, exclude: options.exclude, transform_options }
  }

  /// Wraps code with react-refresh boundary using MagicString for sourcemap support.
  fn add_refresh_wrapper(&self, code: &str, id: &str) -> Option<(String, SourceMap)> {
    let has_refresh = memchr::memmem::find(code.as_bytes(), b"$RefreshReg$(").is_some();

    if !(has_refresh || REACT_COMP_RE.is_match(code)) {
      return None;
    }

    let escaped_id = to_string_literal(id);
    let mut ms = MagicString::new(code);

    if has_refresh {
      ms.prepend(format!(
        "\
var __prev$RefreshReg$ = global.$RefreshReg$;
var __prev$RefreshSig$ = global.$RefreshSig$;
global.$RefreshReg$ = function(type, id) {{ return __ReactRefresh.register(type, {escaped_id} + ' ' + id) }}
global.$RefreshSig$ = function() {{ return __ReactRefresh.createSignatureFunctionForTransform(); }}
"
      ));
    }

    let mut suffix = "\
\nif (import.meta.hot) {{
  if (import.meta.hot.refresh == null) throw new Error('react-refresh runtime is not initialized');
  import.meta.hot.accept((nextExports) => {{
    if (!nextExports) return;
    if (import.meta.hot.refreshUtils.isReactRefreshBoundary(nextExports)) {{
      import.meta.hot.refreshUtils.enqueueUpdate();
    }}
  }});
}}"
      .to_string();

    if has_refresh {
      suffix.push_str(
        "\
\nglobal.$RefreshReg$ = __prev$RefreshReg$;
global.$RefreshSig$ = __prev$RefreshSig$;
",
      );
    }

    ms.append(suffix);

    let map = ms.source_map(SourceMapOptions {
      source: id.into(),
      include_content: true,
      ..Default::default()
    });

    Some((ms.to_string(), map))
  }
}

impl Plugin for RollipopReactRefreshWrapperPlugin {
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed("builtin:rollipop-react-refresh-wrapper")
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::Transform
  }

  async fn transform(
    &self,
    _ctx: SharedTransformPluginContext,
    args: &rolldown_plugin::HookTransformArgs<'_>,
  ) -> rolldown_plugin::HookTransformReturn {
    if matches!(
      filter(Some(&self.exclude), Some(&self.include), args.id, &self.cwd),
      FilterResult::Match(false) | FilterResult::NoneMatch(false)
    ) {
      return Ok(None);
    }

    let source_type = source_type_from_id(args.id);

    if !source_type.is_jsx() {
      return Ok(None);
    }

    let allocator = oxc::allocator::Allocator::default();
    let ret = Parser::new(&allocator, args.code, source_type).parse();
    if ret.panicked || !ret.errors.is_empty() {
      return Err(BatchedBuildDiagnostic::new(BuildDiagnostic::from_oxc_diagnostics(
        ret.errors,
        &ArcStr::from(args.code),
        args.id,
        Severity::Error,
        EventKind::ParseError,
      )))?;
    }

    let mut program = ret.program;
    let scoping = SemanticBuilder::new().build(&program).semantic.into_scoping();
    let transformer = Transformer::new(&allocator, Path::new(args.id), &self.transform_options);
    let transformer_return = transformer.build_with_scoping(scoping, &mut program);
    if !transformer_return.errors.is_empty() {
      return Err(BatchedBuildDiagnostic::new(BuildDiagnostic::from_oxc_diagnostics(
        transformer_return.errors,
        &ArcStr::from(args.code),
        args.id,
        Severity::Error,
        EventKind::ParseError,
      )))?;
    }

    let CodegenReturn { code, map: oxc_map, .. } = Codegen::new()
      .with_options(CodegenOptions {
        comments: CommentOptions { normal: false, ..CommentOptions::default() },
        source_map_path: Some(args.id.into()),
        ..CodegenOptions::default()
      })
      .build(&program);

    if let Some((wrapped_code, wrapper_map)) = self.add_refresh_wrapper(&code, args.id) {
      let final_map = match oxc_map {
        Some(oxc_map) => Some(collapse_sourcemaps(&[&oxc_map, &wrapper_map])),
        None => Some(wrapper_map),
      };
      return Ok(Some(HookTransformOutput {
        code: Some(wrapped_code),
        map: final_map,
        ..Default::default()
      }));
    }

    Ok(Some(HookTransformOutput { code: Some(code), map: oxc_map, ..Default::default() }))
  }
}

fn source_type_from_id(id: &str) -> SourceType {
  let id = id.split_once('?').map_or(id, |(id, _)| id);
  match Path::new(id).extension().and_then(|e| e.to_str()) {
    Some("jsx") => SourceType::jsx(),
    Some("tsx") => SourceType::tsx(),
    Some("ts" | "cts" | "mts") => SourceType::ts(),
    _ => SourceType::mjs(),
  }
}
