# Rollipop lazy imports & cached interop namespaces — Design

## Summary

The Rollipop output format registers every module as a factory and runs it on
the first `__rollipop_require__(id)`. Before this change, every static import
became `var import_x = __rollipop_require__(id)` at the top of the importer's
factory, so requiring the entry evaluated its whole static import graph before
the first render — including modules only used later (screens, handlers,
rarely-taken branches). Two independent optimizations target React Native
startup:

1. **Lazy imports** (`output.lazyImports`, default `false`): an import
   declaration produces a lazy namespace. The importee runs when one of its
   bindings is first read, and later reads cost the same as an eager import.
   This is the role Metro's `inlineRequires` plays, without rewriting use
   sites.
2. **Cached interop namespaces** (always on): ESM code importing CommonJS gets
   the interop namespace from `__rollipop_require__.i(id)`, built once per
   module, instead of a fresh `__rollipop_require__.t(require(id))` wrapper per
   import site. `.t` defines a getter for every export key, so modules like
   `react-native` (hundreds of keys) imported from hundreds of files paid that
   cost once per importer.

See [implementation.md](./implementation.md) for the runtime and finalizer
details.

## Goals

1. **Fewer modules evaluated before first render.** Startup time is the
   motivation.
2. **No use-site rewriting, no steady-state cost.** Metro rewrites every use
   site to an inline `require()` call. That grows the bundle with every
   reference and adds a call to every access, forever. Here only the
   declaration changes; `import_x.name` is untouched. After initialization, a
   read is an own accessor read on a plain object, exactly like today.
   Measured in the React Native 0.88 app's Hermes: 300k×3 reads took 22.8ms in
   both cases, versus 30.5ms with a closure getter (`import_x().name`).
3. **Bytecode and bundle size neutral.** In lazy mode the lazy loader is passed
   as an extra factory parameter, so a lazy declaration `l(5)` compiles to the
   same Hermes call as the eager `r(5)`. No per-import property load.
4. **Opt-in and dev-free.** Laziness changes evaluation order, so it is behind
   a flag, ignored outside `format: 'rollipop'`, and ignored in dev mode, where
   the HMR runtime inspects and replaces `__rollipop_require__.c` records
   directly.

## Semantics

- A module is evaluated on the first property read (or `in` check) through any
  lazy namespace of it, or on any eager require.
- These stay eager: side-effect-only imports (`import './x'`), namespace imports
  (`import * as ns`), `export * from` / `export * as ns from`, bindings that
  stand for a whole namespace (`import { ns }` through a removed
  `export * as ns` barrel), externals, and imports whose bindings are never
  read after tree-shaking. A namespace value can be enumerated or compared,
  which a facade can't support before it initializes. Imports with unread
  bindings are kept only for the importee's side effects, so deferring them
  would drop those effects.
- "Read" means read in the finalized module. If the minifier later removes a
  read as dead code (for example an unused `x.y;` statement), the importee is
  no longer evaluated through that binding.
- Eager wins: if one module imports the same importee both lazily and eagerly,
  the single generated declaration is eager.
- Initialization errors are rethrown on every read, matching the eager
  rethrow semantics from #60. A namespace only becomes "initialized" once its
  module finished loading successfully.
- In a circular import, a lazy namespace of a module that is still loading
  reads through to its partially-initialized exports, like an eager import. It
  becomes final once that module has loaded.
- Without `Proxy`, `__rollipop_require__.l` falls back to eager evaluation.

As with Metro's `inlineRequires`, code that relies on another module's
import-time side effects without reading any of its bindings must use an
explicit side-effect import.

## Rejected alternatives

- **Inline requires at use sites** (Metro): grows the bundle with every
  reference and adds a call per access. Interop (`.t`) would also run per
  access unless separately cached.
- **Closure getter** (`var a = lazy(id); a().x`): +2 bytes per reference and
  ~34% slower steady-state reads on Hermes.
- **Lazy namespace as a permanent `Proxy`**: every access would go through a
  trap forever. Here the `Proxy` is only the facade's _prototype_ until first
  access, and then it is replaced by the real prototype.
- **Returning the importee's real `module.exports` with a trapping
  prototype**: exports have to exist before the factory runs, and a failed
  factory leaves own getters behind, so later reads would not rethrow. The
  separate facade avoids both problems.

## Interop cache trade-off

The cached namespace snapshots the export keys when it is first built after the
module finished loading. Previously each importer snapshotted at its own import
time. The two differ only when a CommonJS module adds export keys
asynchronously after loading, which already broke named imports for earlier
importers. Namespaces are never cached while a module is still loading
(circular imports), and they are rebuilt if `module.exports` is reassigned.
Sharing one namespace per module is closer to ESM, where a module has a single
namespace object.

## Configuration surface

`lazyImports: boolean | { exclude?: StringOrRegExp | StringOrRegExp[] }`.

Metro's inline requires is a per-file Babel transform that can't see the module
graph and leaves a `require()` call at every use site. Most of its options make
up for that:

- `nonInlinedRequires` keeps hot modules (React, react-native) from paying a
  `require()` call per access. Not needed here: once initialized, reads cost the
  same as eager reads, and a lazy import of a module that has already run returns
  the eager value without creating a facade.
- Memoization options (`unstable_memoizeInlineRequires` and related) avoid
  repeated `require()` calls. Not needed: a facade is cached once per module.
- `blockList` (absolute importer paths) is a manual escape hatch for files that
  break when inlined. Most of those cases are decided automatically from the
  graph here: side-effect imports, namespace values and unread bindings stay
  eager.

The case left over is an importee whose load-time side effects are needed
before any of its bindings is read, such as a module that registers itself.
Source code can express this with a side-effect import (`import './setup'`).
`exclude` covers code that can't be edited, such as dependencies. It matches
**importee** module ids, because the dependency on load-time effects belongs to
the imported module. A single pattern then applies at every import site, unlike
Metro's importer-keyed `blockList`. Patterns use the same string-glob/RegExp
semantics as other Rolldown module filters (`pattern_filter::filter`).

## Default

`lazyImports` is off by default, both here and in Rollipop. React Native's Metro
template enables `inlineRequires` by default, but this feature changes
evaluation order, so it stays opt-in until it has been stabilized on real apps.
Turning it on by default for Rollipop production builds should be revisited
then.

## Unresolved questions

- A stricter mode that only defers modules without side effects (from
  `sideEffects` and Rolldown's own analysis) would never change behavior. Most
  React Native modules probably count as side-effectful (top-level
  `StyleSheet.create(...)` calls, for example), so the startup win could
  shrink. This hasn't been measured.

## Related

- [implementation.md](./implementation.md)
- [../react-native-compiler/design.md](../react-native-compiler/design.md) — other Rollipop React Native pipeline decisions
