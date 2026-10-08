// Handling of the Vite checkout (`vite/` at the repo root, gitignored), the
// single Vite checkout shared by the dev-server test harness
// (`packages/test-dev-server`, see `main.ts`) and Vite's own test suite
// (`packages/vite-tests/run.ts`, which clones this checkout locally). It is
// a plain clone of vitejs/vite on `main`, so both harnesses track the
// latest Vite fixes without rebasing the stale canary test patches.

import { execFileSync, execSync } from 'node:child_process';
import nodeFs from 'node:fs';
import nodePath from 'node:path';
import nodeUrl from 'node:url';

// scripts/src/setup-vite/checkout.ts sits three levels below the repo root.
export const repoRoot = nodePath.resolve(
  nodePath.dirname(nodeUrl.fileURLToPath(import.meta.url)),
  '../../..',
);
export const viteDir = nodePath.join(repoRoot, 'vite');

export const run = (cmd: string, cwd: string): void => {
  console.log(`[setup-vite] ${cmd}`);
  execSync(cmd, { cwd, stdio: 'inherit' });
};

// Spawn a Node script directly, without a shell: run() goes through the
// platform shell, which on Windows is cmd.exe (execSync uses %ComSpec%) and
// cannot execute POSIX-style paths such as `./node_modules/.bin/rolldown`.
export const runNode = (scriptPath: string, args: string[], cwd: string): void => {
  console.log(`[setup-vite] node ${[scriptPath, ...args].join(' ')}`);
  execFileSync(process.execPath, [scriptPath, ...args], { cwd, stdio: 'inherit' });
};
const capture = (cmd: string, cwd: string): string =>
  execSync(cmd, { cwd, encoding: 'utf8' }).trim();

// Track main directly. The stale canary branch misses current compatibility
// fixes, and rebasing its test patches can conflict before any tests run.
//
//   - Missing (fresh clone / CI): clone `main`.
//   - Clean and on `main` or the old `rolldown-canary`: update to main
//     (kept as-is when the fetch fails, e.g. offline).
//   - Dirty or on another branch: taken over by the developer, used
//     exactly as-is so local experiments are never trampled.
export function ensureViteCheckout(): void {
  if (!nodeFs.existsSync(nodePath.join(viteDir, 'package.json'))) {
    run('git clone --branch main https://github.com/vitejs/vite.git vite', repoRoot);
  } else {
    updateViteCheckout();
  }
  const head = capture('git rev-parse --short HEAD', viteDir);
  console.log(`[setup-vite] using vite ${head}`);
}

function updateViteCheckout(): void {
  const dirty = capture('git status --porcelain', viteDir) !== '';
  const branch = capture('git rev-parse --abbrev-ref HEAD', viteDir);
  if (dirty || (branch !== 'main' && branch !== 'rolldown-canary')) {
    console.log(
      `[setup-vite] vite/ ${dirty ? 'has local changes' : 'is on a developer branch'}, using it as-is`,
    );
    return;
  }
  try {
    run('git fetch origin main', viteDir);
  } catch {
    console.log('[setup-vite] fetch failed, using the existing checkout as-is');
    return;
  }
  run('git checkout -B main origin/main', viteDir);
}
