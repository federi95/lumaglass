// Desktop-only catalog rendering. No TV endpoint, mount or system mutation.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { buildNative } from '../tools/ci/native-smoke.mjs';
import { root } from '../tools/lib/project.mjs';

buildNative();
const out = path.join(root, '.build');
await fs.mkdir(out, { recursive: true });
const temporary = await fs.mkdtemp(path.join(out, 'system-apps-smoke-'));
const preferences = path.join(temporary, 'preferences.json');
const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
const catalog = `settings,down,down,down,down,down,right,${Array(5).fill('down').join(',')},enter`;
const detail = `${catalog},down,down,down,down,enter`;
async function scenario(name, script) {
  const dump = path.join(temporary, `${name}.json`);
  const diagnostics = path.join(out, `${name}-render.json`);
  const result = spawnSync(
    binary,
    [
      '--preferences',
      preferences,
      '--script',
      script,
      '--frames',
      '90',
      '--dump-state',
      dump,
      '--screenshot',
      path.join(out, `${name}.png`),
      '--diagnostics',
      diagnostics,
    ],
    { cwd: root, stdio: 'inherit', timeout: 120000 },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0);
  const state = JSON.parse(await fs.readFile(dump, 'utf8'));
  const render = JSON.parse(await fs.readFile(diagnostics, 'utf8'));
  assert.equal(state.webos, false);
  assert.equal(render.capture.glyph_failures, 0);
  assert.equal(render.capture.missing_images, 0);
  return state;
}
try {
  await fs.writeFile(preferences, JSON.stringify({ reduced_motion: true, show_weather: false }));
  const listed = await scenario('system-app-catalog', catalog);
  assert.equal(listed.subpage, 'AppManagement');
  assert.ok(listed.app_catalog.report.preview);
  assert.ok(listed.app_catalog.report.apps.length > 0);
  const selected = await scenario('system-app-detail', detail);
  assert.equal(typeof selected.subpage.ManagedApp, 'string');
  assert.ok(selected.ui_text.includes('Inspect system visibility'));
  const inspected = await scenario('system-app-inspected', `${detail},down,down,enter`);
  assert.equal(inspected.app_catalog.system_visibility.report.status, 'unavailable');
  assert.equal(inspected.app_catalog.system_visibility.report.can_restore, false);
  for (const [name, extra] of [
    ['restore', ',down,enter'],
    ['hide', ',down,down,enter'],
  ]) {
    const refused = await scenario(
      `system-app-${name}-unavailable`,
      `${detail},down,down,enter${extra}`,
    );
    assert.equal(refused.app_catalog.system_visibility.confirming, false);
    assert.deepEqual(refused.preferences.hidden, []);
    assert.equal(typeof refused.subpage.ManagedApp, 'string');
  }
  const restored = await scenario('system-app-back', `${detail},back`);
  assert.equal(restored.subpage, 'AppManagement');
  assert.deepEqual(restored.focus, { CatalogApp: selected.subpage.ManagedApp });
  console.log(
    'Catalog rendering, read-only inspection, disabled global actions and Back focus verified.',
  );
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
