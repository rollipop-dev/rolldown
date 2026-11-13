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
   declaration produces a lazy holder, and reads become `import_x.a.name`. The
   importee runs when one of its bindings is first read; later reads cost one
   extra property load. This is the role Metro's `inlineRequires` plays,
   without an inline `require()` call per use site.
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
2. **A small, measured steady-state cost.** Metro rewrites every use site to
   an inline `require()` call, which adds a call to every access, forever. Here
   a read adds one property load on the holder (`import_x.a.name`).
   Measured in Hermes (3M×3 reads): ESM 224 ms eager, 239 ms holder (+6.7%);
   CJS 447 / 463 ms (+3.6%); a closure getter (`import_x().name`) takes
   312 / 547 ms. In the example app the time to first commit didn't change
   measurably against the earlier Proxy facade (+0.05 ms, CI ±3.5 ms, A/A
   control 2.7 ms), and JS allocation before first commit dropped 2%.
3. **Small bundle cost.** The lazy loader is passed as an extra factory
   parameter, so a lazy declaration `l(5)` compiles to the same Hermes call as
   the eager `r(5)`. The `.a` load per read costs about 8 bytes of HBC per
   read site: +16 KB (+1.45%) in the example app.
4. **Opt-in and dev-free.** Laziness changes evaluation order, so it is behind
   a flag, ignored outside `format: 'rollipop'`, and ignored in dev mode, where
   the HMR runtime inspects and replaces `__rollipop_require__.c` records
   directly.

## Semantics

- A module is evaluated on the first read of a binding through any lazy holder
  of it, or on any eager require.
- These stay eager: side-effect-only imports (`import './x'`), namespace imports
  (`import * as ns`), `export * from` / `export * as ns from`, bindings that
  stand for a whole namespace (`import { ns }` through a removed
  `export * as ns` barrel), externals, and imports whose bindings are never
  read after tree-shaking. A namespace value can be enumerated or compared,
  so it has to be the real object. Imports with unread
  bindings are kept only for the importee's side effects, so deferring them
  would drop those effects.
- A re-export getter (`export { x } from`, or `import { x }` plus
  `export { x }`) is a read only if some emitted code reads `x` _through_ the
  re-exporting module: an importer of `x` from it that references the binding,
  a namespace member read `ns.x`, a further re-export or `export *` that is
  itself read. Otherwise the importee is kept only for its side effects and
  stays eager, as it would without lazy imports. Only reads visible in the
  source count; escaping namespaces are covered by the next rule.
  Symbol usage can't decide this: the linker binds importers straight to the
  canonical symbol, so a module that imports `x` from its own module retains it
  even when nothing goes through the barrel.
- `import 'pkg'` runs pkg's whole graph eagerly, as without lazy imports. So
  do `require('pkg')` (what Babel emits for `import 'pkg'` in CommonJS),
  `import('pkg')`, and a namespace import whose object escapes (a bare `ns`,
  or re-exported as `export * as ns`). Re-export getters of such a module, and
  of every barrel it reaches through re-exports and `export *`, don't count as
  reads, so the modules they re-export (often ones that register themselves at
  load, as in `import 'react-native-gesture-handler'`) run with it. Otherwise
  another file reading `a` through pkg would silently change what loading pkg
  does. That costs laziness, never correctness.
- Nothing in the bundle reads a user-defined entry's exports, so its re-exports
  stay eager. With tree-shaking off, every statement is included and nothing
  tells which re-exports are imported, so re-exported modules stay eager too.
- "Read" means read in the finalized module. If the minifier later removes a
  read as dead code (for example an unused `x.y;` statement), the importee is
  no longer evaluated through that binding.
- Eager wins: if one module imports the same importee both lazily and eagerly,
  the single generated declaration is eager. This includes `import './x'` next
  to `import { a } from './x'`, which tree-shaking merges into one declaration
  before the finalizer sees it, and into `export { a } from './x'` when the
  module also re-exports `a`.
- Initialization errors, from the factory or from interop construction, are
  rethrown on every read, matching the eager rethrow semantics from #60. A
  holder only resolves once its module finished loading successfully.
- A lazily imported module's load error surfaces at the first read, not when
  its importer runs. Feature detection such as
  `try { require('./wrapper') } catch {}`, where the wrapper imports a module
  that throws at load, no longer catches it; `exclude` the module that may
  throw, or read one of its bindings inside the `try`.
- In a circular import, a lazy holder of a module that is still loading reads
  through to its partially-initialized exports, like an eager import. It
  resolves once that module has loaded.
