# Rollipop module ids — Implementation

> The rationale lives in [design.md](./design.md).

## Data flow

1. `GenerateStage::generate` (`stages/generate_stage/mod.rs`) builds
   `RollipopModuleIds::new(&module_table, is_bundled)` right before
   `finalize_rollipop_modules`, only for `format: 'rollipop'` without
   `profilerNames`. `is_bundled` is the filter `finalize_rollipop_modules`
   uses: the runtime module or `metas[idx].is_included`. It is stored on the stage (`rollipop_module_ids`) and is
   empty otherwise.
2. `finalize_rollipop_modules` passes it to every finalizer as
   `RollipopAstFinalizerContext::module_ids`. `runtime_id_expr_for` emits the
   numeric id for `__rollipop_require__(id)`, `.i(id)`, `__rollipop_lazy__(id)`
   and dynamic imports; with `profilerNames` it emits the `stable_id` string.
3. `GenerateContext::rollipop_module_ids` carries the same table to the chunk
   renderer: `render_module_runtime_id` (`ecmascript/format/rollipop.rs`)
   prints it for module definitions and the entry require.

## Assignment (`rollipop/module_ids.rs`)

- `assign_module_ids` takes a `ModuleIdEntry` (`idx`, `stable_id`, `id`,
  `bundled`) for every normal module, sorts by `(!bundled, stable_id, id)`, and
  gives each module
  `xxh3_64(stable_id) % ROLLIPOP_MODULE_ID_SPACE` (2^24), probing linearly
  (wrapping) past taken slots.
- The result is `IndexVec<ModuleIdx, Option<u32>>`; externals are `None`
  (they are required by their string id through `__rollipop_require__.e`).
  `RollipopModuleIds::get` panics on a module without an id: every normal
  module has one, so a miss is a bug.
- The function takes the space as a parameter so the unit tests can force
  collisions and wraparound with a space of 2 or 4.

## Export getters

`create_define_exports_stmt` sorts `named_exports` by exported name before
emitting the `__rollipop_require__.d(exports, { … })` getters.

## Generated binding names

`binding_name_for_import` (`module_finalizers/rollipop.rs`) names an
importee's binding `import_{repr}_{record}` and appends `$n` when the name is
taken (user bindings, unresolved globals, factory parameters, earlier
generated names). Calls happen in AST order, so the suffixes are
deterministic.

## Tests

- Unit tests in `rollipop/module_ids.rs`: ids are the hash, independent of
  `ModuleIdx`, collisions probe in `stable_id` order, probing wraps, shared
  `stable_id`s are ordered by `id`, adding a module keeps the others, and a
  module that isn't bundled never shifts a bundled one.
- `module_id_collision` fixture: `c1467.js` and `c4211.js` hash to the same
  id; the static import, `require()` and `import()` all use the probed ids.
- `packages/rolldown/tests/rollipop-module-ids.test.ts`: 120 modules loaded
  with random latency (which shuffles `ModuleIdx`) build to byte-identical
  bundles, minified and not, and adding a module keeps every other module's
  id. A removed barrel resolving one record to two `util.js` modules checks
  the `$n` binding suffix. All three tests fail with the old `ModuleIdx` ids.
- Every snapshot under `tests/rolldown/function/format/rollipop/` and the
  `topics/hmr/rollipop_*` fixtures, including the re-enabled
  `cjs_interop_namespace` and `lazy_imports` snapshots.
