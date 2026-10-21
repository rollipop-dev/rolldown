use std::sync::Arc;

use rollipop_react_native_transform::{
  ReactCompilerFilter, ReactCompilerOptions, ReactConfig, RuntimeTarget, SwcConfig,
  SwcWorkletsOptions, TransformInput, TransformOutput, Transformer, TransformerOptions,
  WorkletsConfig,
};
use swc_common::{GLOBALS, Globals, SourceMap, Span};
use swc_ecma_ast::{ArrowExpr, Function, Pat, VarDeclarator};
use swc_ecma_parser::{Syntax, TsSyntax, parse_file_as_program};
use swc_ecma_visit::{Visit, VisitWith};

const COMPONENT: &str =
  "export function App({ value }) { const doubled = value * 2; return <View value={doubled}/>; }";
const VAR_COMPONENT: &str =
  "export function App({ value }) { var doubled = value * 2; return <View value={doubled}/>; }";

fn transform(
  filename: &str,
  source: &str,
  runtime_target: RuntimeTarget,
  config: Option<ReactCompilerOptions>,
) -> anyhow::Result<TransformOutput> {
  Transformer::new(
    None,
    TransformerOptions {
      runtime_target,
      swc: Some(SwcConfig {
        react: ReactConfig { compiler: config, ..Default::default() },
        ..Default::default()
      }),
      ..Default::default()
    },
  )?
  .transform(TransformInput { cwd: "", filename, code: source, module_kind: None })
}

#[test]
fn compiles_lexical_components_before_both_hermes_lowering_presets() {
  for runtime_target in [RuntimeTarget::Hermes, RuntimeTarget::HermesV1] {
    let output =
      transform("App.jsx", COMPONENT, runtime_target, Some(ReactCompilerOptions::default()))
        .unwrap();
    assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
    assert!(!output.code.contains("const doubled"), "{}", output.code);
    assert!(output.warnings.is_empty(), "{:?}", output.warnings);
  }
}

#[test]
fn compiles_flow_and_typescript_before_type_stripping() {
  for (filename, source) in [
    (
      "App.jsx",
      "// @flow\ntype Props = $ReadOnly<{ value: number }>; export function App({ value }: Props) { const doubled: number = value * 2; return <View value={doubled}/>; }",
    ),
    (
      "App.tsx",
      "type Props = { readonly value: number }; export function App({ value }: Props) { const doubled: number = value * 2; return <View value={doubled}/>; }",
    ),
  ] {
    let output =
      transform(filename, source, RuntimeTarget::HermesV1, Some(ReactCompilerOptions::default()))
        .unwrap();
    assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
    assert!(!output.code.contains("type Props"), "{}", output.code);
    assert!(!output.code.contains(": number"), "{}", output.code);
  }
}

#[test]
fn infers_typescript_with_queries_and_fragments() {
  let source = "export function App({ value }: { value: number }) { const doubled = (value as number) * 2; return <View value={doubled}/>; }";
  for filename in ["App.tsx?platform=ios", "App.tsx#component", "App.tsx?platform=ios#component"] {
    let output =
      transform(filename, source, RuntimeTarget::HermesV1, Some(ReactCompilerOptions::default()))
        .unwrap();
    assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
    assert!(!output.code.contains("as number"), "{}", output.code);
    assert!(!output.code.contains(": number"), "{}", output.code);
    assert!(output.output_module_kind.is_none());
  }
}

#[test]
fn preserves_type_annotations_required_by_native_component_codegen() {
  let output = transform(
    "NativeView.tsx",
    r"import type { ViewProps, HostComponent } from 'react-native';
import codegenNativeComponent from 'react-native/Libraries/Utilities/codegenNativeComponent';
interface NativeProps extends ViewProps { enabled?: boolean; }
export default codegenNativeComponent<NativeProps>('CompilerProbe') as HostComponent<NativeProps>;
export function App({ value }: { value: number }) {
  const doubled = value * 2;
  return <View value={doubled}/>;
}",
    RuntimeTarget::HermesV1,
    Some(ReactCompilerOptions::default()),
  )
  .unwrap();
  assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
  assert!(output.code.contains("__INTERNAL_VIEW_CONFIG"), "{}", output.code);
  assert!(output.code.contains("enabled: true"), "{}", output.code);
  assert!(!output.code.contains("codegenNativeComponent<"), "{}", output.code);
}

