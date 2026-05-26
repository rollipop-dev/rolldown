use rollipop_react_native_transform::{
  ModuleConfig, SwcConfig, SwcModuleType, TransformInput, Transformer, TransformerOptions,
  WorkletsConfig,
};

fn options_with_globals(module: Option<ModuleConfig>) -> TransformerOptions {
  TransformerOptions {
    swc: Some(SwcConfig {
      globals: std::iter::once(("import.meta.hot".to_string(), "undefined".to_string())).collect(),
      module,
      ..Default::default()
    }),
    ..Default::default()
  }
}

fn transform_with_globals(code: &str, module: Option<ModuleConfig>) -> String {
  let transformer = Transformer::new(None, options_with_globals(module)).unwrap();
  transformer
    .transform(TransformInput { filename: "input.js", code, module_kind: None })
    .unwrap()
    .code
}

#[test]
fn replaces_global_expressions() {
  let code = transform_with_globals("console.log(import.meta.hot);\n", None);

  assert_eq!(code, "console.log(undefined);\n");
}

#[test]
fn replaces_global_expressions_before_commonjs_transform() {
  let code = transform_with_globals(
    "export const hot = import.meta.hot;\n",
    Some(ModuleConfig { r#type: SwcModuleType::CommonJs }),
  );

  assert_eq!(
    code,
    r#""use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
Object.defineProperty(exports, "hot", {
    enumerable: true,
    get: function() {
        return hot;
    }
});
var hot = undefined;
"#
  );
}

#[test]
fn replaces_global_expressions_after_worklets_transform() {
  let mut options = options_with_globals(None);
  options.worklets = Some(WorkletsConfig::default());
  let transformer = Transformer::new(None, options).unwrap();

  let code = transformer
    .transform(TransformInput {
      filename: "input.js",
      code: "function getHot() { 'worklet'; return import.meta.hot; }\n",
      module_kind: None,
    })
    .unwrap()
    .code;

  assert!(
    code.contains("return import.meta.hot;"),
    "worklet code must be serialized before globals: {code}"
  );
  assert!(
    code.contains("return undefined;"),
    "outer generated function must still receive globals: {code}"
  );
}
