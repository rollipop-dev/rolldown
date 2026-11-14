# Rollipop lazy imports & cached interop namespaces — Implementation

> The rationale and principles behind this live in [design.md](./design.md).

## Summary

Two runtime helpers (`__rollipop_require__.i`, `__rollipop_require__.l`) plus
changes to what the Rollipop module finalizer emits for import declarations.
`lazyImports` is gated by `is_lazy_imports_enabled` (`crates/rolldown/src/rollipop/mod.rs`):
the option has to be on, the format has to be `rollipop`, and dev mode has to be
off.

## Generated code

| Import                           | Before                    | Now (lazy off)      | Now (lazy on)     |
| -------------------------------- | ------------------------- | ------------------- | ----------------- |
| ESM → ESM named/default          | `var a = r(5)`            | `var a = r(5)`      | `var a = l(5)`    |
| ESM → CJS, named reads only      | `var a = r.t(r(5))`       | `var a = r(5)`      | `var a = l(5)`    |
| ESM → CJS (Babel interop)        | `var a = r.t(r(5))`       | `var a = r.i(5)`    | `var a = l(5, 0)` |
| ESM → CJS (Node interop, `.mjs`) | `var a = r.t(r(5), 1)`    | `var a = r.i(5, 1)` | `var a = l(5, 1)` |
| reads of a lazy binding          | `a.x`                     | `a.x`               | `a.a.x`           |
| inline named access of CJS       | `r.t(r(5)).x`             | `r(5).x`            | same as off       |
| `import()` of CJS                | `r.t(r(5))`               | `r.i(5)`            | same as off       |
| bare, namespace, `export *`      | `var a = r(5)` / `r.t(…)` | `r(5)` / `r.i(5)`   | same as off       |

`r` is `__rollipop_require__`. `l` is `__rollipop_lazy__`, a fifth factory
parameter that is only declared on modules with static import records
(`ecmascript/format/rollipop.rs`). The runtime passes `__rollipop_require__.l`
for it (`rollipop_runtime(lazy)` patches the `factory.call` line). A parameter
instead of `r.l(5)` keeps the Hermes bytecode for a lazy declaration identical
to an eager one. Every read of a lazy binding loads the holder's `a` property
first; in the example app that is about 2,100 reads and +16 KB HBC (+1.45%)
over the earlier Proxy facade design.

## Finalizer (`module_finalizers/rollipop.rs`)

- `namespace_expr_for_module` emits `r(id)` / `r.i(id[, 1])` for every bundled
  importee: import bindings, inline accesses
  (`inline_import_access_for_symbol_ref`) and dynamic imports. Inline named
  accesses of CommonJS and named-only CommonJS bindings use `r(id)` instead
  (see "Direct CommonJS reads" below). Externals keep `r.t(r.e(id))`.
- Lazy mode bookkeeping (`LazyImportsState`, present only when lazy mode is on):
  - `candidates`: binding name → (importee, mode). Recorded when
    `create_import_binding_stmt` emits a declaration for a bundled importee.
    There is one declaration per importee per module (`generated_imports`).
  - `eager_targets`: importees reached through a bare import, a namespace
    import, `export {} from`, `export *` / `export * as`, or a binding whose
    imported specifier resolves to a whole namespace (`Specifier::Star`, set in
    `create_import_binding_for_module`). Records flagged
    `ImportRecordMeta::HasSideEffectImport` are eager too: the DCE step in
    `pre_process_ecma_ast.rs` merges `import './x'` into a named import of the
    same request, so the requests of side-effect-only imports are collected
    before it (`collect_side_effect_import_requests`, lazy mode only) and the
    flag is set in `ecma_module_view_factory.rs`. Both the `ImportDeclaration`
    and the `ExportFromDeclaration` arms check it, because DCE's
    `merge_import_export` can fold the merged import plus `export { a }` into
    `export { a } from './x'`.
  - `read_bindings`: binding names that are actually emitted as reads.
    These come from identifier rewrites in `exit_expression`, re-export getters
    and `create_define_exports_stmt`. A getter counts only when
    `is_export_getter_read` holds: tree-shaking is on, the module is not a
    user-defined entry, and the export is in `lazy_export_reads`.