#[test]
fn leaves_compiler_disabled_when_configuration_is_absent() {
  let output = transform("App.jsx", COMPONENT, RuntimeTarget::HermesV1, None).unwrap();
  assert!(!output.code.contains("react/compiler-runtime"), "{}", output.code);
  assert!(output.warnings.is_empty(), "{:?}", output.warnings);
}

#[test]
fn all_mode_compiles_without_infer_mode_react_patterns() {
  let config = ReactCompilerOptions { compilation_mode: Some("all".into()), ..Default::default() };
  let output = transform(
    "utility.js",
    "export function compute(value) { return { value }; }",
    RuntimeTarget::HermesV1,
    Some(config),
  )
  .unwrap();
  assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
}

#[test]
fn annotation_mode_honors_opt_in_directives_without_jsx() {
  let config =
    ReactCompilerOptions { compilation_mode: Some("annotation".into()), ..Default::default() };
  let output = transform(
    "utility.js",
    "export function compute(value) { 'use memo'; return { value }; }",
    RuntimeTarget::HermesV1,
    Some(config),
  )
  .unwrap();
  assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
}

#[derive(Debug)]
struct AppFilesOnly;

impl ReactCompilerFilter for AppFilesOnly {
  fn matches(&self, filename: &str, _cwd: &str) -> bool {
    filename.ends_with("App.jsx")
  }
}

#[test]
fn filters_the_original_input_filename() {
  let config = ReactCompilerOptions { filter: Some(Arc::new(AppFilesOnly)), ..Default::default() };
  for (filename, compiled) in [("/app/App.jsx", true), ("/app/vendor.jsx", false)] {
    let output =
      transform(filename, COMPONENT, RuntimeTarget::HermesV1, Some(config.clone())).unwrap();
    assert_eq!(output.code.contains("react/compiler-runtime"), compiled, "{}", output.code);
  }
}

#[test]
fn default_panic_threshold_keeps_unsupported_var_components_buildable() {
  let output = transform(
    "App.jsx",
    VAR_COMPONENT,
    RuntimeTarget::HermesV1,
    Some(ReactCompilerOptions::default()),
  )
  .unwrap();
  assert!(!output.code.contains("react/compiler-runtime"), "{}", output.code);
}

#[test]
fn strict_panic_threshold_fails_on_unsupported_var_components() {
  let config =
    ReactCompilerOptions { panic_threshold: Some("all_errors".into()), ..Default::default() };
  let error = transform("App.jsx", VAR_COMPONENT, RuntimeTarget::HermesV1, Some(config))
    .unwrap_err()
    .to_string();
  assert!(error.contains("App.jsx"), "{error}");
  assert!(error.contains("var"), "{error}");
}

#[test]
fn no_emit_keeps_lint_errors_nonfatal() {
  let config = ReactCompilerOptions { no_emit: Some(true), ..Default::default() };
  let output = transform("App.jsx", VAR_COMPONENT, RuntimeTarget::HermesV1, Some(config)).unwrap();
  assert!(!output.code.contains("react/compiler-runtime"), "{}", output.code);
  assert!(output.warnings.iter().any(|warning| warning.contains("var")), "{:?}", output.warnings);
}

#[test]
fn lint_output_mode_keeps_lint_errors_as_warnings() {
  let config = ReactCompilerOptions { output_mode: Some("lint".into()), ..Default::default() };
  let output = transform("App.jsx", VAR_COMPONENT, RuntimeTarget::HermesV1, Some(config)).unwrap();
  assert!(!output.code.contains("react/compiler-runtime"), "{}", output.code);
  assert!(output.warnings.iter().any(|warning| warning.contains("var")), "{:?}", output.warnings);
}

#[test]
fn lint_output_mode_honors_strict_panic_threshold() {
  let config = ReactCompilerOptions {
    output_mode: Some("lint".into()),
    panic_threshold: Some("all_errors".into()),
    ..Default::default()
  };
  let error = transform("App.jsx", VAR_COMPONENT, RuntimeTarget::HermesV1, Some(config))
    .unwrap_err()
    .to_string();
  assert!(error.contains("App.jsx"), "{error}");
  assert!(error.contains("var"), "{error}");
}

#[test]
fn disabled_compilation_skips_strict_diagnostics() {
  let config = ReactCompilerOptions {
    filter: Some(Arc::new(AppFilesOnly)),
    panic_threshold: Some("all_errors".into()),
    ..Default::default()
  };
  let output =
    transform("Vendor.jsx", VAR_COMPONENT, RuntimeTarget::HermesV1, Some(config)).unwrap();
  assert!(!output.code.contains("react/compiler-runtime"), "{}", output.code);
  assert!(output.warnings.is_empty(), "{:?}", output.warnings);
}

