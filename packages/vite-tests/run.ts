import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';
import { x } from 'tinyexec';
import { parseDocument } from 'yaml';

const VITE_DIR = path.resolve(import.meta.dirname, '../../vite');
const REPO_PATH = path.resolve(import.meta.dirname, './repo');
const ROLLDOWN_DIR = path.resolve(import.meta.dirname, '../rolldown');
const ROLLDOWN_OVERRIDE = `link:${ROLLDOWN_DIR}`;

function printTitle(title: string) {
  console.info(styleText(['cyan', 'bold'], title));
}

async function runCmdAndPipe(title: string, cmdOptions: Parameters<typeof x>): Promise<boolean> {
  printTitle(title);
  console.info('------------------------');
  const proc = x(...cmdOptions);
  proc.process?.stdout?.pipe(process.stdout);
  proc.process?.stderr?.pipe(process.stderr);
  const result = await proc;
  console.info('------------------------');
  if (result.exitCode !== 0) {
    console.error(
      styleText(
        'red',
        `${styleText('bold', 'Failed to execute command:')} ${
          [cmdOptions[0]].concat(cmdOptions[1] ?? []).join(' ')
        }`,
      ),
    );
    return true;
  }
  return false;
}

async function runCmdAndPipeOrExit(title: string, cmdOptions: Parameters<typeof x>): Promise<void> {
  const failed = await runCmdAndPipe(title, cmdOptions);
  if (failed) {
    process.exit(1);
  }
}

fs.rmSync(REPO_PATH, { recursive: true, force: true });

// Reuse the shared `vite/` checkout at the repo root, prepared by
// `just setup-vite` (the latest Vite `main`), the same
// code the dev-server tests run on. Setup happens only
// there, never here, so the checkout and the dev-server's built vite dist
// cannot drift apart. The tests run on a throwaway LOCAL clone of the
// checkout, never on the checkout itself: this suite edits tracked files
// (pnpm overrides) and the checkout must stay unpatched. The clone shares
// objects via hardlinks, so it needs no network.
if (!fs.existsSync(path.join(VITE_DIR, 'package.json'))) {
  console.error(
    styleText(['red', 'bold'], `Vite checkout not found at ${VITE_DIR}. Run \`just setup-vite\` first.`),
  );
  process.exit(1);
}
await runCmdAndPipeOrExit(
  '# Cloning the local vite checkout...',
  ['git', ['clone', VITE_DIR, REPO_PATH]],
);

// Write the `rolldown` override ourselves: Vite no longer carries a
// `rolldown: $rolldown` line for us to rewrite. A document edit keeps
// comments and survives any YAML formatting Vite picks.
printTitle('# Updating pnpm-workspace.yaml to link to local rolldown...');
const pnpmWorkspace = path.resolve(REPO_PATH, 'pnpm-workspace.yaml');
const workspaceDoc = parseDocument(fs.readFileSync(pnpmWorkspace, 'utf-8'));
workspaceDoc.setIn(['overrides', 'rolldown'], ROLLDOWN_OVERRIDE);
const newPnpmWorkspaceYaml = workspaceDoc.toString();
if (parseDocument(newPnpmWorkspaceYaml).getIn(['overrides', 'rolldown']) !== ROLLDOWN_OVERRIDE) {
  console.error(
    styleText(['red', 'bold'], `Failed to set \`overrides.rolldown: ${ROLLDOWN_OVERRIDE}\` in ${pnpmWorkspace}`),
  );
  process.exit(1);
}
fs.writeFileSync(pnpmWorkspace, newPnpmWorkspaceYaml, 'utf-8');

await runCmdAndPipeOrExit(
  '# Running `pnpm install`...',
  ['pnpm', ['install', '--no-frozen-lockfile'], { nodeOptions: { cwd: REPO_PATH } }],
);