- `lazy_export_reads` (`rollipop/export_reads.rs`, computed once per build in
  `finalize_rollipop_modules` when lazy mode is on) holds, per module, the
  exports that emitted code reads through that module. It follows codegen
  routing: an import of an included module goes through it.
  - Seeds: a named import whose local symbol is referenced by an included
    statement marks `(importee, name)`. For a namespace import, each member
    read `ns.x` (the first property of a `MemberExprRef`) marks `(importee, x)`.
  - Edges, walked with a worklist: a re-exported import makes `(importee, name)`
    read when the re-export is read, and `export * from` passes on names the
    module doesn't declare itself.
  - Pinned roots: importees of a static import record no named import uses (or
    one flagged `HasSideEffectImport`), of `require()` and `import()`, and of a
    namespace import whose local is referenced bare or re-exported.
    `pinned_barrels` adds every barrel reachable from them through re-exports
    and `export *`, and their entries are removed from the map. It also returns,
    per pinned module, the importees it re-exports (`PinnedReExportsMap`,
    passed as `lazy_pinned_re_exports`); `finalize_import_initializers` keeps those
    imports eager even when the module also reads them locally. So loading pkg
    in any of those ways keeps pkg's re-exported modules eager.
- Direct CommonJS reads (`DirectCjsReadsState`, absent in dev mode):
  - `candidates`: binding name → importee, for every generated binding of a
    bundled CommonJS importee (any interop).
  - `namespace_reads`: bindings read as the namespace itself or through
    `default` (`reads_namespace`), plus `export *` / `export * as` bindings.
    A namespace import read only as `ns.key` / `ns["key"]` with
    `key !== "default"` (`is_named_member_read`), including under `typeof` or
    `!`, does not count. Export getters
    count even when `is_export_getter_read` is false, to stay conservative.
  - Every read goes through `note_import_binding_read`, which feeds both this
    state and the lazy `read_bindings`.
  - `inline_import_access_for_symbol_ref` emits `r(id).key` instead of
    `r.i(id).key` for the same reason.
- `exit_program` calls `finalize_import_initializers` after
  `create_define_exports_stmt`. It rewrites the initializer of each top-level candidate declaration that is
  read and not eager to `__rollipop_lazy__(id[, mode])`, and returns those
  binding names. A CommonJS candidate with no namespace read
  (`reads_cjs_exports_directly`) omits the mode, so its holder resolves to
  `module.exports`. `LazyHolderAccessRewriter` then rewrites every member read of
  them (`import_x.foo`, `import_x["foo"]`, including the export getters) to
  `import_x.a.foo`. Generated reads of an import binding are always member
  expressions; namespace bindings, the only bare ones, are never lazy. Deciding at the end
  makes "eager wins" independent of statement order, and it keeps imports
  retained only for side effects (bindings tree-shaken away) eager. A CommonJS
  candidate that stays eager and has no namespace read becomes
  `__rollipop_require__(id)`.
- `__rollipop_lazy__` is in `FACTORY_PARAM_NAMES`, so a user binding with that
  name gets renamed like the other factory parameters.
- `lazyImports.exclude` is evaluated once per build in
  `stages/generate_stage/finalize_rollipop_modules.rs`
  (`NormalizedLazyImportsOptions::is_excluded`, with module ids against `cwd`).
  The resulting `FxHashSet<ModuleIdx>` is passed to every finalizer as
  `RollipopAstFinalizerContext::lazy_import_excluded`, and `finalize_import_initializers`
  keeps those importees eager.

## Runtime

`runtime/rollipop-runtime.js` (always emitted):

- `.t(mod, nodeMode, target?)`: the interop wrapper. Without `target` it is
  unchanged from before. With `target`, a namespace it built earlier for the
  same `mod`, it only adds the keys `target` doesn't have yet.
- `.i(id, nodeMode)`: cached interop namespace on the module record
  (`ns` / `nodeNs`, invalidated when `nsSource !== module.exports`). It caches
  only when `record.loaded === true`. Records that are still loading (cycles)
  or that came from the HMR holder fall back to a fresh `.t`. `nsKeyCount`
  holds the own key count of `module.exports` at the last call. When it
  changes, `.t` is called again with each cached namespace as `target`, which
  adds only the missing keys. A namespace that is no longer extensible (frozen
  or sealed by an importer) is dropped from the cache instead.

`runtime/rollipop-runtime-lazy.js` (appended when lazy mode is on):

