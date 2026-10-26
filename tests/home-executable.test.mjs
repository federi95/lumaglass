import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import homeMountsReader from '../tv/home-mounts.cjs';

const script = await fs.readFile(new URL('../tv/home.cjs', import.meta.url), 'utf8');
const exe = '/usr/bin/com.webos.app.home';
const appDir = '/media/developer/apps/usr/palm/applications/org.lumaglass.app';
const marker = '/var/run/lumaglass-home';
const lgHome = {
  id: 'com.webos.app.home',
  type: 'native',
  main: `file://${exe}`,
  nativeLifeCycleInterfaceVersion: 2,
};

// Runs home.cjs in a sandbox. Each test replaces the system-facing helpers it reaches.
function load() {
  const unexpected = () => assert.fail('Tests must not invoke any system or TV command');
  const context = vm.createContext({
    require(name) {
      if (name === './home-mounts.cjs') return homeMountsReader;
      if (name === 'fs') return { existsSync: () => false };
      if (name === 'path') return path.posix;
      assert.equal(name, 'child_process');
      return { execFileSync: unexpected, spawnSync: unexpected };
    },
    process: { argv: ['node', 'home.cjs', 'status'] },
    console: { log() {}, error() {} },
  });
  vm.runInContext(script, context, { timeout: 2000 });
  return context;
}

// A TV with Lumaglass's executable mountable over LG Home's; `front` lists foreground replies.
function tv({ root = null, same = false, front = ['com.webos.app.home'] } = {}) {
  const context = load();
  const calls = [];
  const state = { root, same, fronts: [...front], marker: false };
  Object.assign(context, {
    bootIntegrations: () => ({ foreign: false, uncertain: false }),
    fs: {
      existsSync: (file) => file === `${appDir}/lumaglass` || (file === marker && state.marker),
      writeFileSync(file, data) {
        assert.equal(file, marker);
        assert.equal(String(data), 'org.lumaglass.app\n');
        calls.push('mark');
        state.marker = true;
      },
      unlinkSync(file) {
        assert.equal(file, marker);
        calls.push('unmark');
        state.marker = false;
      },
      readFileSync(file) {
        assert.equal(file, marker);
        return 'org.lumaglass.app\n';
      },
    },
    homeMounts: () =>
      homeMountsReader.inspect(
        `1 0 0:1 / / rw - rootfs rootfs rw\n${state.root ? `2 1 0:1 ${state.root} ${exe} rw - rootfs rootfs rw\n` : ''}`,
      ),
    exeMount: () => state.root,
    exeMounted: () => state.same,
    run(file, args) {
      calls.push([path.posix.basename(file), ...args].join(' '));
      if (file === '/bin/mount') {
        state.root = '/developer/apps/usr/palm/applications/org.lumaglass.app/lumaglass';
        state.same = true;
      }
      if (file === '/bin/umount') {
        state.root = null;
        state.same = false;
      }
      return '';
    },
    foreground: () => (state.fronts.length > 1 ? state.fronts.shift() : state.fronts[0]),
    stopHome: () => calls.push('stop'),
    launchHome: () => calls.push('launch'),
    pause() {},
  });
  return { context, calls, state };
}

test('webOS 5/6 with a native lifecycle-v2 LG Home uses the executable method', () => {
  const { homeMethod } = load();
  assert.equal(homeMethod('Rockhopper release 5.6.2-21 (jhericurl-jimna)', lgHome), 'executable');
  assert.equal(homeMethod('Rockhopper release 6.5.3-10', lgHome), 'executable');
  assert.equal(homeMethod('Rockhopper release 10.2.2-58', null), 'manifest');
  assert.equal(homeMethod('Rockhopper release 11.2.0-1', null), 'manifest');
  assert.throws(() => homeMethod('Rockhopper release 9.2.1', lgHome), /targets webOS/);
  assert.throws(
    () => homeMethod('Rockhopper release 5.6.2', { ...lgHome, nativeLifeCycleInterfaceVersion: 1 }),
    /targets webOS/,
  );
  assert.throws(
    () => homeMethod('Rockhopper release 6.1.0', { ...lgHome, type: 'web' }),
    /targets webOS/,
  );
});

test('only a mount of Lumaglass executable counts as this integration, also after an update', () => {
  const { exeOwned } = load();
  assert.ok(exeOwned('/developer/apps/usr/palm/applications/org.lumaglass.app/lumaglass'));
  assert.ok(exeOwned('/cryptofs/apps/usr/palm/applications/org.lumaglass.app/lumaglass//deleted'));
  assert.ok(!exeOwned('/var/lib/other/launcher'));
  assert.ok(!exeOwned(null));
});

