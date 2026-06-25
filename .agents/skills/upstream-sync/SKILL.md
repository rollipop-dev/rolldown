---
name: rollipop-upstream-sync
description: Sync upstream rolldown/rolldown main into the rollipop-dev/rolldown main fork branch, resolve fork-specific conflicts, preserve @rollipop package names and custom Rollipop patches, keep pluginutils aligned with upstream's rolldown/plugins package, and iterate until `just roll` passes.
---

# Rollipop upstream sync

Use this skill when syncing upstream `rolldown/rolldown` `main` into the Rollipop fork (`rollipop-dev/rolldown`, branch `main`). This fork produces the Rolldown build used by the Rollipop project, so upstream changes must be merged while preserving fork-specific package names, custom plugins, release behavior, and compatibility patches.

## Success criteria

- The local `main` branch starts from the latest `origin/main`.
- Upstream `main` changes are applied onto the fork's `main` as one single-parent, squash-style sync commit.
- Fork-specific behavior is preserved and clearly marked.
- The pnpm catalog `rolldown` version matches the synced upstream `packages/rolldown` version.
- Fork-managed npm package versions under `packages/*` stay at their pre-sync fork version unless the user explicitly asks for a release version bump.
- NAPI binding metadata is regenerated so package versions and binding versions stay in sync with the preserved fork package version.
- Lockfiles, generated bindings, submodules, and snapshots are consistent.
- `just roll` passes. Keep fixing and rerunning until it does.
- After `just roll` passes, request user review before creating the sync commit.
- Once the user approves the reviewed result, create the single-parent sync commit using the commit message rules below and leave the final working tree clean.

## Preflight

1. Confirm branch and cleanliness:

   ```bash
   git status --short --branch
   ```

2. Fetch and verify the local branch is not behind remote:

   ```bash
   git fetch origin main
   git rev-list --left-right --count HEAD...origin/main
   ```

   If the right-side count is nonzero, run:

   ```bash
   git pull --ff-only origin main
   ```

   Do not start the sync from a branch that is behind `origin/main`.

3. Fetch upstream:

   ```bash
   git fetch upstream main
   ```

   If `upstream` is missing, inspect remotes and add/ask before guessing.

## Merge workflow

1. Record the exact upstream commit being synced:

   ```bash
   git rev-parse upstream/main
   ```

2. Apply upstream changes with merge semantics, but do not create a normal upstream merge commit:

   ```bash
   git merge --squash upstream/main
   ```

   The final public commit must have only the fork `main` commit as its parent. Do not push a two-parent merge commit that makes upstream commits appear in the fork's mainline history. If a temporary normal merge commit was created while resolving conflicts, replace it with a single-parent commit that reuses the resolved tree before pushing.

3. Resolve conflicts surgically. Prefer upstream for ordinary Rolldown code, but preserve Rollipop-specific decisions.

4. Regenerate derived files via project commands rather than hand-editing generated outputs:

   ```bash
   just build-rolldown
   pnpm install --no-frozen-lockfile
   ```

   Use only the commands needed by the conflicts you resolved.

5. For submodules, prefer upstream unless Rollipop intentionally pins a different commit:

   ```bash
   git submodule update --init
   ```

## Fork-specific rules

### Package names and imports

The published package scope is `@rollipop`, not upstream's package names.

- `rolldown` package name becomes `@rollipop/rolldown`.
- `@rolldown/debug` becomes `@rollipop/rolldown-debug`.
- Keep upstream `@rolldown/pluginutils` as a dependency of `@rollipop/rolldown`; expose it through `@rollipop/rolldown/filter`.

When upstream adds tests, examples, or fixtures, check for imports from upstream package names:

```ts
import { rolldown } from 'rolldown';
import { viteImportGlobPlugin } from 'rolldown/experimental';
```

Rewrite them to the workspace package so tests exercise local Rollipop code:

```ts
import { rolldown } from '@rollipop/rolldown';
import { viteImportGlobPlugin } from '@rollipop/rolldown/experimental';
```

Search broadly after resolving conflicts:

```bash
rg "from ['\"]rolldown($|/)|require\\(['\"]rolldown($|/)" packages examples docs scripts
rg "@rolldown/debug|\"name\": \"rolldown\"" packages package.json pnpm-workspace.yaml
```

Only keep upstream package references when the test explicitly needs the published upstream package.

### Rollipop custom code

Rollipop-specific implementations should be easy to identify during future upstream syncs.

- Prefer isolated modules/crates for Rollipop-only behavior.
- If a change must live inside upstream-owned code, mark it with a clear `Rollipop` / `MARK: - Rollipop` comment consistent with the surrounding file style.
- Do not bury fork behavior in unrelated refactors.
- When a Rollipop custom dependency conflicts with an upstream upgrade, either update/remove the custom feature intentionally or document why the upstream change is being held back.

