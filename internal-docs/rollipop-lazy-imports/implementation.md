# Rollipop lazy imports & cached interop namespaces — Implementation

> The rationale and principles behind this live in [design.md](./design.md).

## Summary

Two runtime helpers (`__rollipop_require__.i`, `__rollipop_require__.l`) plus
changes to what the Rollipop module finalizer emits for import declarations.
`lazyImports` is gated by `is_lazy_imports_enabled` (`crates/rolldown/src/rollipop/mod.rs`):
the option has to be on, the format has to be `rollipop`, and dev mode has to be
off.

## Generated code

| Import                            | Before                    | Now (lazy off)      | Now (lazy on)     |
| --------------------------------- | ------------------------- | ------------------- | ----------------- |
| ESM → ESM named/default           | `var a = r(5)`            | `var a = r(5)`      | `var a = l(5)`    |
| ESM → CJS (Babel interop)         | `var a = r.t(r(5))`       | `var a = r.i(5)`    | `var a = l(5, 0)` |
| ESM → CJS (Node interop, `.mjs`)  | `var a = r.t(r(5), 1)`    | `var a = r.i(5, 1)` | `var a = l(5, 1)` |
| inline access / `import()` of CJS | `r.t(r(5)).x`             | `r.i(5).x`          | same as off       |
| bare, namespace, `export *`       | `var a = r(5)` / `r.t(…)` | `r(5)` / `r.i(5)`   | same as off       |

`r` is `__rollipop_require__`. `l` is `__rollipop_lazy__`, a fifth factory
parameter that is only declared on modules with static import records
(`ecmascript/format/rollipop.rs`). The runtime passes `__rollipop_require__.l`
for it (`rollipop_runtime(lazy)` patches the `factory.call` line). A parameter
instead of `r.l(5)` keeps the Hermes bytecode for a lazy declaration identical
to an eager one. In the example app the lazy bundle grew by +0.04% HBC, all of
it runtime helpers.

## Finalizer (`module_finalizers/rollipop.rs`)

- `namespace_expr_for_module` emits `r(id)` / `r.i(id[, 1])` for every bundled
  importee, at all call sites: import bindings, inline accesses
  (`inline_import_access_for_symbol_ref`) and dynamic imports. Externals keep
  `r.t(r.e(id))`.
- Lazy mode bookkeeping (`LazyImportsState`, present only when lazy mode is on):
  - `candidates`: binding name → (importee, mode). Recorded when
    `create_import_binding_stmt` emits a declaration for a bundled importee.
    There is one declaration per importee per module (`generated_imports`).
  - `eager_targets`: importees reached through a bare import, a namespace
    import, `export {} from`, `export *` / `export * as`, or a binding whose
    imported specifier resolves to a whole namespace (`Specifier::Star`, set in
    `create_import_binding_for_module`).
  - `read_bindings`: binding names that are actually emitted as reads.
    These come from identifier rewrites in `exit_expression`, re-export getters
    and `create_define_exports_stmt`.
- `exit_program` calls `apply_lazy_imports` after `create_define_exports_stmt`.
  It rewrites the initializer of each top-level candidate declaration that is
  read and not eager to `__rollipop_lazy__(id[, mode])`. Deciding at the end
  makes "eager wins" independent of statement order, and it keeps imports
  retained only for side effects (bindings tree-shaken away) eager.
- `__rollipop_lazy__` is in `FACTORY_PARAM_NAMES`, so a user binding with that
  name gets renamed like the other factory parameters.
- `lazyImports.exclude` is evaluated once per build in
  `stages/generate_stage/finalize_rollipop_modules.rs`
  (`NormalizedLazyImportsOptions::is_excluded`, with module ids against `cwd`).
  The resulting `FxHashSet<ModuleIdx>` is passed to every finalizer as
  `RollipopAstFinalizerContext::lazy_import_excluded`, and `apply_lazy_imports`
  keeps those importees eager.

## Runtime

`runtime/rollipop-runtime.js` (always emitted):

- `.t(mod, nodeMode, target?)`: the interop wrapper. When given `target`, it
  sets that object's prototype and fills it in place.
- `.i(id, nodeMode, target?)`: cached interop namespace on the module record
  (`ns` / `nodeNs`, invalidated when `nsSource !== module.exports`). It caches
  only when `record.loaded === true`. Records that are still loading (cycles)
  or that came from the HMR holder fall back to a fresh `.t`.

`runtime/rollipop-runtime-lazy.js` (appended when lazy mode is on):

- `.l(id, mode)`: mode `2` (omitted) means ESM, `0` Babel, `1` Node. If the
  module already has a cache record, or there is no `Proxy`, it returns the
  eager value. Otherwise it returns a facade, shared per (module, mode), whose
  prototype is `new Proxy(state, handler)`.
- Proxy `get` / `has` call `__rollipop_lazy_resolve`, which requires the
  module:
  - ESM: once the module has loaded, it copies the export descriptors onto the
    facade and sets the facade's real prototype.
  - CJS: it calls `.i(id, mode, facade)`. When the facade is the first
    namespace for that module, `.t` builds it in place and the record caches
    the facade itself, so there is no copy. Otherwise it copies from the cached
    namespace.
  - While the module is still loading, or after its factory threw, the facade
    keeps its `Proxy` prototype. Reads go through to the partial exports, or
    rethrow the cached error.

After resolution, the facade is a plain object with own accessors and an
ordinary prototype. Reads are then indistinguishable from the eager path.

## Tests

`crates/rolldown/tests/rolldown/function/format/rollipop/`:

- `lazy_imports`: evaluation order for every eager and lazy case, re-export
  getters, CJS interop, cycles, and error rethrow on repeated reads.
- `lazy_imports_minify`: minified output; the parameter is declared only on
  modules with imports.
- `lazy_imports_no_proxy`: eager fallback without `Proxy`.
- `lazy_imports_sourcemap{,_minify}`: lazily evaluated modules keep exact
  original line and column mappings.
- `lazy_imports_exclude` (plus `packages/rolldown/tests/rollipop-lazy-imports.test.ts`
  for RegExp/glob patterns through the JS API): `exclude` keeps a
  load-time-registering module eager.
- `lazy_imports_namespace_binding`: whole-namespace bindings through a removed
  barrel stay eager, so they can be enumerated and compared.
- `topics/hmr/rollipop_lazy_imports_dev_mode`: dev mode ignores the flag.
- `cjs_interop_namespace`: one namespace per module and interop mode, shared
  with `import()`; not cached during a cycle; `module.exports` reassignment;
  `__esModule` handling.

## Related

- [design.md](./design.md)
- `crates/rolldown/src/ecmascript/format/rollipop.rs`: chunk renderer and
  factory signatures
