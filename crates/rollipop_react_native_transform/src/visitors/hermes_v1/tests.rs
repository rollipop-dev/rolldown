use swc_common::{FileName, GLOBALS, Globals, Mark, SourceMap, sync::Lrc};
use swc_ecma_ast::{EsVersion, Pass, Program};
use swc_ecma_codegen::{Config, Emitter, text_writer::JsWriter};
use swc_ecma_parser::{EsSyntax, Syntax, parse_file_as_program};
use swc_ecma_transformer::hook_pass;
use swc_ecma_transforms_base::{fixer::fixer, hygiene::hygiene, resolver};

use super::hermes_v1_fixes;

fn transform(source: &str, fused: bool, dummy_spans: bool) -> String {
  GLOBALS.set(&Globals::new(), || {
    let cm = Lrc::<SourceMap>::default();
    let file = cm.new_source_file(FileName::Custom("input.js".into()).into(), source.to_string());
    let mut program: Program = parse_file_as_program(
      &file,
      Syntax::Es(EsSyntax { decorators: true, ..Default::default() }),
      EsVersion::default(),
      None,
      &mut Vec::new(),
    )
    .unwrap();
    if dummy_spans {
      use swc_ecma_visit::{VisitMut, VisitMutWith};
      struct ClearSpans;
      impl VisitMut for ClearSpans {
        fn visit_mut_span(&mut self, span: &mut swc_common::Span) {
          *span = swc_common::DUMMY_SP;
        }
      }
      program.visit_mut_with(&mut ClearSpans);
    }
    resolver(Mark::new(), Mark::new(), false).process(&mut program);
    if fused {
      hook_pass(hermes_v1_fixes()).process(&mut program);
    } else {
      (
        swc_react_native::fix_async_arrow_non_simple_params(),
        swc_react_native::fix_super_in_object_accessor(),
        swc_react_native::fix_class_in_finally(),
      )
        .process(&mut program);
    }
    (hygiene(), fixer(None)).process(&mut program);
    let mut output = Vec::new();
    Emitter {
      cfg: Config::default(),
      cm: Lrc::clone(&cm),
      comments: None,
      wr: JsWriter::new(cm, "\n", &mut output, None),
    }
    .emit_program(&program)
    .unwrap();
    String::from_utf8(output).unwrap()
  })
}

#[test]
fn matches_upstream_fix_visitors() {
  let cases = [
    r"const _p = 'outer';
const read = async (value = 1, { name }, [first]) => value + name + first + _p;
const collect = async (...items) => items.length;
const nested = async (outer = async ({ value }) => value) => outer;",
    r"const obj = {
  get value() { return () => super.value; },
  set value(next) { super.value = next; },
  get outer() { return { get inner() { return super.inner; } }; },
  get isolated() { return function() { return { get value() { return super.value; } }; }; },
  method() { return super.value; }
};",
    r"try { class InTry {} } catch (error) { class InCatch {} } finally {
  class Decl extends Base { method() { class Local {} } }
  const Expr = class Named {};
  use(class {});
  function nested() { class Local {} try {} finally { class Inner {} } }
  const arrow = () => { class Local {} try {} finally { class Inner {} } };
  const obj = {
    method() { class Local {} },
    get value() { class Local {} return Local; },
    set value(next) { class Local {} }
  };
  try {} catch (error) { class NestedCatch {} } finally { class NestedFinally {} }
  class AfterNested {}
}
class AfterFinally {}",
    r"try {} finally {
  @decorate class Decorated {}
  class Outer extends (class Base {}) {
    field = class Field {};
    static { class StaticLocal {} }
  }
}",
    r"try {} finally {
  class Example {
    field = async ({ value } = { value: 1 }) => value;
    method() { return { get value() { return super.value; } }; }
  }
  const Expr = class { field = async (...values) => values; };
}",
    r"export const read = async ({ value } = { value: 1 }) => value;
try {} finally { class Local {} use(Local); }",
  ];
  for source in cases {
    for dummy_spans in [false, true] {
      assert_eq!(
        transform(source, true, dummy_spans),
        transform(source, false, dummy_spans),
        "dummy spans: {dummy_spans}, source:\n{source}"
      );
    }
  }
}
