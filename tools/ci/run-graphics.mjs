// Separate runners keep the unchanged suites, preferences and X servers isolated.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { root } from '../lib/project.mjs';

const suites = {
  home: 'smoke.mjs',
  catalog: 'system-apps-smoke.mjs',
  screensaver: 'screensaver-smoke.mjs',
  photos: 'immich-smoke.mjs',
  design: 'design-smoke.mjs',
  brands: 'brands-smoke.mjs',
  weather: 'weather-smoke.mjs',
  translations: 'translations-smoke.mjs',
  display: 'display-watchdog-smoke.mjs',
};
const suite = process.argv[2];
if (!Object.hasOwn(suites, suite)) throw new Error('Select a supported graphics suite.');
const started = performance.now();
const result = spawnSync(
  'xvfb-run',
  ['-a', '-s', '-screen 0 1920x1080x24', process.execPath, `tests/${suites[suite]}`],
  {
    cwd: root,
    stdio: 'inherit',
    timeout: 20 * 60 * 1000,
    env: { ...process.env, LIBGL_ALWAYS_SOFTWARE: '1', LUMAGLASS_SMOKE_PREBUILT: '1' },
  },
);
const seconds = Math.round((performance.now() - started) / 10) / 100;
const exitCode = result.status ?? 1;
await fs.mkdir(path.join(root, '.build'), { recursive: true });
await fs.writeFile(
  path.join(root, `.build/ci-timing-${suite}.json`),
  `${JSON.stringify({ suite, seconds, exitCode, signal: result.signal })}\n`,
);
if (process.env.GITHUB_STEP_SUMMARY) {
  await fs.appendFile(
    process.env.GITHUB_STEP_SUMMARY,
    `Graphics **${suite}**: ${seconds} s, exit ${exitCode}.\n`,
  );
}
if (result.error) console.error(result.error.message);
process.exitCode = exitCode;