test('apply mounts the executable, stops the old Home and opens Lumaglass', () => {
  const { context, calls } = tv({ front: ['some.app', 'com.webos.app.home'] });
  context.applyExecutable({ method: 'executable', appDir }, true);
  assert.deepEqual(calls, [`mount --bind ${appDir}/lumaglass ${exe}`, 'mark', 'stop', 'launch']);
});

test('a boot apply does not take the screen from another app', () => {
  const { context, calls } = tv({ front: ['com.webos.app.livetv'] });
  context.applyExecutable({ method: 'executable', appDir }, false);
  assert.deepEqual(calls, [`mount --bind ${appDir}/lumaglass ${exe}`, 'mark', 'stop']);
});

test('after a package update the replaced file is unmounted before mounting the new one', () => {
  const { context, calls } = tv({
    root: '/developer/apps/usr/palm/applications/org.lumaglass.app/lumaglass//deleted',
  });
  context.applyExecutable({ method: 'executable', appDir }, true);
  assert.deepEqual(calls, [
    `umount ${exe}`,
    `mount --bind ${appDir}/lumaglass ${exe}`,
    'mark',
    'stop',
    'launch',
  ]);
});

test('another integration mounted over LG Home is left alone', () => {
  const { context, calls } = tv({ root: '/var/lib/other/launcher' });
  assert.throws(
    () => context.applyExecutable({ method: 'executable', appDir }, true),
    /Another integration/,
  );
  assert.deepEqual(calls, []);
});

test('Home is launched again once and LG Home restored when Lumaglass never comes to the front', () => {
  const { context, calls, state } = tv({ front: ['com.webos.app.home', 'com.webos.app.livetv'] });
  assert.throws(
    () => context.applyExecutable({ method: 'executable', appDir }, true),
    /Home did not come to the front/,
  );
  assert.deepEqual(calls, [
    `mount --bind ${appDir}/lumaglass ${exe}`,
    'mark',
    'stop',
    'launch',
    'launch',
    'unmark',
    `umount ${exe}`,
    'stop',
  ]);
  assert.equal(state.root, null);
  assert.equal(state.marker, false);
});

test('restore unmounts, stops Lumaglass and reopens LG Home only if Home was in front', () => {
  for (const [front, reopened] of [
    ['com.webos.app.home', true],
    ['youtube.leanback.v4', false],
  ]) {
    const { context, calls, state } = tv({
      root: '/developer/apps/usr/palm/applications/org.lumaglass.app/lumaglass',
      same: true,
      front: [front],
    });
    state.marker = true;
    context.restore({ method: 'executable', appDir });
    assert.deepEqual(calls, ['unmark', `umount ${exe}`, 'stop', ...(reopened ? ['launch'] : [])]);
  }
});

test('explicit recovery of an own executable mount survives corrupt state', () => {
  const { context, calls } = tv({ root: `${appDir}/lumaglass`, front: ['youtube.leanback.v4'] });
  const originalExists = context.fs.existsSync;
  // A merely configured foreign startup must not prevent restoring our own mount.
  context.bootIntegrations = () => ({ foreign: true, uncertain: false });
  context.fs.existsSync = (file) =>
    file === '/var/lib/lumaglass-home/state.json' || originalExists(file);
  context.fs.readFileSync = () => {
    throw new SyntaxError('Corrupt state');
  };
  context.hookOwned = () => true;
  context.fs.unlinkSync = (file) => {
    assert.equal(file, '/var/lib/webosbrew/init.d/64-lumaglass-home');
    calls.push('unhook');
  };
  context.writeJson = (file, saved) => {
    assert.equal(file, '/var/lib/lumaglass-home/state.json');
    assert.equal(saved.enabled, false);
    assert.equal(saved.method, 'executable');
    calls.push('disable-state');
  };
  context.recover();
  assert.deepEqual(calls, ['unhook', 'disable-state', `umount ${exe}`, 'stop']);
});

test('explicit recovery refuses foreign ownership before changing hooks or state', () => {
  const { context, calls } = tv({ root: '/var/lib/other/launcher' });
  context.hookOwned = () => assert.fail('No hook mutation path is allowed');
  context.writeJson = () => assert.fail('No state writes are allowed');
  assert.throws(() => context.recover(), /Another integration/);
  assert.deepEqual(calls, []);
});

test('a foreign startup configuration prevents apply before mounts or app changes', () => {
  const { context, calls } = tv();
  context.bootIntegrations = () => ({ foreign: true, uncertain: false });
  assert.throws(() => context.applyExecutable({ method: 'executable', appDir }, true), /startup/);
  assert.deepEqual(calls, []);
});
