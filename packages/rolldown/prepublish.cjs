const { spawnSync } = require('node:child_process');

const args = ['pre-publish', '-t', 'npm', '--no-gh-release'];
const isDryRun =
  process.env.ROLLDOWN_NAPI_PREPUBLISH_DRY_RUN === 'true' ||
  process.env.npm_config_dry_run === 'true' ||
  process.env.npm_config_dry_run === '1';

if (isDryRun) {
  args.push('--dry-run');
}

const result = spawnSync('napi', args, {
  stdio: 'inherit',
  shell: process.platform === 'win32',
});

process.exit(result.status ?? 1);
