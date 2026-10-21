# React Compiler in the native SWC pipeline

React Native inputs need SWC's Flow parser, native-component codegen, Worklets,
and Hermes compatibility passes. Running React Compiler after those passes loses
the lexical declarations it expects: both compiler implementations bail out on
function-local `var`. Skipping block-scoping is unsafe on supported Hermes
runtimes, which parse `let` but do not preserve block/loop-closure semantics by
default.

The native pipeline therefore runs `swc_ecma_react_compiler` on its existing AST
before the resolver and native transformations. There is no additional source
parse/emission or JavaScript compiler invocation. The SWC adapter still converts
AST/scope data into the compiler's internal representation; compilation itself
has a cost. OXC continues to handle the ordinary bundler transformations, without
running another React Compiler.

## Comparison before replacement

The previous OXC pin was `d47efe5cb875e118e845ee6eda15d205ce3b566d`. It vendors a
fork of the React Rust compiler and lowers directly from OXC AST/scopes. SWC
25.0.0 initially used the published `react_compiler` 0.1.0 crates, whose source
revision is `ff7445e636429ab26ce3d388535a79c845d88c94`. These are related
implementations, not the same library instance.

A 51-case differential probe covered lexical/var bindings, aliases and mutations,
hooks, TS, inference/annotation/all modes, opt-outs, gating, targets 17/18/19,
memo/forwardRef wrappers, generated-name collisions, and worklet callbacks.
Fifty cases agreed on observable behavior, including the same 32 memoized cases.
Ninety-seven successful outputs matched the original input's return values over
three stubbed React renders each. This checks representative behavior, not full
React reconciliation or native UI-runtime equivalence.

The remaining case was `panicThreshold: 'all_errors'` with a local `var`: the old
Rolldown/OXC diagnostic route returned code because its Todo diagnostic had
Advice severity even though the compiler reported Fatal. SWC propagates the
failure. The replacement honors that stricter explicit setting. Default bailouts
remain nonfatal; SWC's adapter can suppress bailout diagnostics that OXC exposed
as warnings.

## Scope and public configuration

Both `rollipopReactNativePlugin` and the standalone
`RollipopReactNativeTransformer` accept `swc.react.compiler`. Compiler options
sit alongside the existing native JSX settings in `swc.react`. Compiler runs
before JSX lowering and can run while the JSX runtime remains `Preserve`.
Omitting `compiler` disables analysis; configured include/exclude filters select
its inputs. The standalone transformer uses the same configuration for both
synchronous and asynchronous calls.

The ordinary Rolldown OXC pipeline follows the upstream v1.2.14 implementation.
It does not register a Compiler plugin, run a Compiler source prepass, or force
plain-JS transformation because Compiler is configured. Generic transform
utilities and Vite's transform plugin do not expose or execute Compiler. The
bundler's transform options have no compiler setting or fallback path.

Compiler option data, filename filtering, SWC option conversion and execution
belong to the RN plugin/transform crates. The common options crate has no
Compiler configuration. Existing fork-specific Fast Refresh filters remain
intact.

The configuration remains nested through the entire native pipeline:
`SwcConfig.react.compiler` owns the reusable compiler settings, and
`Transformer::transform` reads them directly. The plugin and standalone binding
do not extract or store a separate compiler configuration. Host-specific
filename matching stays behind a filter adapter in the RN plugin crate, keeping
the native transform crate independent of Rolldown.

SWC consumes its compiler options, so the native transform constructs owned SWC
options only after filename filtering and the applicable `fast_check` pass.
Configured strings and lists still need copying from the reusable settings;
the whole options object is never cloned. This avoids allocating SWC options
for skipped files without serializing concurrent transforms.

Rollipop's JavaScript API can provide `transform.jsx.compiler` as an alias that
merges into `transform.swc.native.react.compiler`. That alias belongs to the
downstream Rollipop implementation; this repository exposes only the native
`swc.react.compiler` setting.

See [implementation](./implementation.md) for ordering and diagnostics.

## Preserve compiled body source maps

The published core drops raw AST offsets when it rebuilds component and hook
bodies. SWC's reverse AST converter requires those offsets and otherwise emits
unmapped spans, so retaining only `sourcesContent` does not preserve debugging.

The workspace patches `react_compiler`, `react_compiler_ast` and
`react_compiler_hir` to [React PR #36921](https://github.com/facebook/react/pull/36921)
at `b3ec622201fd97464de0cb92f47c9c5978d05d79`, the PR head checked on October 22, 2026. This includes the later declarator-range, rebuilt-function and outlined
binding fixes, and carries offsets through lowering/codegen without a restoration
traversal. Its crates declare version `0.0.1-oidc.0`, so the vendored SWC 25
adapter's three React dependency constraints and the native transform's direct
core dependency use that exact version. The SWC crate versions remain unchanged.

Patching only the core would leave SWC's direct AST/HIR dependencies on the
registry and introduce incompatible Rust types. These three entry-point patches
resolve all twelve compiler crates to the same Git revision. No additional
runtime pass, source parse or source-map approximation is introduced.

## Preserve binding identity independently of source locations

The source-offset fix also gives generated identifiers their originating spans.
SWC 25's `apply_renames` identified references only by offset and could rename a
generated `_temp2` to `item_0` or `data_1` without updating its declaration.
Repeated callback parameters and shadowed `data` bindings then failed at runtime.

The workspace patches the SWC adapter to the copy in
`vendor/swc_ecma_react_compiler`. Only `src/apply_renames.rs` differs from the
published adapter's source: declaration and reference lookups now match both the
source offset and original identifier name. Already-renamed identifiers and
generated temporaries retain their names. Source spans remain intact, including
the body mappings fixed by React #36921. The existing pass and transform order
remain unchanged; no additional parse or AST traversal is introduced.

Vendoring the adapter makes this private SWC implementation patch reproducible
inside Rolldown without changing the SWC or React repositories. Its original
version, source metadata, tests and fixtures are retained; the vendor README
records provenance and the upgrade procedure.