### Version alignment

Upstream and fork package versions are intentionally managed differently during a sync.

- Match the pnpm catalog `rolldown` version to the synced upstream `packages/rolldown` package version. Verify the upstream version from the fetched `upstream/main`, rather than assuming it from memory.
- Preserve the pre-sync version for npm-published fork-managed packages under `packages/*` unless the user explicitly asks for a release version bump. This fork package version is independent of the upstream version.
- Do not bump private/internal test packages such as `@rolldown/test-dev-server`; packages with `"private": true` are not npm publish targets.
- Ensure generated NAPI binding code uses the preserved fork package version. `just roll` normally builds and regenerates the binding code; if package versions changed by explicit user request but the generated binding output did not, run the NAPI binding build explicitly:

  ```bash
  just build-rolldown-binding
  ```

- CI checks that package versions and NAPI binding versions match, so do not finish the sync while those generated files are stale.

### Sync commit message

When the user approves creating the sync commit, use a `chore:` subject that summarizes the actual upstream Rolldown version synced. Include the exact upstream commit hash in the body. Do not add generic decision-context trailers to routine upstream sync commits.

Example:

```text
chore: sync upstream rolldown v1.1.2

Upstream commit: e0d0b1b876c9416037550516b1adbd9624072d5d
```

### pluginutils follows upstream

