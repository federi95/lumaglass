// CI-only native connector coverage. No real account, key or service is contacted.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { root } from '../tools/lib/project.mjs';
import { buildNative } from '../tools/ci/native-smoke.mjs';
buildNative();
const out = path.join(root, '.build');
const temporary = await fs.mkdtemp(path.join(out, 'cloud-smoke-'));
const preferences = path.join(temporary, 'preferences.json');
const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
const open = (provider) =>
  `settings,tap:{"Sidebar":1},tap:{"Setting":17},tap:{"PhotoCloud":"${provider}"}`;
async function scenario(name, script, demo = false) {
  const dump = path.join(temporary, `${name}.json`);
  const diagnostics = path.join(out, `${name}-render.json`);
  const args = [
    '--preferences',
    preferences,
    '--script',
    script,
    '--frames',
    '180',
    '--dump-state',
    dump,
    '--diagnostics',
    diagnostics,
    '--screenshot',
    path.join(out, `${name}.png`),
  ];
  if (demo) args.push('--cloud-preview');
  const result = spawnSync(binary, args, { cwd: root, stdio: 'inherit', timeout: 90000 });
  if (result.error) throw result.error;
  assert.equal(result.status, 0);
  const serialized = await fs.readFile(dump, 'utf8');
  for (const secret of [
    'fixture-password',
    'refresh_token',
    'client_secret',
    'fixture-user',
    'photos.google.com/picker/',
  ])
    assert.ok(!serialized.includes(secret));
  const state = JSON.parse(serialized);
  const render = JSON.parse(await fs.readFile(diagnostics, 'utf8'));
  assert.equal(render.capture.missing_images, 0);
  assert.equal(render.capture.missing_glyphs, 0);
  assert.equal(render.capture.glyph_failures, 0);
  return state;
}
try {
  await fs.writeFile(
    preferences,
    JSON.stringify({ reduced_motion: true, show_weather: false, language: 'en' }),
  );
  for (const provider of ['webdav', 'drive', 'google-photos']) {
    const missing = await scenario(
      `cloud-${provider}-missing`,
      `${open(provider)},enter,await-cloud`,
    );
    assert.deepEqual(missing.subpage, { Cloud: provider });
    assert.equal(missing.cloud.status, 'cloudCredentials');
    assert.equal(missing.cloud.configured, false);
    const demo = await scenario(`cloud-${provider}-demo`, open(provider), true);
    assert.equal(demo.cloud.status, 'cloudPreview');
    assert.equal(demo.cloud.offline_export, provider === 'google-photos');
    assert.ok(demo.ui_text.length === 0, 'Connector text must remain private in dumps');
    assert.ok(
      demo.controls
        .filter((c) => c.target.Cloud === 0 || c.target.Cloud === 1 || c.target.Cloud === 2)
        .every((c) => !c.enabled),
    );
  }
  for (const language of ['it', 'zh', 'ko']) {
    await fs.writeFile(
      preferences,
      JSON.stringify({ reduced_motion: true, show_weather: false, language }),
    );
    await scenario(`cloud-${language}-export-demo`, open('google-photos'), true);
  }
  await fs.writeFile(
    preferences,
    JSON.stringify({ reduced_motion: true, show_weather: false, language: 'en' }),
  );
  const credentials = path.join(temporary, 'preferences.webdav.json');
  await fs.writeFile(
    credentials,
    JSON.stringify({
      schema: 1,
      server: 'http://127.0.0.1:1/dav/',
      username: 'fixture-user',
      app_password: 'fixture-password',
    }),
    { mode: 0o600 },
  );
  const invalid = await scenario('cloud-webdav-insecure', `${open('webdav')},enter,await-cloud`);
  assert.equal(invalid.cloud.status, 'cloudCredentials');
  const cache = path.join(temporary, 'preferences.wallpapers');
  await fs.mkdir(cache, { recursive: true });
  await fs.writeFile(path.join(cache, 'offline-marker'), 'retained');
  const cancelled = await scenario('cloud-forget-cancel', `${open('webdav')},down,down,enter`);
  assert.deepEqual(cancelled.focus, { Cloud: 3 });
  await fs.access(credentials);
  const removed = await scenario(
    'cloud-forgotten',
    `${open('webdav')},down,down,enter,down,enter,await-cloud`,
  );
  assert.equal(removed.cloud.status, 'cloudForgotten');
  await assert.rejects(fs.access(credentials));
  await fs.access(path.join(cache, 'offline-marker'));
  const back = await scenario('cloud-back', `${open('drive')},back`);
  assert.equal(back.subpage, 'Wallpapers');
  assert.deepEqual(back.focus, { PhotoCloud: 'drive' });
  console.log(
    'Three native cloud views, private diagnostics, offline export, explicit loading, rejection, Cancel, Forget and cache retention verified.',
  );
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
