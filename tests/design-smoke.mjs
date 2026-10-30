// CI-only visual and interaction coverage for the tvOS-inspired native restyle.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import sharp from 'sharp';
import { spawnSync } from 'node:child_process';
import { buildNative } from '../tools/ci/native-smoke.mjs';
import { root } from '../tools/lib/project.mjs';
buildNative();
const out = path.join(root, '.build');
const temporary = await fs.mkdtemp(path.join(out, 'design-smoke-'));
const preferences = path.join(temporary, 'preferences.json');
const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
async function scenario(name, script, config = {}) {
  await fs.writeFile(
    preferences,
    JSON.stringify({ reduced_motion: true, show_weather: false, ...config }),
  );
  const dump = path.join(temporary, `${name}.json`);
  const report = path.join(out, `${name}-render.json`);
  const screenshot = path.join(out, `${name}.png`);
  const result = spawnSync(
    binary,
    [
      '--preferences',
      preferences,
      '--script',
      script,
      '--frames',
      '140',
      '--screenshot',
      screenshot,
      '--diagnostics',
      report,
      '--dump-state',
      dump,
    ],
    { cwd: root, stdio: 'inherit', timeout: 90000 },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0);
  const state = JSON.parse(await fs.readFile(dump, 'utf8'));
  const render = JSON.parse(await fs.readFile(report, 'utf8'));
  assert.equal(render.capture.glyph_failures, 0);
  assert.equal(render.capture.missing_glyphs, 0);
  assert.equal(render.capture.missing_images, 0);
  return { state, render, screenshot };
}
function control(state, target) {
  const c = state.controls.find((c) => JSON.stringify(c.target) === JSON.stringify(target));
  assert.ok(c, `Missing target ${JSON.stringify(target)}`);
  return c;
}
try {
  const rootPage = await scenario('design-settings-root', 'settings,down');
  assert.equal(rootPage.state.settings_category, false);
  for (let i = 0; i < 5; i++) {
    const rect = control(rootPage.state, { Sidebar: i }).rect;
    assert.equal(rect[0], 1032);
    assert.equal(rect[2], 800);
  }
  const general = await scenario('design-settings-general', 'settings,down,enter');
  assert.equal(general.state.settings_category, true);
  assert.equal(control(general.state, { Setting: 0 }).rect[0], 1032);
  const { data, info } = await sharp(general.screenshot)
    .removeAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  const pixel = (x, y) => (y * info.width + x) * info.channels;
  const offset = pixel(1058, 236);
  assert.ok(
    data[offset] > 175 && data[offset + 1] > 175 && data[offset + 2] > 175,
    'Focused list item must use a light capsule',
  );
  const background = await scenario('design-background', 'settings,tap:{"Sidebar":1}');
  assert.equal(background.state.section, 1);
  assert.equal(control(background.state, { Background: 0 }).rect[0], 1032);
  await scenario('design-apps', 'apps,down');
  const dock = await scenario('design-home-dock', 'home,down', { home: { favorites_dock: true } });
  assert.equal(dock.state.preferences.home.favorites_dock, true);
  assert.equal(dock.state.ui_text.includes('Favorites'), false);
  const centered = await scenario('design-profile-centered', 'tap:"Profile"');
  const profileRect = control(centered.state, 'ProfileName').rect;
  assert.equal(profileRect[0] + profileRect[2] / 2, 960);
  assert.equal(
    centered.state.controls.some((c) => String(c.target).includes('CloseProfile')),
    false,
  );
  const outside = await scenario('design-profile-outside', 'tap:"Profile",click:100:540');
  assert.equal(outside.state.modal, false);
  assert.equal(outside.state.focus, 'Profile');
  const quickOutside = await scenario('design-center-outside', 'control-center,click:100:540');
  assert.equal(quickOutside.state.modal, false);
  const quickToggle = await scenario('design-center-toggle', 'control-center,tap:"ControlCenter"');
  assert.equal(quickToggle.state.modal, false);
  const quickProfile = await scenario('design-center-profile', 'control-center,tap:"Profile"');
  assert.ok(control(quickProfile.state, 'ProfileName'));
  const quickPanel = await scenario('design-center-targets', 'control-center');
  assert.equal(
    quickPanel.state.controls.some((c) => String(c.target).includes('CloseControlCenter')),
    false,
  );
  for (const glass of ['off', 'light', 'full']) {
    for (const [name, script] of [
      ['control-center', 'control-center'],
      ['profile', 'tap:"Profile"'],
      ['appearance', 'settings,tap:{"Sidebar":0},tap:{"Setting":12}'],
      ['options', 'apps,down,menu'],
    ]) {
      const result = await scenario(`design-${name}-${glass}`, script, {
        appearance: { liquid_glass: glass },
      });
      if (glass === 'off') {
        assert.equal(result.render.capture.glass_background_bytes, 0);
        assert.equal(result.render.capture.glass_background_refreshes, 0);
      } else {
        assert.ok(
          result.render.capture.glass_background_bytes <= (glass === 'light' ? 7372800 : 16588800),
        );
      }
    }
  }
  const profile = await scenario('design-profile-action', 'tap:"Profile",enter');
  assert.equal(profile.state.modal, true);
  assert.equal(profile.state.subpage, null);
  const back = await scenario('design-settings-back', 'settings,down,enter,back');
  assert.equal(back.state.settings_category, false);
  assert.deepEqual(back.state.focus, { Sidebar: 0 });
  console.log(
    'Two-pane settings, native taps, focused capsules, dock, popovers, profile and Off/Light/Full GPU budgets verified.',
  );
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