Upstream moved the Node package to the separate [`rolldown/plugins`](https://github.com/rolldown/plugins) repository under `packages/pluginutils`. This fork should not keep a separate `@rollipop/rolldown-pluginutils` workspace package. Instead, `@rollipop/rolldown` depends on upstream `@rolldown/pluginutils` and re-exports the filter helpers through `@rollipop/rolldown/filter`.

Important distinction: upstream still has the Rust helper crate `crates/rolldown_plugin_utils`; that is not the removed Node package. Treat the Rust crate as ordinary upstream code.

During every sync, ensure the fork still follows that shape:

1. Inspect current upstream references in `rolldown/rolldown`:

   ```bash
   git show upstream/main:packages/rolldown/src/filter-index.ts
   git show upstream/main:packages/rolldown/package.json | rg "pluginutils|./filter"
   git show upstream/main:pnpm-workspace.yaml | rg "@rolldown/pluginutils"
   ```

   At the time this skill was written, upstream `packages/rolldown/src/filter-index.ts` re-exported from the external catalog package `@rolldown/pluginutils/filter`.

2. Check the current upstream implementation in `rolldown/plugins`:

   ```bash
   git ls-remote https://github.com/rolldown/plugins.git HEAD
   ```

   Fetch or inspect `https://github.com/rolldown/plugins/tree/main/packages/pluginutils` when API changes affect filter helpers.

3. After `pnpm install`, verify the external package implementation installed by the catalog/lockfile, for example:

   ```bash
   rg --files node_modules/.pnpm | rg '@rolldown\\+pluginutils|@rolldown/pluginutils' | head
   ```

   If the package path/version changes, use `pnpm-lock.yaml` and `pnpm-workspace.yaml` to locate the actual installed implementation rather than assuming the old monorepo path.

4. Keep `packages/rolldown/src/filter-index.ts` aligned with upstream:

   ```ts
   export * from '@rolldown/pluginutils/filter';
   export { withFilter } from './plugin';
   ```

5. Do not reintroduce `packages/pluginutils`, `@rollipop/rolldown-pluginutils`, or dedicated pluginutils build/publish jobs.

## Conflict heuristics

- **Workflows:** preserve Rollipop repository guards, package names, and publish behavior. Upstream workflow improvements can be adopted when they still apply to the fork.
- **OXC/Rolldown API changes:** when an upstream API change breaks Rollipop code, first find the closest migrated implementation in Rolldown's upstream-owned code and follow that structure. For example, if sourcemap APIs move from borrowed to owned values, mirror nearby `OwnedSourceMap` handling instead of inventing a fork-only pattern.
- **Generated bindings:** update Rust binding definitions first, then run `just build-rolldown`; do not hand-maintain generated declarations except as a temporary diagnostic.
- **Lockfiles:** after resolving `package.json` / `Cargo.toml`, regenerate with `pnpm install --no-frozen-lockfile` and the relevant Cargo command (`cargo update ...` or the `just` recipe that triggered it).
- **Snapshots:** prefer rerunning the owning test/update command over editing snapshots by hand.
- **Deleted upstream files:** if the fork intentionally deleted CI/release files, keep them deleted unless the user asks to restore upstream behavior.

## Verification loop

Run targeted checks while fixing, but the final gate is always:

```bash
just roll
```

If it fails:

1. Read the first real failure, not only the final summary.
2. Fix the underlying conflict or stale generated output.
3. Rerun the smallest relevant check first.
4. Rerun `just roll` before declaring the sync complete.

Useful targeted checks:

```bash
just build-rolldown
just test-rust
just lint-rust
just test-node-rolldown-only
just lint-node
just lint-repo
just update-esbuild-diff
```

Report the final result with:

- merge commit SHA, if the user has approved and the merge commit has been created
- whether `just roll` passed
- notable fork-specific decisions made during conflict resolution
- any unresolved risk or skipped verification

If `just roll` passes but review has not happened yet, stop before committing and report:

- upstream commit merged
- current version/binding alignment
- notable fork-specific decisions made during conflict resolution
- any unresolved risk or skipped verification
- that the merge commit is pending user approval

## Optional Rollipop integration test

After `just roll` passes, the user may ask to validate the synced Rolldown build inside a local Rollipop project before the merge commit. Treat this as optional: ask whether to create the merge commit or run Rollipop integration tests unless the user already gave a clear instruction.

Use this flow when requested:

1. Confirm the upstream sync is complete and `just roll` passed.
2. Ask for the Rollipop project path if it was not provided. In that project, verify it is on `main`, fetch the remote, and fast-forward to the latest remote state before testing.
3. Before choosing setup, build, or test commands, read the Rollipop repository's local guidance such as `README.md`, `AGENTS.md`, package scripts, or other project docs. Follow that repository's instructions for initial setup, build, and test commands instead of guessing from memory.
4. In the Rolldown fork, build release artifacts for the current synced state. The Rollipop project normally consumes the native `.node` binary from `packages/rolldown`.
5. Decide whether binary replacement is enough:
   - Rollipop installs the JS package at `rollipop/node_modules/@rollipop/rolldown`.
   - Rollipop installs native binding packages beside it at `rollipop/node_modules/@rollipop/rolldown-binding-<platform>-<arch>`. Copy or move the generated `.node` binary into this binding package directory for the current architecture.
   - If the Rolldown binding package changed, pack `@rollipop/rolldown` and install that packed package into the Rollipop project. Before packing, temporarily change the packed package's native binding dependency from any unpublished workspace version to the latest already-published major-compatible range, for example `^1`, so Rollipop can install the package before the local `.node` binary is copied into place. Restore the fork package metadata after packing.
6. After the packed package is installed successfully, copy the rebuilt `.node` binary into the installed native package directory for the current architecture.
7. Run the Rollipop test command requested by the user, or use the normal full test command from the Rollipop repository guidance if none was specified.
8. If tests fail, determine whether the issue belongs in Rolldown or Rollipop:
   - For Rolldown-side issues, fix the fork, rerun the relevant Rolldown checks, rebuild the release binary, replace it in Rollipop, and rerun Rollipop tests.
   - For Rollipop-side required adaptations, report the needed Rollipop change clearly and do not hide it inside the Rolldown merge.
9. Repeat until the Rollipop tests pass or a clear external blocker remains.
10. Return to the Rolldown fork, report the Rollipop integration result, and ask for review before creating the merge commit.

Extra guardrails learned from integration testing:

- Resolve actual installed package locations from the Rollipop project before copying files. The expected layout is under the Rollipop project root, but package managers may hoist or link differently:

  ```bash
  node -p "require.resolve('@rollipop/rolldown/package.json')"
  node -p "require.resolve('@rollipop/rolldown-binding-darwin-arm64/package.json')"
  ```

- Inspect the packed tarball's `package.json` before installing it. `pnpm pack` may not include the same native `optionalDependencies` metadata that a real publish/prepublish step would generate, so do not assume the packed package will install unpublished binding packages automatically:

  ```bash
  tar -xOf /path/to/rollipop-rolldown-*.tgz package/package.json
  ```

- If the generated JS glue checks the native package version, temporarily adjust the installed native binding package's `package.json` version only inside the Rollipop test install, then restore it. Keep this as a test-only `node_modules` change.
- Scope `NAPI_RS_ENFORCE_VERSION_CHECK=1` to a focused Rolldown smoke test if needed. Do not run the full Rollipop test command with that environment variable unless requested; unrelated native packages in Rollipop can fail their own version checks and hide the Rolldown result.
- Back up every Rollipop `node_modules` directory or file you replace, restore it after testing, and verify the Rollipop working tree is clean. Do not commit Rollipop-side lockfile or install changes as part of the Rolldown upstream sync.

Do not let Rollipop integration testing replace the normal `just roll` gate. It is an extra compatibility check after the fork itself is already green.
