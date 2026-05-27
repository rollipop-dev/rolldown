use rollipop_react_native_transform::{
  RuntimeTarget, TransformInput, Transformer, TransformerOptions,
};

fn transform(runtime_target: RuntimeTarget, code: &str) -> String {
  let transformer =
    Transformer::new(None, TransformerOptions { runtime_target, ..Default::default() }).unwrap();
  transformer
    .transform(TransformInput { filename: "input.js", code, module_kind: None })
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
fn preserves_private_field_updates_after_object_freeze_for_hermes_targets() {
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

    assert!(code.contains("WeakMap"), "private field did not use spec storage: {code}");
    assert!(
      !code.contains("Object.defineProperty(this, _hasPlaceholderTypes"),
      "private field was lowered to a frozen own property: {code}"
    );
  }
}

#[test]
fn preserves_public_field_assignment_for_hermes_targets() {
  for runtime_target in [RuntimeTarget::Hermes, RuntimeTarget::HermesV1] {
    let code = transform(
      runtime_target,
      r"class Type {
  value = 1;
}
",
    );

    assert!(code.contains("this.value = 1"), "public field was not assigned directly: {code}");
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
