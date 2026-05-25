use rollipop_react_native_transform::{
  ModuleConfig, ReactConfig, ReactRuntime, SwcConfig, SwcModuleType, TransformInput, Transformer,
  TransformerOptions,
};

fn transform_with_swc_config(filename: &str, code: &str, swc: SwcConfig) -> String {
  let transformer =
    Transformer::new(None, TransformerOptions { swc: Some(swc), ..Default::default() }).unwrap();
  transformer.transform(TransformInput { filename, code, module_kind: None }).unwrap().code
}

#[test]
fn defaults_to_unambiguous_module_type() {
  let code = transform_with_swc_config(
    "input.js",
    r#"import { foo } from "foo";
export const value = foo;
"#,
    SwcConfig::default(),
  );

  assert_eq!(
    code,
    r#"import { foo } from "foo";
export var value = foo;
"#
  );
}

#[test]
fn preserves_commonjs_expressions_by_default() {
  let code = transform_with_swc_config(
    "input.js",
    r#"var foo = require("foo");
module.exports = foo;
"#,
    SwcConfig::default(),
  );

  assert_eq!(
    code,
    r#"var foo = require("foo");
module.exports = foo;
"#
  );
}

#[test]
fn transforms_es_modules_to_commonjs() {
  let code = transform_with_swc_config(
    "input.js",
    r#"import { foo } from "foo";
export const value = foo;
"#,
    SwcConfig {
      module: Some(ModuleConfig { r#type: SwcModuleType::CommonJs }),
      ..Default::default()
    },
  );

  assert_eq!(
    code,
    r#""use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
Object.defineProperty(exports, "value", {
    enumerable: true,
    get: function() {
        return value;
    }
});
var _foo = require("foo");
var value = _foo.foo;
"#
  );
}

#[test]
fn injects_commonjs_interop_helpers() {
  let code = transform_with_swc_config(
    "input.js",
    r#"import foo from "foo";
console.log(foo);
"#,
    SwcConfig {
      module: Some(ModuleConfig { r#type: SwcModuleType::CommonJs }),
      ..Default::default()
    },
  );

  assert_eq!(
    code,
    r#""use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
var _foo = /*#__PURE__*/ _interop_require_default(require("foo"));
function _interop_require_default(obj) {
    return obj && obj.__esModule ? obj : {
        default: obj
    };
}
console.log(_foo.default);
"#
  );
}

#[test]
fn lets_commonjs_pass_handle_typescript_import_export_assign() {
  let code = transform_with_swc_config(
    "input.ts",
    r#"import foo = require("foo");
export = foo;
"#,
    SwcConfig {
      module: Some(ModuleConfig { r#type: SwcModuleType::CommonJs }),
      ..Default::default()
    },
  );

  assert_eq!(
    code,
    r#""use strict";
var foo = require("foo");
module.exports = foo;
"#
  );
}

#[test]
fn transforms_react_runtime_import_to_commonjs() {
  let code = transform_with_swc_config(
    "input.jsx",
    "export const node = <View />;\n",
    SwcConfig {
      react: ReactConfig { runtime: ReactRuntime::Automatic, ..Default::default() },
      module: Some(ModuleConfig { r#type: SwcModuleType::CommonJs }),
      ..Default::default()
    },
  );

  assert_eq!(
    code,
    r#""use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
Object.defineProperty(exports, "node", {
    enumerable: true,
    get: function() {
        return node;
    }
});
var _jsxruntime = require("react/jsx-runtime");
var node = /*#__PURE__*/ (0, _jsxruntime.jsx)(View, {});
"#
  );
}