- Laziness can change which module of an import cycle runs first. Eagerly, a
  cycle is entered at the first member in import order; lazily, at the first
  member whose binding is read. Code that only works because of the eager entry
  point (a member reading another member's `const` at load time) can then hit a
  TDZ error. `exclude` on the member that has to run first restores the eager
  entry point (fixture `lazy_imports_cycle_exclude`). Keeping every cycle member
  eager would avoid this, but React Native app code has many cycles, so that
  would give up much of the benefit; Metro's `inlineRequires` has the same
  hazard.

As with Metro's `inlineRequires`, code that relies on another module's
import-time side effects without reading any of its bindings must use an
explicit side-effect import.

## Rejected alternatives

- **Inline requires at use sites** (Metro): grows the bundle with every
  reference and adds a call per access. Interop (`.t`) would also run per
  access unless separately cached.
- **Closure getter** (`var a = lazy(id); a().x`): a call per read, 22-39%
  slower steady-state reads on Hermes than the holder's property load.
- **Proxy facade** (the first implementation): the declaration held a facade
  object whose prototype was a `Proxy` until first access; then it copied the
  export descriptors onto itself and dropped the `Proxy`. Use sites stayed
  `import_x.name`, so reads cost exactly as much as eager ones. Review found the
  partially resolved state hard to get right: interop construction removed the
  `Proxy` before it had succeeded, so after a throw later reads returned
  `undefined`. It also needed descriptor copying, a no-`Proxy` fallback, and
  per-module facade caching, and creating plus first-reading a facade cost
  about 4× a holder. The holder has none of that, for one property load per
  read; app startup time didn't change measurably between the two.
- **Lazy namespace as a permanent `Proxy`**: every access would go through a
  trap forever.
- **Deferring only side-effect-free modules**: this could never change
  behavior, but Rolldown's analysis is conservative. Top-level calls without
  `/*#__PURE__*/` (`StyleSheet.create(...)`, `createContext(...)`) and most
  CommonJS modules count as side-effectful, so little React Native code would
  be deferred. Like Metro, we defer all modules instead, decide the unsafe
  cases from the graph, and leave `exclude` as the escape hatch. Not measured.

## Interop cache trade-off

Previously each importer built its own namespace, with the export keys present
at its import time. A CommonJS module can add keys after it finished loading,
synchronously (`exports.init = () => { exports.client = … }`, or another module
monkeypatching it) or asynchronously. Importers evaluated after that point saw
the new keys. To keep that, `.i` compares the number of own keys of
`module.exports` on every call and extends the cached namespaces in place when
it changed. This adds one `Object.getOwnPropertyNames` per import site. The
startup numbers in PR #65 were measured before this check and have not been
re-measured.

Remaining differences from per-importer namespaces:

- Earlier importers now also see the new keys, because the object is shared.
  This is closer to ESM, where a module has a single namespace object. For the
  same reason, a property one importer adds to the namespace (for example with
  `Object.assign`), or a `this` write by a method of a class-instance
  `module.exports` called through the namespace, is seen by every importer, and
  `import()` returns the same object as `import * as`.
- A delete plus an add that leaves the key count unchanged is not detected.
- A namespace that an importer froze, sealed or made non-extensible can't take
  new keys. It is dropped from the cache instead, so later importers get a new
  object and stop sharing it with earlier ones.

Namespaces are never cached while a module is still loading (circular imports),
and they are rebuilt if `module.exports` is reassigned.

## Configuration surface

`lazyImports: boolean | { exclude?: StringOrRegExp | StringOrRegExp[] }`.

Metro's inline requires is a per-file Babel transform that can't see the module
graph and leaves a `require()` call at every use site. Most of its options make
up for that:

- `nonInlinedRequires` keeps hot modules (React, react-native) from paying a
  `require()` call per access. Not needed here: once initialized, a read costs
  one property load more than an eager read, and a lazy import of a module that
  has already run gets an already resolved holder.
- Memoization options (`unstable_memoizeInlineRequires` and related) avoid
  repeated `require()` calls. Not needed: a holder resolves once.
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

`exclude` is decided per import edge. When the module is reached through a
package index (`import { x } from 'pkg'` → `pkg/index.js` →
`export { x } from './register.js'`), the index itself is a lazy holder, and
the excluded module runs only when the index does. The pattern has to match the
index as well, for example `'**/node_modules/pkg/**'`. Propagating `exclude`
backwards along re-export edges was not done: it would turn whole package
entries eager without the user asking for it.

## Relation to `import defer`

`lazyImports` is non-standard. It ports Metro's `inlineRequires`: the bundler
decides for every static import whether to defer it, so existing React Native
code benefits without changes. The standard mechanism is TC39
[`import defer`](https://github.com/tc39/proposal-defer-import-eval)
(`import defer * as ns from './x'`), where the importer opts in per import and
only namespace imports can be deferred. Rolldown tracks it in
[rolldown/rolldown#4857](https://github.com/rolldown/rolldown/issues/4857) and
currently lowers it to a normal import with a warning
(`pre_process_ecma_ast.rs`).

`lazyImports` is meant as an interim feature. Once Rolldown supports
`import defer`, Rollipop should move to it. The holder runtime has the same
shape as Rspack's `import defer` runtime (`.zO`), so it can back both; at that
point `lazyImports` can become a compatibility option on top of it or be
removed.

## Default

`lazyImports` is off by default, both here and in Rollipop. React Native's Metro
template enables `inlineRequires` by default, but this feature changes
evaluation order, so it stays opt-in until it has been stabilized on real apps.
Turning it on by default for Rollipop production builds should be revisited
then.

## Related

- [implementation.md](./implementation.md)
- [../react-native-compiler/design.md](../react-native-compiler/design.md) — other Rollipop React Native pipeline decisions
