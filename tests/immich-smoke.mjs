// CI only. No personal account, real token or network endpoint is used.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { buildNative } from '../tools/ci/native-smoke.mjs';
import { root } from '../tools/lib/project.mjs';
buildNative();
const out = path.join(root, '.build');
const temporary = await fs.mkdtemp(path.join(out, 'immich-smoke-'));
const preferences = path.join(temporary, 'preferences.json');
const credentials = path.join(temporary, 'preferences.immich.json');
const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
const open = `settings,down,down,right,click:900:980,${Array(6).fill('down').join(',')},enter`;
const sentinel = 'fixture-key-not-a-real-credential';
async function scenario(name, script, extra = []) {
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
      '160',
      '--dump-state',
      dump,
      '--screenshot',
      path.join(out, `${name}.png`),
      '--diagnostics',
      diagnostics,
      ...extra,
    ],
    { cwd: root, stdio: 'inherit', timeout: 90000 },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0);
  const serialized = await fs.readFile(dump, 'utf8');
  assert.ok(!serialized.includes(sentinel));
  assert.ok(!serialized.includes('api_key'));
  const state = JSON.parse(serialized);
  const render = JSON.parse(await fs.readFile(diagnostics, 'utf8'));
  assert.equal(render.capture.missing_images, 0);
  assert.equal(render.capture.glyph_failures, 0);
  assert.equal(render.capture.missing_glyphs, 0);
  return state;
}
try {
  await fs.writeFile(
    preferences,
    JSON.stringify({ reduced_motion: true, show_weather: false, language: 'en' }),
  );
  const missing = await scenario('immich-missing', `${open},enter,await-immich`);
  assert.equal(missing.subpage, 'Immich');
  assert.equal(missing.immich.status, 'immichCredentials');
  assert.equal(missing.immich.configured, false);
  await fs.writeFile(
    credentials,
    JSON.stringify({ schema: 1, server: 'http://127.0.0.1:1', api_key: sentinel, albums: [] }),
    { mode: 0o600 },
  );
  const refused = await scenario('immich-insecure', `${open},enter,await-immich`);
  assert.equal(refused.immich.status, 'immichInvalidServer');
  assert.equal(refused.immich.configured, false);
  const cancel = await scenario('immich-forget-cancel', `${open},down,down,enter`);
  assert.equal(cancel.subpage, 'Immich');
  assert.deepEqual(cancel.focus, { Immich: 3 });
  await fs.access(credentials);
  const cache = path.join(temporary, 'preferences.wallpapers');
  await fs.mkdir(cache, { recursive: true });
  await fs.writeFile(path.join(cache, 'offline-marker.jpg'), 'offline image remains');
  const forgotten = await scenario(
    'immich-forgotten',
    `${open},down,down,enter,down,enter,await-immich`,
  );
  assert.equal(forgotten.immich.status, 'immichForgotten');
  assert.equal(forgotten.immich.configured, false);
  await assert.rejects(fs.access(credentials));
  assert.equal(
    await fs.readFile(path.join(cache, 'offline-marker.jpg'), 'utf8'),
    'offline image remains',
  );
  // Demo mode is explicitly desktop-only and blocks all connection commands.
  const demo = await scenario('immich-albums', `${open},${Array(5).fill('down').join(',')},enter`, [
    '--immich-preview',
  ]);
  assert.equal(demo.immich.album_count, 18);
  assert.equal(demo.immich.selection_count, 1);
  assert.equal(demo.immich.configured, false);
  assert.equal(demo.focus, 'ImmichAlbum');
  assert.ok(!JSON.stringify(demo).includes('Demo album 1'));
  assert.deepEqual(demo.ui_text, []);
  const page = await scenario('immich-page', `${open},${Array(4).fill('down').join(',')},enter`, [
    '--immich-preview',
  ]);
  assert.equal(page.immich.page, 1);
  assert.equal(page.immich.album_count, 18);
  const back = await scenario('immich-back', `${open},back`);
  assert.equal(back.subpage, 'Wallpapers');
  assert.equal(back.focus, 'PhotoImmich');
  console.log(
    'Native Immich settings, private-file errors, confirmation, offline retention, demo selection, paging and Back verified.',
  );
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
