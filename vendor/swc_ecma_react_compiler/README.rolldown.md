# Rolldown's SWC React Compiler adapter patch

This is the published `swc_ecma_react_compiler` 25.0.0 crate. Its archive SHA-256 is
`69da32d5f1c98dd6be5eb823a00efd30e7bb9f2bfb38b62eef39cee3ee38747a`;
the original VCS metadata is retained in `.cargo_vcs_info.json`.
Upstream production sources, tests and fixtures are unchanged except for
`src/apply_renames.rs`. The normalized `Cargo.toml` also changes the three React
dependencies from `=0.1.0` to `=0.0.1-oidc.0`, matching React PR #36921's pinned
head `b3ec622201fd97464de0cb92f47c9c5978d05d79`. `Cargo.toml.orig` retains the
published constraints. The upstream SWC Apache license and
React MIT notice referenced by the source headers are included.

React [PR #36921](https://github.com/facebook/react/pull/36921), pinned by the
root workspace, preserves source offsets on generated identifiers too. SWC's
position-only rename pass can consequently rename a generated `_temp2` reference
to `item_0` or `data_1`, while leaving its declaration unchanged.

The local patch keys both declaration and reference lookups by source offset
**and original identifier name**. It preserves source spans and retains the
existing traversal and shorthand/import/export handling. It adds no source
parse, compiler invocation or AST pass. Bundle and standalone RN APIs share it.
The 36 callback/shadowing runtime regressions still fail with the unmodified
adapter against `b3ec622`; the binding-identity patch remains necessary.

When updating SWC, replace this directory from the published crate, review the
diff against `src/apply_renames.rs` and `Cargo.toml`, and reapply the patches only
if still needed. Align the React dependency versions with the pinned core.
Run the compiler runtime and source-map regressions followed by `just roll`.
The vendor directory is excluded from workspace membership and repository
format/lint rewrites so the upstream sources remain easy to compare.
