// Native desktop graphics in CI only; never contacts or emulates a TV service.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import sharp from 'sharp';
import { buildNative } from '../tools/ci/native-smoke.mjs';
import { root } from '../tools/lib/project.mjs';
buildNative();
const out = path.join(root, '.build');
const temporary = await fs.mkdtemp(path.join(out, 'screensaver-smoke-'));
const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
const photo = 'personal-1234567890abcdef';
const settings = { schema: 1, photos: false, language: 'en', hour_24: true, images: [] };
async function capture(name, config, extra = []) {
  await fs.writeFile(path.join(temporary, 'settings.json'), JSON.stringify(config));
  const image = path.join(out, `${name}.png`);
  const result = spawnSync(
    binary,
    ['--system-screensaver', '--saver-root', temporary, '--screenshot', image, ...extra],
    { cwd: root, stdio: 'inherit', timeout: 30000 },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0);
  const stats = JSON.parse(await fs.readFile(path.join(out, `${name}.json`), 'utf8'));
  assert.equal(stats.glyph_failures, 0);
  assert.equal(stats.missing_images, 0);
  assert.equal(stats.glass_background_bytes, 0);
  assert.equal(stats.glass_background_refreshes, 0);
  const { data, info } = await sharp(image)
    .removeAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  assert.equal(info.width, 1920);
  assert.equal(info.height, 1080);
  return { data, info, stats };
}
try {
  await fs.mkdir(path.join(temporary, 'photos'));
  const a = await capture('screensaver-clock-render', settings);
  const b = await capture('screensaver-clock-moved-render', settings, ['--saver-elapsed', '15']);
  const lit = (data, x0, y0, x1, y1) => {
    let count = 0;
    for (let y = y0; y < y1; y++)
      for (let x = x0; x < x1; x++) {
        const i = (y * 1920 + x) * 3;
        if (data[i] > 35 || data[i + 1] > 35 || data[i + 2] > 35) count++;
      }
    return count;
  };
  assert.ok(lit(a.data, 120, 120, 560, 250) > 200, 'Initial moving clock must be visible');
  assert.equal(lit(b.data, 120, 120, 560, 250), 0, 'Old clock position must be cleared');
  assert.ok(lit(b.data, 437, 313, 877, 443) > 200, 'Clock must move after fifteen seconds');
  await sharp({ create: { width: 1920, height: 1080, channels: 3, background: '#2878c8' } })
    .jpeg()
    .toFile(path.join(temporary, 'photos', `${photo}.jpg`));
  const photos = await capture('screensaver-photos-render', {
    ...settings,
    photos: true,
    images: [photo],
  });
  const pixel = (1000 * 1920 + 1000) * 3;
  assert.ok(photos.data[pixel + 2] > 35, 'Photo should remain visible behind the scrim');
  await fs.unlink(path.join(temporary, 'photos', `${photo}.jpg`));
  const fallback = await capture('screensaver-fallback-render', {
    ...settings,
    photos: true,
    images: [photo],
  });
  assert.equal(fallback.data[pixel + 2], 0);
  assert.ok(lit(fallback.data, 120, 120, 560, 250) > 200);
  const preferences = path.join(temporary, 'preferences.json');
  await fs.writeFile(preferences, JSON.stringify({ reduced_motion: true, show_weather: false }));
  const dump = path.join(temporary, 'settings-dump.json');
  const open = `settings,down,right,${Array(9).fill('down').join(',')},enter`;
  const result = spawnSync(
    binary,
    [
      '--preferences',
      preferences,
      '--script',
      `${open},down,down,down,down,enter`,
      '--frames',
      '100',
      '--dump-state',
      dump,
      '--screenshot',
      path.join(out, 'screensaver-settings.png'),
    ],
    { cwd: root, stdio: 'inherit', timeout: 120000 },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0);
  const state = JSON.parse(await fs.readFile(dump, 'utf8'));
  assert.equal(state.subpage, 'SystemScreensaver');
  assert.equal(state.system_saver.report.status, 'unavailable');
  assert.equal(state.system_saver.confirming, false);
  assert.ok(state.ui_text.includes('Apply native screensaver'));
  assert.ok(state.ui_text.includes('System screensaver style'));
  assert.ok(!state.ui_text.includes('Local screensaver style'));
  assert.equal(state.system_saver.report.can_restore, false);
  console.log(
    'Dedicated native saver, moving clock, photos, fallback, no-glass rendering and firmware-gated settings verified.',
  );
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
