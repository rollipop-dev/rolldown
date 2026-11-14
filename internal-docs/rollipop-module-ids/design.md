# Rollipop module ids — Design

## Summary

`format: 'rollipop'` registers every module as `id: function(...) { ... }` and
loads it with `__rollipop_require__(id)`. The numeric id of a module is
`xxh3(stable_id) mod 2^24`, with collisions resolved by linear probing in
`stable_id` order. The same input always produces the same bundle, and adding
or removing one module leaves the other modules' ids alone. With
`profilerNames` (what the Rollipop host sets in dev), ids stay `stable_id`
strings. See [implementation.md](./implementation.md) for the code.

## Goals

1. **Reproducible output.** Ids used to be `ModuleIdx`, which follows loader
   discovery order and so depends on how parallel resolve and load calls
   finish. Identical inputs produced different bundles, differing only in
   digits. That defeats content hashes, makes OTA updates ship spurious
   changes, and made snapshot tests flaky (`cjs_interop_namespace` and
   `lazy_imports` had snapshots disabled for this reason).
2. **Small OTA diffs.** An update that adds one module should change that
   module and its importers, not renumber the rest of the bundle. An id is a
   function of the module's own `stable_id`, so it is the same in every build
   that contains the module.
3. **Cheap literals.** Ids are emitted at every definition and every require
   site, so they should stay short integers within every engine's
   small-integer range.

## Decisions

- **Hash of `stable_id`.** `stable_id` is the cwd-relative path with `/`
  separators, so it is the same across machines and operating systems. The
  hash is `xxh3_64`, spelled out rather than taken from `FxHash` or `std`
  hashers, because the id is now an output contract that must survive
  toolchain and dependency upgrades.
- **Fixed space of 2^24.** Ids have at most 8 digits and stay far below the
  31-bit small-integer range of JS engines. With `n` modules about
  `n² / 2^25` pairs collide: about 3 for 10,000 modules. A space that grew
  with the module count would renumber every module whenever the count
  crossed a threshold, so it is fixed. A wider space would cost about one more
  digit per reference to save a handful of collisions.
- **Deterministic collision resolution.** Modules are processed sorted by
  `(stable_id, id)` and a taken slot probes to the next one. The full `id`
  breaks ties when two modules share a `stable_id` (a plugin's relative id and
  an absolute path). A module's id changes only when a module that hashes to
  the same run of slots and sorts before it is added or removed.
- **Assigned once per build.** `GenerateStage` computes the ids before
  finalizing modules and hands the same table to the finalizer
  (`__rollipop_require__(id)`) and the chunk renderer (`id: function`), so the
  two can't disagree.
- **Every normal module gets an id,** including ones tree-shaking removes, so
  no lookup can miss. Such a module can still take a slot that an included
  module would otherwise probe into; that is rare and still deterministic.
- **Import binding names don't encode ids.** Generated bindings used to be
  `import_{repr}_{importerIdx}_{record}_{importeeIdx}`, which leaked
  `ModuleIdx` into the output even with `profilerNames`. They are now
  `import_{repr}_{record}`, with `$n` suffixes in source order when a removed
  barrel resolves one record to several same-named modules. Each factory is
  its own function scope, so the importer's index added nothing.
- **Dev mode uses the same scheme.** HMR patches address modules by
  `stable_id` strings (`registerFactory("dep.js", ...)`,
  `loadExports("dep.js")`) and never by numeric id, and the Rollipop host turns
  on `profilerNames` in dev, so its dev bundles never contain numeric ids. A
  hash of `stable_id` is also stable across incremental rebuilds, which
  `ModuleIdx` only was because the module table is reused.

## Trade-offs

Hashed ids are longer than dense ones. On the repo's Rollipop fixtures (a
handful of modules each) the snapshots grew by 12 to 36 bytes. That is net of
the shorter generated binding names (`import_dep_0` instead of
`import_dep_1_0_2`).

On a synthetic graph of 1,974 tiny modules with 10,524 id references,
minified, compared with the same output renumbered densely:

| Output                | Dense ids | Hashed ids | Change                    |
| --------------------- | --------- | ---------- | ------------------------- |
| JS                    | 250.7 KB  | 291.7 KB   | +41.0 KB (~3.9 B per ref) |
| JS, gzip              | 35.9 KB   | 48.5 KB    | +12.6 KB                  |
| Hermes bytecode       | 788.6 KB  | 799.7 KB   | +11.2 KB (~1.1 B per ref) |
| Hermes bytecode, gzip | 126.3 KB  | 148.6 KB   | +22.3 KB                  |

The modules there are tiny, so the relative numbers overstate what a real app
sees; the per-reference cost is what carries over. Random digits compress
poorly, so compressed output grows more than raw output does. This is the
price for ids that don't move between builds: with dense ids, a one-module
change can renumber much of the bundle.

## Rejected alternatives

- **Dense ids in `stable_id` order.** Shortest literals and also
  deterministic, but adding or removing one module renumbers every module
  sorted after it. Most of the bundle would change on a one-file update, the
  OTA diff problem this design exists to fix.
- **Dense ids in execution order.** The same renumbering problem, and the
  order also moves when imports are reordered.
- **Hash space scaled to the module count.** Shorter ids for small apps, but
  crossing a size threshold renumbers everything.
- **`stable_id` strings everywhere.** What `profilerNames` does: readable and
  stable, but far larger than an integer per reference.

## Unresolved questions

- Separately built bundles that share a module registry (split bundles, a
  host bundle plus feature bundles) can't resolve collisions against each
  other's modules. If Rollipop ever ships those, it needs a wider space or a
  shared id table.

## Related

- [implementation.md](./implementation.md)
- [../rollipop-lazy-imports/design.md](../rollipop-lazy-imports/design.md)
