use std::process::Command;

use rollipop_react_native_transform::{
  RuntimeTarget, TransformInput, Transformer, TransformerOptions,
};

fn execute(code: &str) -> String {
  let output = Command::new("node")
    .args(["--input-type=module", "--eval", code])
    .output()
    .expect("Node.js is required for Hermes transform execution tests");
  assert!(
    output.status.success(),
    "execution failed: {}\n{code}",
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8(output.stdout).unwrap()
}

fn assert_preserves_execution(source: &str) {
  let transformer = Transformer::new(
    None,
    TransformerOptions { runtime_target: RuntimeTarget::Hermes, ..Default::default() },
  )
  .unwrap();
  let output = transformer
    .transform(TransformInput { cwd: "", filename: "input.js", code: source, module_kind: None })
    .unwrap();
  let expected = execute(source);
  assert!(!expected.is_empty(), "fixture must report its result");
  assert_eq!(execute(&output.code), expected);
}

#[test]
fn preserves_static_blocks_private_brands_regexp_and_template_caches() {
  assert_preserves_execution(
    r"
const events = [];
const sites = [];
function tag(strings, value) {
  sites.push(strings);
  return strings[0] + value;
}
function label(value) { return tag`value:${value}`; }
class Store {
  static first = label(1);
  static {
    events.push(this.first);
    this.Inner = class Inner {
      #read() { return this.value; }
      constructor(value) { this.value = value; }
      has(other) { return #read in other; }
      async read({ value, ...rest }) {
        return { ...rest, value: await this.#read() + value };
      }
    };
    events.push(label(2), sites[0] === sites[1]);
  }
  static last = /(?<word>a)\k<word>/u.exec('aa').groups.word;
}
async function main() {
  const inner = new Store.Inner(3);
  events.push(inner.has(inner), inner.has({}), Store.last);
  events.push(await inner.read({ value: 4, name: 'retained' }));
  console.log(JSON.stringify(events));
}
main().catch(error => { console.error(error); process.exitCode = 1; });
",
  );
}

#[test]
fn preserves_async_generator_object_rest_and_spread_side_effects() {
  assert_preserves_execution(
    r"
const events = [];
const symbol = Symbol('retained');
const source = {
  get selected() { events.push('selected'); return 2; },
  get retained() { events.push('retained'); return 3; },
  [symbol]: 4
};
async function* stream(input) {
  const { [await Promise.resolve('selected')]: selected, ...rest } = input;
  yield { ...rest, selected };
  for await (const value of [Promise.resolve(5), 6]) {
    yield { ...rest, selected: value };
  }
}
async function main() {
  for await (const { selected, ...rest } of stream(source)) {
    events.push([selected, rest.retained, rest[symbol]]);
  }
  console.log(JSON.stringify(events));
}
main().catch(error => { console.error(error); process.exitCode = 1; });
",
  );
}

#[test]
fn preserves_async_super_parameters_and_loop_closures() {
  assert_preserves_execution(
    r"
class Base {
  async read(value) { return this.base + value; }
}
class Derived extends Base {
  constructor(...args) { super(); this.base = args[0]; }
  async read({ selected = 2, ...rest } = { selected: 3, other: 4 }) {
    const callbacks = [];
    for (let index = 0; index < 3; index++) {
      const value = await super.read(selected + index);
      callbacks.push(() => ({ ...rest, value }));
    }
    return callbacks.map(callback => callback());
  }
}
async function main() {
  const instance = new Derived(10);
  console.log(JSON.stringify([Derived.prototype.read.length, await instance.read()]));
}
main().catch(error => { console.error(error); process.exitCode = 1; });
",
  );
}