- `.l(id, mode)` returns a holder, one per import declaration. Mode is omitted
  for ESM and for named-only CommonJS bindings (`r(id)`: the namespace or
  `module.exports`), `0` for Babel and `1` for Node interop (`.i(id, mode)`).
  - If the module already finished loading, the holder is `{ a: value }`.
  - Otherwise it is `{ get a() { … } }`. The getter loads the module and
    builds the namespace. Only when the module has finished loading does it
    replace itself with a data property (`Object.defineProperty(this, 'a', …)`).
  - A module still loading (a cycle) is read through to its partial exports on
    every read. A throw, from the factory or from interop construction, leaves
    the getter in place, so every later read throws again.
- This follows Rspack's optimized deferred namespace object (`.zO`). It needs no
  `Proxy`, copies no descriptors, and has no partially resolved state.

## Tests

`crates/rolldown/tests/rolldown/function/format/rollipop/`:

- `lazy_imports`: evaluation order for every eager and lazy case, re-export
  getters, CJS interop, cycles, and error rethrow on repeated reads.
- `lazy_imports_minify`: minified output; the parameter is declared only on
  modules with imports.
- `lazy_imports_no_proxy`: lazy holders work without `Proxy`.
- `lazy_imports_interop_failure`: an interop failure, immediate (`__esModule`
  getter throws) or after an earlier export descriptor was processed, rethrows
  on every read of a `default` binding; named-only bindings of the same modules
  skip interop and read the values.
- `lazy_imports_cjs_direct_reads`: named-only CommonJS lazy bindings are
  `l(id)` and resolve to `module.exports` (live values, keys added later,
  accessor exports with `module.exports` as `this`, Proxy exports, rethrown
  load errors); `default` bindings keep `l(id, 0)`.
- `lazy_imports_sourcemap{,_minify}`: lazily evaluated modules keep exact
  original line and column mappings.
- `lazy_imports_exclude` (plus `packages/rolldown/tests/rollipop-lazy-imports.test.ts`
  for RegExp/glob patterns through the JS API): `exclude` keeps a
  load-time-registering module eager.
- `lazy_imports_namespace_binding`: whole-namespace bindings through a removed
  barrel stay eager, so they can be enumerated and compared.
- `topics/hmr/rollipop_lazy_imports_dev_mode`: dev mode ignores the flag.
- `lazy_imports_unused_reexport`: re-exports that nothing reads through the
  barrel (direct, `import` + `export`, through two barrels, CommonJS importee,
  and the same symbols imported directly elsewhere or named but unused) keep
  their module eager. A re-export that is read stays lazy, except in the
  `no-tree-shaking` variant, where every re-exported module stays eager.
- `lazy_imports_reexport_namespace`: re-exports behind a namespace import, a
  namespace through `export *`, `export *`, `require()` and `import()` stay
  eager when only other exports are read; `ns.x` and `export *` reads stay lazy.
- `lazy_imports_merged_side_effect_import`: `import './x'` plus a named import
  of `./x` in the same file stays eager after tree-shaking merges them, also
  when the merged import is re-exported and read through the barrel.
- `lazy_imports_escaped_barrel`: the same through `require()`, `import()`, an
  escaping `import * as` and `export * as`.
- `lazy_imports_side_effect_import_barrel`: `import './lib/index.js'` runs the
  modules the index re-exports, through a nested barrel and when the index also
  reads one locally, even when another file reads them through the index.
- `lazy_imports_cycle_exclude`: `exclude` restores the eager entry point of an
  import cycle that would otherwise hit a TDZ error.
- `lazy_imports_entry_reexport`: a user-defined entry's re-exports stay eager.
- `lazy_imports_exclude_barrel`: `exclude` reaches a module behind a package
  index only when the pattern also matches the index.
- `cjs_direct_reads`: named-only and member-only namespace bindings use
  `r(id)`; `default` and namespace values keep `.i`; reads stay live, see keys
  added after load, and see exports added after a cycle finishes loading.
- `cjs_interop_namespace`: one namespace per module and interop mode, shared
  with `import()`; not cached during a cycle; `module.exports` reassignment;
  `__esModule` handling.
- `cjs_interop_late_keys` / `lazy_imports_cjs_late_keys`: keys added to a
  CommonJS module after load (an `init()` call, a monkeypatch) reach importers
  evaluated later, in both interop modes, and the namespace stays shared.
- `cjs_interop_frozen_namespace` / `lazy_imports_cjs_frozen_namespace`: a
  frozen shared namespace doesn't break importers evaluated after a key is
  added.

## Related

- [design.md](./design.md)
- `crates/rolldown/src/ecmascript/format/rollipop.rs`: chunk renderer and
  factory signatures