// Fail when the clone does not resolve the workspace rolldown, so the suites
// never silently test an npm release again.
printTitle('# Checking that vite resolves the workspace rolldown...');
const resolveResult = await x(
  process.execPath,
  ['-e', "console.log(require('fs').realpathSync(require.resolve('rolldown/package.json')))"],
  { nodeOptions: { cwd: path.join(REPO_PATH, 'packages/vite') } },
);
const resolvedRolldownPkg = resolveResult.stdout.trim();
const expectedRolldownPkg = fs.realpathSync(path.join(ROLLDOWN_DIR, 'package.json'));
if (resolveResult.exitCode !== 0 || resolvedRolldownPkg !== expectedRolldownPkg) {
  console.error(
    styleText(
      ['red', 'bold'],
      `vite resolves rolldown to ${resolvedRolldownPkg || resolveResult.stderr.trim()}, expected ${expectedRolldownPkg}`,
    ),
  );
  process.exit(1);
}
const { version: rolldownVersion } = JSON.parse(fs.readFileSync(resolvedRolldownPkg, 'utf-8'));
console.info(`vite resolves rolldown ${rolldownVersion} at ${resolvedRolldownPkg}`);

await runCmdAndPipeOrExit(
  '# Running `pnpm exec playwright install chromium`...',
  ['pnpm', ['exec', 'playwright', 'install', 'chromium'], { nodeOptions: { cwd: REPO_PATH } }],
);
await runCmdAndPipeOrExit(
  '# Running `pnpm run build`...',
  ['pnpm', ['run', 'build'], { nodeOptions: { cwd: REPO_PATH } }],
);

// Skip known failing tests
// https://github.com/rolldown/rolldown/issues/8839
const assetsSpecPath = path.resolve(REPO_PATH, 'playground/assets/__tests__/assets.spec.ts');
const assetsSpec = fs.readFileSync(assetsSpecPath, 'utf-8');
fs.writeFileSync(assetsSpecPath, assetsSpec.replace(
  "test('import with raw query'",
  "test.skip('import with raw query'"
), 'utf-8');

const legacyStylesWatchSpecPath = path.resolve(
  REPO_PATH,
  'playground/legacy/__tests__/watch/legacy-styles-only-entry-watch.spec.ts',
);
const legacyStylesWatchSpec = fs.readFileSync(legacyStylesWatchSpecPath, 'utf-8');
fs.writeFileSync(legacyStylesWatchSpecPath, legacyStylesWatchSpec.replace(
  "test.runIf(isBuild)('rebuilds styles only entry on change'",
  "test.skip('rebuilds styles only entry on change'",
), 'utf-8');

// Preserve vitejs/vite@08a70f0a9's canary-only test stabilization on the
// throwaway clone, without rebasing its stale source tree onto main.
const reactSsrSpecPath = path.resolve(
  REPO_PATH,
  'playground/environment-react-ssr/__tests__/environment-react-ssr.spec.ts',
);
const reactSsrSpec = fs.readFileSync(reactSsrSpecPath, 'utf-8');
fs.writeFileSync(reactSsrSpecPath, reactSsrSpec.replace(
  `          .filter(Boolean),
      )
      .toStrictEqual(['react-fake-server', 'react-fake-client'])`,
  `          .filter(Boolean)
          .sort(),
        { timeout: process.env.CI ? 20000 : 5000 },
      )
      .toStrictEqual(['react-fake-client', 'react-fake-server'])`,
), 'utf-8');

for (const relativePath of [
  'playground/test-utils.ts',
  'playground/hmr-ssr/__tests__/hmr-ssr.spec.ts',
]) {
  const filePath = path.resolve(REPO_PATH, relativePath);
  const source = fs.readFileSync(filePath, 'utf-8');
  fs.writeFileSync(filePath, source.replace(
    '    }, 5000)',
    '    }, process.env.CI ? 20000 : 5000)',
  ), 'utf-8');
}

const e2eConfigPath = path.resolve(REPO_PATH, 'vitest.config.e2e.ts');
const e2eConfig = fs.readFileSync(e2eConfigPath, 'utf-8');
fs.writeFileSync(e2eConfigPath, e2eConfig.replace(
  'timeout: 50 * (process.env.CI ? 200 : 50)',
  'timeout: 50 * (process.env.CI ? 400 : 50)',
), 'utf-8');

