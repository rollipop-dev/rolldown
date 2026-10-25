# Native SWC compatibility passes

`crates/rollipop_react_native_transform/src/compat.rs` composes
`swc_ecma_transformer::transform_hook` with local hooks and executes them through
`hook_pass`. Each group shares one AST traversal and one `TraverseCtx`.
The existing SWC dependency graph already supplies the transformer; its version
stays aligned with the compatibility crates.

## Groups and ordering

Legacy Hermes runs two compatibility groups around `class_properties`:

1. Regular expressions, tagged-template caching and static blocks.
2. Private-in checks, object rest/spread and async functions/generators.

Static blocks must become synthetic private fields before class-properties
lowering. Keep that pass between the groups. Class lowering, parameters,
destructuring and block scoping still run afterward so they process generated
code. The second group uses SWC's ordering and declaration injection for object
rest/spread and async lowering instead of separate whole-program walks.

Hermes V1 runs all three bug fixes, tagged-template caching and async lowering
in one shared traversal; block scoping follows it. V1 preserves native regular
expressions, object rest/spread and private fields. The previous no-op regexp
pass is omitted.

The bug-fix hooks in `src/visitors/hermes_v1/` port
`swc_react_native_hermes_v1_fixes` 0.3.2. The production dependency enables only
codegen and Worklets; the original fixes remain a development dependency for
output comparisons. Keep the ports aligned when upgrading that crate.

- Async arrow parameters are rewritten on entry, before SWC's async hook sees
  them. Rest parameters create a synchronous outer arrow and an async inner
  arrow. Generated nodes enter the same traversal. The upstream local name
  scan remains for arrows with non-simple parameters to avoid generated-name
  collisions; it is not another whole-program transformation pass.
- Object accessor hooks collect direct `super` references during the shared
  traversal, then rewrite the accessor key on exit. Arrows inherit this state;
  ordinary functions, methods, classes and nested accessors establish boundaries.
  Only the accessor body activates collection, not its key or parameters. This
  replaces the upstream accessor-body finder walks.
- Class-in-finally hooks track entry and exit of finalizer blocks and reset the
  depth across functions, arrows, object methods/accessors and class member lists.
  Class decorators and superclass expressions retain the surrounding depth.
  Generated IIFE arrows reset it before visiting the wrapped class. Finalizer
  blocks and accessor bodies are identified by addresses valid during their
  containing node's traversal, not spans, because generated nodes can share
  `DUMMY_SP`. No pointers are dereferenced.

The bug-fix hooks enter before SWC's transformer hooks and exit afterward.
This lets parameter rewrites precede async lowering and lets accessor hooks
observe lexical `super` before async lowering moves it into generated functions.

`TemplateLiteralCaching` implements `VisitMutHook<TraverseCtx>`. It rewrites
expressions on exit and adds cache declarations on module/script exit, after
all call sites have been seen. `CompositeHook` executes exit hooks in reverse
order, so the template hook is its second member: caching runs before SWC
rewrites an enclosing expression. Its generated identity arrow and cache
bindings need only the later block-scoping pass. Generated tagged templates
must not be revisited by the caching hook.

These groups do not replace the full native pipeline with one walk. React
Compiler, resolver, native-component codegen, Flow field removal, type stripping,
WASM plugins, JSX and Worklets retain their ordering boundaries. In particular,
codegen needs types and Worklets needs stripped types and lowered JSX. See
[React Compiler implementation](../react-native-compiler/implementation.md).

## Validation

`tests/hermes.rs` covers both presets, preserved JSX, nested templates and async
arrows in static blocks, script directives, rest-parameter wrappers, and object
rest/spread inside async generators. `tests/hermes_v1.rs` covers the upstream
bug fixes composed with async lowering and tagged-template caching inside a
class-in-finally wrapper. The hook unit tests compare output with the original
three upstream visitors for scope boundaries, nested accessors, generated-name
collisions, rest parameters and decorated classes, both with source spans and
with all spans set to `DUMMY_SP`. The remaining transform tests cover Compiler,
globals and CommonJS integration.
