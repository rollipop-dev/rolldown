use rollipop_react_native_transform::{TransformInput, Transformer, TransformerOptions};

fn transform(code: &str) -> String {
  let transformer = Transformer::new(None, TransformerOptions::default()).unwrap();
  transformer
    .transform(TransformInput { filename: "input.js", code, module_kind: None })
    .unwrap()
    .code
}

#[test]
fn preserves_native_private_fields_for_hermes_v1() {
  let code = transform(
    r"class FrozenCache {
  #cache = null;
  constructor() {
    Object.freeze(this);
  }
  read() {
    return (this.#cache ??= 1);
  }
}
",
  );

  assert!(code.contains("#cache = null;"), "private field was lowered: {code}");
  assert!(code.contains("this.#cache"), "private field access was lowered: {code}");
  assert!(!code.contains("Object.defineProperty"), "private field became an own property: {code}");
}

#[test]
fn lowers_only_class_static_blocks_for_hermes_v1() {
  let code = transform(
    r"class Registry {
  value = 1;
  #secret = 2;
  static {
    this.ready = true;
  }
}
",
  );

  assert!(code.contains("value = 1;"), "public field was lowered: {code}");
  assert!(code.contains("#secret = 2;"), "private field was lowered: {code}");
  assert!(!code.contains("static {"), "static block remained: {code}");
  assert!(code.contains("this.ready = true"), "static block body was lost: {code}");
}

#[test]
fn applies_hermes_v1_fix_passes_before_compat_lowering() {
  let code = transform(
    r"const read = async (value = 1, { name }) => value + name;
const obj = {
  get value() {
    return super.value;
  }
};
try {
  work();
} finally {
  class Foo {}
  use(Foo);
}
",
  );

  assert!(code.contains("var value ="), "async arrow params were not rewritten: {code}");
  assert!(code.contains("var { name }"), "async arrow destructuring was not rewritten: {code}");
  assert!(code.contains("get [\"value\"]"), "super object accessor key was not rewritten: {code}");
  assert!(code.contains("var Foo ="), "class in finally was not rewritten: {code}");
  assert!(code.contains("return Foo;"), "class in finally wrapper was not emitted: {code}");
}