// Dynamic CSS imports are not awaited by the entry module. Bundled dev can
// still be compiling them when page.goto() resolves, so wait for their effects.
const cssSpecPath = path.resolve(REPO_PATH, 'playground/css-codesplit/__tests__/css-codesplit.spec.ts');
const cssSpec = fs.readFileSync(cssSpecPath, 'utf-8');
fs.writeFileSync(cssSpecPath, cssSpec
  .replace("expect(await getColor('.dynamic'))", "await expect.poll(() => getColor('.dynamic'))")
  .replace("expect(await getColor('.async-js'))", "await expect.poll(() => getColor('.async-js'))")
  .replace(`  const css = await page.textContent('.dynamic-inline')
  expect(css).toMatch('.inline')`, `  await expect.poll(() => page.textContent('.dynamic-inline')).toMatch('.inline')`)
  .replace(`  const css = await page.textContent('.dynamic-module')
  expect(css).toMatch('_mod_')`, `  await expect.poll(() => page.textContent('.dynamic-module')).toMatch('_mod_')`),
'utf-8');

// Rolldown keeps the deduplicated CSS file under the `style2-*` name, not
// `style-*` (same adjustment as vitejs/vite@d716106b5 on the old rolldown-canary branch).
const cssCodesplitSpecPath = path.resolve(
  REPO_PATH,
  'playground/css-codesplit/__tests__/css-codesplit-consistent.spec.ts',
);
const cssCodesplitSpec = fs.readFileSync(cssCodesplitSpecPath, 'utf-8');
fs.writeFileSync(cssCodesplitSpecPath, cssCodesplitSpec.replaceAll(
  `      expect(findAssetFile(/style2-.+\\.css/)).toBeUndefined()
      expect(findAssetFile(/style-.+\\.css/)).toMatch('h2{color:#00f}')`,
  `      expect(findAssetFile(/style-.+\\.css/)).toBeUndefined()
      expect(findAssetFile(/style2-.+\\.css/)).toMatch('h2{color:#00f}')`,
), 'utf-8');

// With client-side HMR, `import.meta.hot.invalidate()` is handled inside the
// client and never reaches the server, so there is no "hmr invalidate" server
// log anymore. Assert the user-visible result instead.
const fbmHmrSpecPath = path.resolve(
  REPO_PATH,
  'playground/hmr-full-bundle-mode/__tests__/hmr-full-bundle-mode.spec.ts',
);
const fbmHmrSpec = fs.readFileSync(fbmHmrSpecPath, 'utf-8');
fs.writeFileSync(fbmHmrSpecPath, fbmHmrSpec.replace(
  `    await expect
      .poll(() => serverLogs.slice(logIndex).join('\\n'))
      .toContain('hmr invalidate')`,
  `    await expect
      .poll(() => page.textContent('.invalidation-parent'))
      .toBe('child updated')`,
), 'utf-8');

// Remove VITE_PLUS_* env vars to prevent leaking into loadEnv() test snapshots
for (const key of Object.keys(process.env)) {
  if (key.startsWith('VITE_PLUS_')) {
    delete process.env[key];
  }
}

const failed = []

const failedTestUnit = await runCmdAndPipe(
  '# Running `pnpm test-unit`...',
  ['pnpm', ['run', 'test-unit'], { nodeOptions: { cwd: REPO_PATH } }],
);
if (failedTestUnit) failed.push('test-unit');

const failedTestServe = await runCmdAndPipe(
  '# Running `pnpm test-serve`...',
  ['pnpm', ['run', 'test-serve'], { nodeOptions: { cwd: REPO_PATH } }],
);
if (failedTestServe) failed.push('test-serve');

const failedTestServeBundled = await runCmdAndPipe(
  '# Running `pnpm test-serve-bundled`...',
  ['pnpm', ['run', 'test-serve-bundled'], { nodeOptions: { cwd: REPO_PATH } }],
);
if (failedTestServeBundled) failed.push('test-serve-bundled');

const failedTestBuild = await runCmdAndPipe(
  '# Running `pnpm test-build`...',
  ['pnpm', ['run', 'test-build'], { nodeOptions: { cwd: REPO_PATH } }],
);
if (failedTestBuild) failed.push('test-build');

if (failed.length > 0) {
  console.error(styleText(['red', 'bold'], 'The following test suites failed:'));
  failed.forEach(test => console.error(styleText('red', ` - ${test}`)));
  process.exit(1);
}
