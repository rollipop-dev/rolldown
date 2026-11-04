# Worklets and Reanimated 3 in the native pipeline

The RN plugin and standalone `RollipopReactNativeTransformer` accept the same
`worklets` configuration. Callers supply the installed runtime version:

```ts
rollipopReactNativePlugin({ worklets: { pluginVersion: '0.7.4' } });
rollipopReactNativePlugin({ worklets: { reanimatedVersion: '3.19.5' } });
```

`RollipopReactNativeWorkletsConfig` is a union with mutually exclusive version
fields. Shared transform options sit alongside the version. Worklets-only options
(`strictGlobal`, `disableWorkletClasses`, `bundleMode`) belong to the Worklets
branch; `processNestedWorklets` belongs to Reanimated 3. Omitting `worklets` skips
the transform. An empty object is invalid: the native API does not detect packages.
Rollipop can detect packages or accept user overrides before calling this API.

The NAPI boundary converts the flat object into `swc-react-native` 0.4's
`Options::Worklets` or `Options::Reanimated` once when constructing the plugin or
transformer. It validates legacy versions there, rejecting anything outside 3.x
and rejecting Reanimated Bundle Mode. For untyped JavaScript callers supplying
both versions, `pluginVersion` takes precedence, matching the upstream options
deserializer. Babel extension hooks remain unsupported by the native transform.

The transform dispatches to the corresponding visitor after codegen, type
stripping and JSX lowering, before Hermes compatibility and helper injection.
This keeps object spread inside serialized Reanimated worklets rather than
capturing a helper introduced by later lowering. Each visitor keeps its runtime's
closure and metadata format. Runtime selection adds no extra AST traversal.

`packages/rolldown/tests/rollipop-react-native-transform.test.ts` executes host
and serialized worklets through bundling and standalone sync/async APIs with both
Hermes presets. It also covers legacy callbacks, debug/release metadata and
configuration errors. React Compiler ordering is documented in
[the compiler implementation](../react-native-compiler/implementation.md).
