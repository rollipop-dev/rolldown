use rollipop_react_native_transform::{
  RuntimeTarget, TransformInput, Transformer, TransformerOptions,
};

fn transform(runtime_target: RuntimeTarget, code: &str) -> String {
  let transformer =
    Transformer::new(None, TransformerOptions { runtime_target, ..Default::default() }).unwrap();
  transformer
    .transform(TransformInput { cwd: "", filename: "input.js", code, module_kind: None })
    .unwrap()
    .code
}

#[test]
fn lowers_class_static_blocks_for_hermes() {
  let code = transform(
    RuntimeTarget::Hermes,
    r"class Registry {
  static {
    this.ready = true;
  }
}
",
  );

  assert!(!code.contains("static {"), "static block remained: {code}");
  assert!(code.contains("Registry.ready = true"), "static block body was lost: {code}");
}

#[test]
fn lowers_constructor_rest_parameters() {
  let code = transform(
    RuntimeTarget::Hermes,
    r"class Class {
  constructor(...args) {}
}
",
  );

  assert!(
    !code.contains("constructor(...args)"),
    "constructor rest parameters were not lowered: {code}"
  );
  assert!(
    code.contains("arguments.length"),
    "constructor rest parameters were not lowered: {code}"
  );
}

#[test]
fn applies_regexp_compat_for_legacy_hermes() {
  let code = transform(
    RuntimeTarget::Hermes,
    r"const named = /(?<word>a)\k<word>/u;
const astral = /\u{1F600}/u;
const ascii = /\p{ASCII}+/u;
const asciiCtor = new RegExp('\\p{ASCII}+', 'u');
",
  );

  assert!(code.contains("RegExp("), "regexp constructor was not emitted: {code}");
  assert!(code.contains("_wrap_reg_exp"), "named capture regexp wrapper was not emitted: {code}");
  assert!(code.contains("word: 1"), "named capture group mapping was not emitted: {code}");
  assert!(!code.contains(r"/(?<word>"), "named capture regexp literal remained: {code}");
  assert!(!code.contains(r"/\u{1F600}/u"), "unicode regexp literal remained: {code}");
  assert!(!code.contains(r"\p{ASCII}"), "unicode property regexp remained: {code}");
}

#[test]
fn preserves_native_private_field_updates_after_object_freeze_for_hermes_targets() {
  for runtime_target in [RuntimeTarget::Hermes, RuntimeTarget::HermesV1] {
    let code = transform(
      runtime_target,
      r"class Type {
  #hasPlaceholderTypes;

  constructor() {
    Object.freeze(this);
  }

  hasPlaceholder() {
    return this.#hasPlaceholderTypes ??= true;
  }
}
",
    );

    match runtime_target {
      RuntimeTarget::Hermes => {
        assert!(code.contains("WeakMap"), "private field did not use spec storage: {code}");
      }
      RuntimeTarget::HermesV1 => {
        assert!(code.contains("#hasPlaceholderTypes"), "private field was lowered: {code}");
        assert!(
          code.contains("this.#hasPlaceholderTypes"),
          "private field access was lowered: {code}"
        );
      }
    }
    assert!(
      !code.contains("Object.defineProperty(this, _hasPlaceholderTypes"),
      "private field was lowered to a frozen own property: {code}"
    );
  }
}

#[test]
fn preserves_native_public_field_assignment_for_hermes_targets() {
  for runtime_target in [RuntimeTarget::Hermes, RuntimeTarget::HermesV1] {
    let code = transform(
      runtime_target,
      r"class Type {
  value = 1;
}
",
    );

    assert!(code.contains("value = 1;"), "public field was lowered: {code}");
    assert!(
      !code.contains(r#"_define_property(this, "value""#),
      "public field was lowered with defineProperty: {code}"
    );
  }
}

