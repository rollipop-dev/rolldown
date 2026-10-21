# Native React Compiler implementation

```ts
import { rolldown } from '@rollipop/rolldown';
import { rollipopReactNativePlugin } from '@rollipop/rolldown/experimental';

const bundle = await rolldown({
  input: 'App.tsx',
  plugins: [
    rollipopReactNativePlugin({
      runtimeTarget: 'hermes',
      swc: { react: { compiler: { exclude: [/node_modules/] } } },
    }),
  ],
});
```

The native configuration accepts `swc.react.compiler`, exposed through
`BindingRollipopReactNativeCompilerOptions`. Compiler options sit alongside the
native JSX settings in `swc.react`; Compiler also runs when the JSX runtime is
`Preserve`. The binding converts these settings once into
`ReactConfig.compiler` within `SwcConfig`. They remain part of the transformer's
configuration instead of being extracted by `into_parts` or stored separately
in the plugin or standalone binding.

Compiler options and filename filtering belong to the RN implementation. The
plugin uses only its own `swc.react.compiler`; omitting it disables Compiler.
There is no compiler option in the bundler's global transform configuration.
Include/exclude filters retain the existing glob/regex and bundler cwd
semantics. Includes can match the original ID or the filename without a
query/fragment; exclusions matching either form take precedence.

The standalone transformer accepts the same native configuration:

```ts
import { RollipopReactNativeTransformer } from '@rollipop/rolldown/experimental';
import { resolve } from 'node:path';

const transformer = new RollipopReactNativeTransformer({
  swc: { react: { compiler: { include: ['src/**'] } } },
});
const filename = resolve('src/App.tsx');
const result = await transformer.transform(filename, source);
const syncResult = transformer.transformSync(filename, source);
```

Both methods use the same pipeline and return `{ code, map, warnings }`.
Compiler filters use the process cwd captured when the standalone transformer
is constructed, only when a filename filter is configured. Concurrent async
calls share immutable configuration and create independent per-call SWC options.

`Transformer::transform` reads `swc.react.compiler` directly for both entry
points. The native transform crate defines the compiler settings and filter
interface; the RN plugin crate implements host filename matching without adding
Rolldown dependencies to the native transform crate. `ReactCompilerFilter::matches` receives
the filename and cwd from `TransformInput`. The plugin passes the bundler cwd,
while the standalone binding passes its captured process cwd.

The native compiler step checks the filename filter and applicable `fast_check`
before constructing SWC `PluginOptions`. SWC requires owned options, so
configured strings and lists are copied from the reusable settings only when
compilation proceeds. The input filename is allocated only when no explicit
compiler filename is configured. There is no whole-options clone or per-call
conversion in either host.

Compiler receives the parsed, typed SWC program before resolver,
native-component codegen, type stripping, Worklets and Hermes lowering. The
normal resolver runs once on the resulting AST; `CodegenVisitor` still receives
type annotations. No extra parse or emit is introduced on this path.

The general bundler does not install a Compiler transform or source prepass.
`enhanced_transform` and Vite's transform implementation match upstream v1.2.14.
Compiler configuration does not affect OXC's target-based plain-JS transform
gate. Existing fork-specific Fast Refresh filters remain supported. The bundler
and generic transform utilities expose no Compiler configuration; Compiler is
configured only through the native API.

The planned Rollipop JavaScript alias `transform.jsx.compiler` belongs in
Rollipop's configuration normalization, where it can merge with explicit
`transform.swc.native.react.compiler` settings before invoking this native API.
That downstream alias is not implemented in this repository.

SWC 25's `fast_check` applies only to infer/annotation; all/syntax must not be
excluded by that heuristic. No configuration means no compiler analysis or extra
AST walk. `enableReanimated` and `isDev` are forwarded upstream but currently
reserved by the pinned core.

## Fatal versus lint diagnostics

The pinned SWC 25 adapter returns the output program, diagnostics and logger
events, but omits the core `CompileResult` discriminant. Diagnostic severity
alone cannot recover fatality: successful `noEmit` linting can include Error
diagnostics, including uncategorized pipeline errors.

