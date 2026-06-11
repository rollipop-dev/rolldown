---
name: rollipop-upstream-sync
description: Sync upstream rolldown/rolldown main into the leegeunhyeok/rolldown rollipop fork branch, resolve fork-specific conflicts, preserve @rollipop package names and custom Rollipop patches, keep pluginutils aligned with upstream's rolldown/plugins package, and iterate until `just roll` passes.
---

# Rollipop upstream sync

Use this skill when syncing upstream `rolldown/rolldown` `main` into the Rollipop fork (`leegeunhyeok/rolldown`, branch `rollipop`). This fork produces the Rolldown build used by the Rollipop project, so upstream changes must be merged while preserving fork-specific package names, custom plugins, release behavior, and compatibility patches.

## Success criteria

- The local `rollipop` branch starts from the latest `origin/rollipop`.
- Upstream `main` is merged into `rollipop`.
- Fork-specific behavior is preserved and clearly marked.
- Lockfiles, generated bindings, submodules, and snapshots are consistent.
- `just roll` passes. Keep fixing and rerunning until it does.
- The final working tree is clean.

## Preflight

1. Confirm branch and cleanliness:

   ```bash
   git status --short --branch
   ```

2. Fetch and verify the local branch is not behind remote:

   ```bash
   git fetch origin rollipop
   git rev-list --left-right --count HEAD...origin/rollipop
   ```

   If the right-side count is nonzero, run:

   ```bash
   git pull --ff-only origin rollipop
   ```

   Do not start the sync from a branch that is behind `origin/rollipop`.

3. Fetch upstream:

   ```bash
   git fetch upstream main
   ```

   If `upstream` is missing, inspect remotes and add/ask before guessing.

## Merge workflow

1. Merge upstream:

   ```bash
   git merge upstream/main
   ```

2. Resolve conflicts surgically. Prefer upstream for ordinary Rolldown code, but preserve Rollipop-specific decisions.

3. Regenerate derived files via project commands rather than hand-editing generated outputs:

   ```bash
   just build-rolldown
   pnpm install --no-frozen-lockfile
   ```

   Use only the commands needed by the conflicts you resolved.

4. For submodules, prefer upstream unless Rollipop intentionally pins a different commit:

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

- **Workflows:** preserve Rollipop branch names, repository guards, package names, and publish behavior. Upstream workflow improvements can be adopted when they still apply to the fork.
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

- merge commit SHA
- whether `just roll` passed
- notable fork-specific decisions made during conflict resolution
- any unresolved risk or skipped verification