#[test]
fn compiler_output_preserves_worklet_transformation() {
  let transformer = Transformer::new(
    None,
    TransformerOptions {
      swc: Some(SwcConfig {
        react: ReactConfig {
          compiler: Some(ReactCompilerOptions::default()),
          ..Default::default()
        },
        ..Default::default()
      }),
      worklets: Some(WorkletsConfig {
        options: SwcWorkletsOptions {
          plugin_version: "0.7.4".into(),
          disable_source_maps: true,
          ..Default::default()
        },
      }),
      ..Default::default()
    },
  )
  .unwrap();
  let output = transformer
    .transform(TransformInput {
      cwd: "",
      filename: "App.jsx",
      code: "const offset = 40; export function compute(value) { 'worklet'; return offset + value; } export function App({ value }) { const doubled = value * 2; return <View value={doubled}/>; }",
      module_kind: None,
    })
    .unwrap();
  assert!(output.code.contains("react/compiler-runtime"), "{}", output.code);
  assert!(output.code.contains("__workletHash"), "{}", output.code);
  assert!(output.code.contains("__initData"), "{}", output.code);
}

#[test]
fn reuses_nested_compiler_configuration_across_transforms() {
  let transformer = Transformer::new(
    None,
    TransformerOptions {
      swc: Some(SwcConfig {
        react: ReactConfig {
          compiler: Some(ReactCompilerOptions::default()),
          ..Default::default()
        },
        ..Default::default()
      }),
      ..Default::default()
    },
  )
  .unwrap();
  let input = TransformInput { cwd: "", filename: "App.jsx", code: COMPONENT, module_kind: None };
  let first = transformer.transform(input).unwrap();
  assert!(first.code.contains("react/compiler-runtime"), "{}", first.code);
  let second = transformer.transform(input).unwrap();
  assert_eq!(first.code, second.code);
  assert_eq!(first.warnings, second.warnings);
}

#[test]
fn compiler_preserves_rebuilt_function_and_parenthesized_declarator_ranges() {
  #[derive(Default)]
  struct Ranges {
    functions: Vec<Span>,
    declarators: Vec<(String, Span)>,
  }
  impl Visit for Ranges {
    fn visit_function(&mut self, function: &Function) {
      self.functions.push(function.span);
      function.visit_children_with(self);
    }

    fn visit_arrow_expr(&mut self, function: &ArrowExpr) {
      self.functions.push(function.span);
      function.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, declaration: &VarDeclarator) {
      if let Pat::Ident(binding) = &declaration.name {
        self.declarators.push((binding.id.sym.to_string(), declaration.span));
      }
      declaration.visit_children_with(self);
    }
  }

  // All three rebuilt function forms must preserve SWC's absolute byte positions.
  // The cast's closing parenthesis belongs to the original declarator range.
  for source in [
    "export function Component(value: unknown) { const y = (value as string); return y; }",
    "export const Component = function(value: unknown) { const y = (value as string); return y; };",
    "export const Component = (value: unknown) => { const y = (value as string); return y; };",
  ] {
    GLOBALS.set(&Globals::new(), || {
      let cm = SourceMap::default();
      cm.new_source_file(swc_common::FileName::Anon.into(), "// preceding module\n");
      let file = cm.new_source_file(swc_common::FileName::Anon.into(), source.to_string());
      let program = parse_file_as_program(
        &file,
        Syntax::Typescript(TsSyntax::default()),
        swc_ecma_ast::EsVersion::latest(),
        None,
        &mut Vec::new(),
      )
      .unwrap();
      let mut original = Ranges::default();
      program.visit_with(&mut original);
      let result = swc_ecma_react_compiler::transform(
        &program,
        swc_ecma_react_compiler::SourceType::from_program(&program).with_typescript(true),
        source,
        None,
        swc_ecma_react_compiler::PluginOptions {
          compilation_mode: "all".into(),
          ..swc_ecma_react_compiler::default_plugin_options()
        },
      );
      assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
      let mut compiled = Ranges::default();
      result.program.expect("component must be compiled").visit_with(&mut compiled);
      assert_eq!(compiled.functions, original.functions, "{source}");
      let original_y = original.declarators.iter().find(|(name, _)| name == "y").unwrap();
      let compiled_y = compiled.declarators.iter().find(|(name, _)| name == "y").unwrap();
      assert_eq!(compiled_y, original_y, "{source}");
    });
  }
}
