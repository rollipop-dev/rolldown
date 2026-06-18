---
name: rollipop-integration
description: Validate a locally built @rollipop/rolldown native binary inside a user-provided Rollipop monorepo path. Use when upstream syncs, Rolldown feature work, NAPI changes, or any change that can affect the rolldown .node binary must be checked against Rollipop tests with a local package tarball and copied native binding.
---

# Rollipop integration

Use this skill to prove that the current `rolldown` checkout works in the real Rollipop project with the exact native binary just built locally.

## Assumptions

- Rolldown repo: `/Users/ghlee/workspace/rolldown` unless the user says otherwise.
- Rollipop repo path must be provided by the user for each run. If it is missing, ask for it before editing or installing dependencies.
- `/Users/ghlee/workspace/rollipop` is only an example path from one local environment.
- Rollipop uses Yarn 4 with `nodeLinker: node-modules`.
- Temporary Rollipop dependency overrides are test setup. Revert your own temporary changes after verification unless the user asks to keep them.

## Workflow

1. Preflight both repos.
   - In both repos, run `git status --short --branch`.
   - If Rollipop has existing user changes in files you would touch, stop and ask before editing.
   - In Rolldown, run `just --list` before falling back to package-manager commands.
   - Refer to the user-provided Rollipop path as `<rollipop-root>` in commands and notes.

2. Build Rolldown artifacts.
   - Run `just build-rolldown` when NAPI glue/binding output may need to be refreshed.
   - Run `just build-rolldown-release` to produce the release `.node` binary.
   - Expected local binary pattern: `packages/rolldown/dist/rolldown-binding.<platform-arch>.node`.

3. Pack the local Rolldown package.
   - From `packages/rolldown`, create a deterministic tarball named `package.tgz`.
   - On current Rolldown, use the fallback form because `pnpm pack --filename` is not supported:
     ```bash
     find . -maxdepth 1 -type f -name '*.tgz' -delete
     pnpm pack --pack-destination .
     mv rollipop-rolldown-<version>.tgz package.tgz
     ```
   - Avoid `rm *.tgz` under zsh; empty globs fail before `rm` runs.
   - Use the absolute tarball path in Rollipop, for example `/Users/ghlee/workspace/rolldown/packages/rolldown/package.tgz`.

4. Point Rollipop at the local package.
   - Check how Rollipop currently supplies `@rollipop/rolldown`.
   - Current Rollipop uses `.yarnrc.yml`:
     ```yaml
     catalogs:
       rolldown:
         "@rollipop/rolldown": file:/Users/ghlee/workspace/rolldown/packages/rolldown/package.tgz
     ```
   - If a future Rollipop checkout uses `package.json` `resolutions` instead, set:
     ```json
     {
       "resolutions": {
         "@rollipop/rolldown": "file:/Users/ghlee/workspace/rolldown/packages/rolldown/package.tgz"
       }
     }
     ```
   - Run `yarn install` from `<rollipop-root>` after the override.
   - Treat both `.yarnrc.yml` and `yarn.lock` changes as temporary integration setup unless the user asks to keep them.

5. Install the local native binary into Rollipop's binary package folder.
   - Determine the current Node platform/arch:
     ```bash
     node -p '`${process.platform}-${process.arch}`'
     ```
   - Map the platform to package folder and filename:
     - `darwin-arm64` -> `node_modules/@rollipop/rolldown-binding-darwin-arm64/rolldown-binding.darwin-arm64.node`
     - `darwin-x64` -> `node_modules/@rollipop/rolldown-binding-darwin-x64/rolldown-binding.darwin-x64.node`
     - `linux-x64` glibc -> `node_modules/@rollipop/rolldown-binding-linux-x64-gnu/rolldown-binding.linux-x64-gnu.node`
     - `linux-x64` musl -> `node_modules/@rollipop/rolldown-binding-linux-x64-musl/rolldown-binding.linux-x64-musl.node`
     - `linux-arm64` glibc -> `node_modules/@rollipop/rolldown-binding-linux-arm64-gnu/rolldown-binding.linux-arm64-gnu.node`
     - `linux-arm64` musl -> `node_modules/@rollipop/rolldown-binding-linux-arm64-musl/rolldown-binding.linux-arm64-musl.node`
     - `win32-x64` -> `node_modules/@rollipop/rolldown-binding-win32-x64-msvc/rolldown-binding.win32-x64-msvc.node`
   - In the observed example layout, `@rollipop/rolldown` is installed under `<rollipop-root>/packages/rollipop/node_modules`, not root `node_modules`.
   - Place the binding package in `packages/rollipop/node_modules/@rollipop/<binding-package>` unless the actual dependency tree shows a different consumer workspace.
   - Create the package folder if missing. Preserve it if it already exists.
   - If the binding package folder was pruned by `yarn install`, create a minimal `package.json` in it:
     ```json
     {
       "name": "@rollipop/rolldown-binding-darwin-arm64",
       "version": "1.0.15",
       "main": "rolldown-binding.darwin-arm64.node",
       "os": ["darwin"],
       "cpu": ["arm64"]
     }
     ```
     Use the actual package name, version, os, cpu, and filename for the current platform.
   - Copy the release binary from Rolldown `packages/rolldown/dist/` to the Rollipop binary package path.

6. Verify Rollipop uses the local package and binary.
   - From `<rollipop-root>`, run:
     ```bash
     yarn workspace rollipop node -e "const p=require.resolve('@rollipop/rolldown/package.json'); console.log(p); console.log(require(p).version)"
     yarn workspace rollipop node -e "console.log(require.resolve('@rollipop/rolldown-binding-<target>'))"
     yarn workspace rollipop node -e "import('@rollipop/rolldown').then(m => console.log(Object.keys(m).slice(0, 5).join(',')))"
     ```
   - Replace `<target>` with the platform target such as `darwin-arm64`.

7. Run Rollipop verification.
   - From `<rollipop-root>`, start with `yarn roll` for the full existing check suite.
   - If `yarn roll` stops in `vp check` before tests, run `yarn test:all` to check the binary integration path and report the `vp check` blocker separately.
   - If `yarn test:all` hangs after failures, stop the session after collecting enough failing test names and rerun the failing tests directly, for example:
     ```bash
     yarn workspace rollipop test e2e/transformer.spec.ts -t native --reporter verbose
     yarn workspace rollipop test src/core/__tests__/rolldown.spec.ts -t polyfills --reporter verbose
     ```
   - If time or failure scope requires a narrower run, use the relevant workspace command, but report that full verification was not run.

8. Cleanup.
   - Revert only your temporary Rollipop override/lockfile changes unless the user asks to keep them.
   - Run `yarn install` again after reverting Rollipop dependency overrides so `node_modules` matches the restored lockfile.
   - Do not delete the generated Rolldown `package.tgz` or `.node` unless cleanup is explicitly requested.
   - End with `git status --short --branch` in both repos.

## Notes

- The packed `@rollipop/rolldown` tarball excludes `.node` files by design, so copying the release binary into Rollipop's platform binary package is required.
- The local tarball can prune optional binding package entries from `yarn.lock`. Creating the current platform binding package folder manually is expected.
- PnP tests create a separate temporary project; if they fail, capture whether the temporary project copied the local file catalog and can resolve the manually installed binding package.
- If `yarn roll` fails before it reaches Rolldown usage, report the unrelated blocker separately from the binary integration result.