On the uncategorized Error path only, construct a core Success result from the
same events and run SWC's public `compile_result_to_diagnostics` conversion. A
fatal result contains an additional root error, while a successful result has
exactly its logger diagnostics. This avoids string matching, source re-parsing,
event serialization, or an extra conversion on successful hot paths. Both
`swc_ecma_react_compiler =25.0.0` and `react_compiler =0.0.1-oidc.0` are pinned because
this bridge relies on their diagnostic contract; recheck it when upgrading.

Nonfatal diagnostics use the RN plugin warning handler or the standalone
result's `warnings` array. Fatal Compiler errors fail the RN bundle, throw from
`transformSync`, or reject `transform`. Both `noEmit: true` and
`outputMode: 'lint'` expose analysis diagnostics; the latter also sets SWC's
`no_emit` flag because its adapter gates lint diagnostics on that flag. Normal
type stripping and native lowering still run in both modes; only Compiler
emission is disabled.

`tests/react_compiler.rs` covers the native ordering, type preservation and fatal
versus lint behavior. `packages/rolldown/tests/rollipop-react-native-compiler.test.ts`
covers the public NAPI/plugin paths, filters, runtime targets, standalone
sync/async and concurrent calls, Worklets, and coexistence with the existing
OXC Fast Refresh transform. The transform type tests reject compiler options
on the ordinary bundler configuration.

The public option path was also exercised with eight bundles (Flow/TS, explicit
and automatic Worklets, native codegen, both Hermes presets) in actual Hermes JSI
and compiled HBC, for sixteen successful executions. These used stub React/RN
modules and check cache reuse, captured/serialized values and generated native
props; they do not replace simulator UI or full Expo/EAS acceptance tests.

## Source-map regression coverage

`Cargo.toml` pins the React #36921 head
`b3ec622201fd97464de0cb92f47c9c5978d05d79` through `[patch.crates-io]` for the
core, AST and HIR entry points. `Cargo.lock` resolves their sibling compiler
crates to that same Git source. Keep the shared type graph intact when updating
the pin. The native core dependency and the vendored adapter's normalized manifest
require `=0.0.1-oidc.0`, matching this revision's crate version contract;
`Cargo.toml.orig` retains SWC's published manifest for comparison.

`rollipop-react-native-compiler-sourcemap.test.ts` checks actual original
file/line/column lookups inside compiled components, custom hooks, effect and
event callbacks, state updates and JSX. The fixture includes Unicode before
mapped expressions to check byte-offset conversion against UTF-16 columns.
Both component and hook must contain memo caches, so a skipped compiler cannot
make the assertions pass. Standalone cases also verify the compiler-off control.

The 24 cases cover Flow and TypeScript, both Hermes presets, standalone sync and
async calls, ESM/CJS bundle map composition, and Fast Refresh. Before the patch,
the initial 16-case version failed on unmapped bodies; the same assertions and
expanded coverage pass with the pinned core and the patched SWC 25.0.0 adapter.

The Rust compiler integration test also checks exact SWC spans for rebuilt
function declarations, function expressions and arrow functions, plus a
parenthesized TypeScript cast's surviving declarator. It uses a nonzero source
file offset. The old July pin drops the cast's closing parenthesis from the
declarator range; the current PR head passes the same assertion.

## Source-preserving binding renames

The root `[patch.crates-io]` resolves `swc_ecma_react_compiler =25.0.0` to
`vendor/swc_ecma_react_compiler`. The adapter's private `apply_renames` module
keys declarations and references by `(original source offset, original name)`.
This prevents generated names carrying reused source offsets from receiving
renames intended for the original binding. Identifier, shorthand property and
destructuring visitors use the same guarded lookup; import/export aliases and
static property handling retain the upstream behavior.

The binding-rename cases in `rollipop-react-native-compiler.test.ts` execute
successive and nested `map`/`filter` callbacks and callbacks shadowing an outer
`data`. They check first render, cache reuse and changed inputs, with compiler-off
controls. The 36 cases cover both Hermes presets, Flow/TypeScript and bundle,
standalone sync and standalone async APIs. All failed before the adapter patch.
The existing 24 source-map cases also remain required.
The 36 cases also fail against the latest core with the unpatched adapter,
confirming that the local rename guard is still required after updating the pin.
