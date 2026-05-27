use rollipop_react_native_transform::{TransformInput, Transformer, TransformerOptions};

#[test]
fn lowers_for_await_inside_async_generator_for_hermes_v1() {
  let transformer = Transformer::new(None, TransformerOptions::default()).unwrap();
  let code = transformer
    .transform(TransformInput {
      filename: "input.js",
      code: r"export class Provider {
  async *stream(events) {
    while (true) {
      for await (const event of events) {
        yield event;
      }
    }
  }
}
",
      module_kind: None,
    })
    .unwrap()
    .code;

  assert!(!code.contains("for await"), "for-await remained after Hermes V1 lowering: {code}");
}