#[test]
fn preserves_native_regexp_features_for_hermes_v1() {
  let code = transform(
    RuntimeTarget::HermesV1,
    r"const named = /(?<word>a)\k<word>/u;
const astral = /\u{1F600}/u;
const property = /\p{ASCII}+/u;
const indices = /a/d;
",
  );

  assert!(!code.contains("_wrap_reg_exp"), "named capture regexp wrapper was emitted: {code}");
  assert!(!code.contains("RegExp("), "regexp constructor was emitted: {code}");
  assert!(code.contains(r"/(?<word>"), "named capture regexp literal was rewritten: {code}");
  assert!(code.contains(r"/\p{ASCII}+/u"), "unicode property regexp literal was rewritten: {code}");
}

#[test]
fn caches_tagged_template_call_sites_for_hermes_targets() {
  for runtime_target in [RuntimeTarget::Hermes, RuntimeTarget::HermesV1] {
    let code = transform(runtime_target, r"const run = name => tag`hello ${name}`;");

    assert!(!code.contains("tag`hello"), "direct tagged template remained: {code}");
    assert!(code.contains("tag("), "tag call was not rewritten: {code}");
    assert!(code.contains("||"), "call site cache was not emitted: {code}");
  }
}

#[test]
fn caches_tagged_template_call_sites_without_rejecting_jsx() {
  let code = transform(
    RuntimeTarget::HermesV1,
    r"const View = ({ name }) => <Box title={tag`hello ${name}`} />;",
  );

  assert!(!code.contains("tag`hello"), "direct tagged template remained: {code}");
  assert!(code.contains("tag("), "tag call was not rewritten: {code}");
  assert!(code.contains("<Box"), "JSX was not preserved: {code}");
}

#[test]
fn lowers_nested_templates_and_async_functions_in_static_blocks() {
  for runtime_target in [RuntimeTarget::Hermes, RuntimeTarget::HermesV1] {
    let code = transform(
      runtime_target,
      r"class Registry {
  static {
    this.run = async (value = /\p{ASCII}+/u) => tag`outer ${tag`inner ${await value}`}`;
  }
}",
    );

    assert!(!code.contains("static {"), "static block remained: {code}");
    assert!(!code.contains("async ("), "async arrow remained: {code}");
    assert!(!code.contains("await value"), "await expression remained: {code}");
    assert!(!code.contains("tag`"), "nested tagged template remained: {code}");
    assert!(code.contains("tag("), "tag calls were lost: {code}");
    assert!(code.contains("outer "), "outer template was lost: {code}");
    assert!(code.contains("inner "), "inner template was lost: {code}");
  }
}

#[test]
fn lowers_object_rest_and_spread_inside_async_generators() {
  let code = transform(
    RuntimeTarget::Hermes,
    r"async function* stream({ first, ...rest }) {
  for await (const { value, ...other } of first) {
    yield { ...rest, ...other, value: await value };
  }
}",
  );

  assert!(!code.contains("for await"), "for-await remained: {code}");
  assert!(!code.contains("...rest"), "object rest or spread remained: {code}");
  assert!(!code.contains("...other"), "loop object rest or spread remained: {code}");
  assert!(code.contains("_object_without_properties"), "object rest helper was lost: {code}");
  assert!(code.contains("_object_spread"), "object spread helper was lost: {code}");
}

#[test]
fn caches_templates_in_scripts_and_keeps_directives_first() {
  let transformer = Transformer::new(None, TransformerOptions::default()).unwrap();
  let code = transformer
    .transform(TransformInput {
      cwd: "",
      filename: "input.cjs",
      code: r#""use strict"; const run = async (...values) => tag`outer ${tag`inner ${await values[0]}`}`;"#,
      module_kind: None,
    })
    .unwrap()
    .code;

  assert!(code.starts_with("\"use strict\";"), "directive moved behind cache bindings: {code}");
  assert!(!code.contains("tag`"), "script tagged template remained: {code}");
  assert!(!code.contains("await values"), "rest-param wrapper kept await: {code}");
  assert!(code.contains("outer "), "outer template was lost: {code}");
  assert!(code.contains("inner "), "inner template was lost: {code}");
}
