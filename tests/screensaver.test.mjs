// CI only. A redirected temporary filesystem and fake LS2/mount executables.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import test from 'node:test';
import vm from 'node:vm';
const source = fs.readFileSync(new URL('../tv/screensaver.cjs', import.meta.url), 'utf8');
const id = 'com.webos.app.screensaver';
const target = `/usr/palm/applications/${id}`;
const root = '/var/lib/lumaglass-screensaver';
const install = '/media/cryptofs/apps/usr/palm/applications/org.lumaglass.app';
const cache = '/media/internal/.org.lumaglass.app/preferences.wallpapers';
const photo = 'personal-1234567890abcdef';
const stock = { id, type: 'qml', main: 'qml/main.qml', vendor: 'LG', panelPolicy: 'keep' };
const profile = {
  model: 'FixtureTV',
  firmware: '1.2.3',
  sdk: '10.0.0',
  stockType: 'qml',
  stockMain: 'qml/main.qml',
  nativeLifecycle: 1,
};
const config = { schema: 1, photos: false, hour_24: true, language: 'en', images: [] };
function fixture(t, options = {}) {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'lumaglass-saver-'));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const state = {
    mounted: false,
    foreign: false,
    rootDenied: false,
    releaseFails: false,
    stale: false,
    power: 'Active',
    ...options,
  };
  const calls = [];
  const writes = [];
  function actual(name) {
    if (state.mounted && (name === target || name.startsWith(`${target}/`)))
      name = `${root}/app${name.slice(target.length)}`;
    return path.join(temp, name);
  }
  function put(name, value) {
    const file = path.join(temp, name);
    fs.mkdirSync(path.dirname(file), { recursive: true, mode: 0o755 });
    fs.writeFileSync(
      file,
      Buffer.isBuffer(value) ? value : typeof value === 'string' ? value : JSON.stringify(value),
      { mode: 0o600 },
    );
  }
  put('/var/lib/.fixture', '');
  put(`${target}/appinfo.json`, stock);
  put(`${install}/appinfo.json`, { id: 'org.lumaglass.app', main: 'lumaglass', type: 'native' });
  const binary = Buffer.alloc(52);
  Buffer.from('7f454c46', 'hex').copy(binary);
  binary[4] = 1;
  binary[5] = 1;
  binary.writeUInt16LE(40, 18);
  put(`${install}/lumaglass`, binary);
  fs.chmodSync(actual(`${install}/lumaglass`), 0o755);
  fs.mkdirSync(actual(`${install}/assets`));
  fs.mkdirSync(actual(`${install}/lib`));
  put(`${cache}/${photo}.jpg`, Buffer.from('fixture-photo'));
  put('/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema', {
    properties: { id: {}, forceBlockSystemApp: {} },
  });
  function mounts() {
    put(
      '/proc/self/mountinfo',
      state.mounted || state.foreign
        ? `1 0 0:1 ${root}/app ${state.nested ? `${target}/qml` : target} rw - ext4 /dev/root rw\n`
        : '',
    );
  }
  mounts();
  function write(name) {
    assert.ok(name === root || name.startsWith(`${root}/`), `Unexpected mutation ${name}`);
    writes.push(name);
    return actual(name);
  }
  const fake = {
    constants: fs.constants,
    existsSync: (name) => fs.existsSync(actual(name)),
    lstatSync(name) {
      const stat = fs.lstatSync(actual(name));
      return Object.assign(Object.create(stat), { uid: 0 });
    },
    fstatSync: (fd) => fs.fstatSync(fd),
    openSync(name, flags, mode) {
      if (flags === 'wx') write(name);
      return fs.openSync(actual(name), flags, mode);
    },
    readSync: (...args) => fs.readSync(...args),
    writeFileSync: (...args) => fs.writeFileSync(...args),
    fsyncSync: (fd) => fs.fsyncSync(fd),
    closeSync: (fd) => fs.closeSync(fd),
    mkdirSync: (name, mode) => fs.mkdirSync(write(name), mode),
    rmdirSync: (name) => fs.rmdirSync(write(name)),
    unlinkSync: (name) => fs.unlinkSync(write(name)),
    renameSync: (from, to) => fs.renameSync(write(from), write(to)),
    chmodSync: (name, mode) => fs.chmodSync(write(name), mode),
    symlinkSync: (dest, name) => fs.symlinkSync(actual(dest), write(name)),
    readlinkSync: (name) => fs.readlinkSync(actual(name)).slice(temp.length),
  };
  let output;
  const context = {
    Buffer,
    module: { exports: {} },
    process: {
      argv: ['node', JSON.stringify({ id, action: 'inspect', config })],
      pid: 4321,
      getuid: () => (state.rootDenied ? 1000 : 0),
      kill(pid, signal) {
        assert.equal(signal, 0);
        assert.equal(pid, 8888);
        if (state.deadLock) throw Object.assign(Error('gone'), { code: 'ESRCH' });
      },
    },
    console: {
      log: (value) => {
        output = JSON.parse(value);
      },
      error() {},
    },
    require(name) {
      if (name === 'fs') return fake;
      if (name === 'path') return path;
      if (name === 'crypto') return crypto;
      assert.equal(name, 'child_process');
      return {
        spawnSync(binary, args) {
          calls.push({ binary, args: Array.from(args) });
          if (binary === '/bin/mount') {
            assert.deepEqual(Array.from(args), ['--bind', `${root}/app`, target]);
            assert.equal(state.foreign, false);
            state.mounted = true;
            mounts();
            return state.mountTimeout ? { error: Error('timeout') } : { status: 0 };
          }
          if (binary === '/bin/umount') {
            assert.deepEqual(Array.from(args), [target]);
            assert.equal(state.foreign, false);
            state.mounted = false;
            mounts();
            return { status: 0 };
          }
          assert.equal(binary, '/usr/bin/luna-send');
          const method = args[4].split('/').at(-1);
          const payload = JSON.parse(args[5]);
          let reply = { returnValue: true };
          if (method === 'getSystemInfo')
            reply = {
              ...reply,
              modelName: profile.model,
              firmwareVersion: state.newFirmware ? '2.0.0' : profile.firmware,
              sdkVersion: profile.sdk,
            };
          else if (method === 'getAppInfo')
            reply.appInfo =
              state.mounted && state.stale
                ? stock
                : JSON.parse(fs.readFileSync(actual(`${target}/appinfo.json`), 'utf8'));
          else if (method === 'blockSystemApp') {
            assert.deepEqual(payload, { id, forceBlockSystemApp: false });
            if (state.blockTimeout) {
              state.blockTimeout = false;
              return { error: Error('timeout') };
            }
          } else if (method === 'releaseBlockSystemApp') {
            assert.deepEqual(payload, { id });
            reply.returnValue = !state.releaseFails;
          } else if (method === 'getPowerState') reply.state = state.power;
          else if (method === 'turnOnScreenSaver') {
            if (!state.startRefused) state.power = 'Screen Saver';
          } else assert.fail(`Unexpected service ${method}`);
          return { status: 0, stdout: JSON.stringify(reply) };
        },
      };
    },
  };
  vm.runInNewContext(source, context);
  const adapter = context.module.exports.createAdapter([profile]);
  const request = (action, extra = {}) => ({ id, action, config, ...extra });
  return {
    temp,
    state,
    calls,
    writes,
    put,
    actual,
    context,
    output,
    adapter,
    request,
    apply(extra = {}) {
      const q = request('inspect', extra);
      const report = adapter.dispatch(q);
      return adapter.dispatch({
        ...q,
        action: 'apply',
        plan: report.plan,
        expiresAt: Date.now() + 8000,
      });
    },
  };
}
test('production profile registry is closed and inspection is read-only', (t) => {
  const f = fixture(t);
  assert.equal(f.output.status, 'unverified');
  assert.equal(f.writes.length, 0);
  const a = f.context.module.exports.createAdapter([]);
  const report = a.dispatch(f.request('inspect'));
  assert.equal(report.status, 'unverified');
  assert.throws(
    () => a.dispatch(f.request('apply', { plan: report.plan, expiresAt: Date.now() + 8000 })),
    /unverified/,
  );
  assert.equal(f.writes.length, 0);
  assert.equal(
    f.calls.some((c) => c.binary === '/bin/mount'),
    false,
  );
});
test('qualified staging preserves LG fields, snapshots photos and restores only its mount', (t) => {
  const f = fixture(t);
  const result = f.apply({
    config: { ...config, photos: true, images: [photo] },
    photo_root: cache,
  });
  assert.equal(result.status, 'owned');
  const manifest = JSON.parse(fs.readFileSync(f.actual(`${root}/app/appinfo.json`), 'utf8'));
  assert.equal(manifest.type, 'native');
  assert.equal(manifest.main, 'lumaglass-screensaver');
  assert.equal(manifest.nativeLifeCycleInterfaceVersion, 1);
  assert.equal(manifest.panelPolicy, 'keep');
  assert.equal(
    fs.readFileSync(f.actual(`${root}/app/photos/${photo}.jpg`), 'utf8'),
    'fixture-photo',
  );
  assert.deepEqual(
    JSON.parse(fs.readFileSync(path.join(f.temp, target, 'appinfo.json'), 'utf8')),
    stock,
  );
  const restore = f.adapter.dispatch(f.request('restore'));
  assert.equal(restore.status, 'restored');
  assert.equal(f.state.mounted, false);
  assert.equal(f.calls.filter((c) => c.binary === '/bin/umount').length, 1);
});
test('stale manifests, snapshots and expired intents cannot mount', (t) => {
  const f = fixture(t);
  let q = f.request('inspect');
  let report = f.adapter.dispatch(q);
  f.put(`${target}/appinfo.json`, { ...stock, version: 'changed' });
  assert.throws(
    () =>
      f.adapter.dispatch({
        ...q,
        action: 'apply',
        plan: report.plan,
        expiresAt: Date.now() + 8000,
      }),
    /unverified/,
  );
  q = f.request('inspect', {
    config: { ...config, photos: true, images: [photo] },
    photo_root: cache,
  });
  report = f.adapter.dispatch(q);
  f.put(`${cache}/${photo}.jpg`, 'different photo');
  assert.throws(
    () =>
      f.adapter.dispatch({
        ...q,
        action: 'apply',
        plan: report.plan,
        expiresAt: Date.now() + 8000,
      }),
    /unverified/,
  );
  assert.throws(
    () =>
      f.adapter.dispatch({ ...q, action: 'apply', plan: report.plan, expiresAt: Date.now() - 1 }),
    /expired/,
  );
  assert.equal(f.writes.length, 0);
});
test('foreign and nested mounts, queued Glasshouse and non-root sessions are refused', (t) => {
  const f = fixture(t, { foreign: true });
  assert.equal(f.adapter.dispatch(f.request('inspect')).status, 'conflict');
  f.state.nested = true;
  f.put('/proc/self/mountinfo', `1 0 0:1 /foreign ${target}/qml rw - ext4 root rw\n`);
  assert.equal(f.adapter.dispatch(f.request('inspect')).status, 'conflict');
  f.put('/proc/self/mountinfo', '');
  f.put('/var/lib/tvweb/screensaver-stock-type', 'qml');
  assert.equal(f.adapter.dispatch(f.request('inspect')).status, 'conflict');
  f.state.rootDenied = true;
  assert.equal(f.adapter.dispatch(f.request('inspect')).status, 'root-required');
  assert.throws(() => f.adapter.dispatch(f.request('restore')), /root-required/);
  assert.equal(f.writes.length, 0);
});
test('ambiguous bind, block timeout and stale SAM readback rollback without restart', (t) => {
  for (const options of [{ mountTimeout: true }, { blockTimeout: true }, { stale: true }]) {
    const f = fixture(t, options);
    assert.equal(f.apply().status, 'restored');
    assert.equal(f.state.mounted, false);
    assert.ok(f.calls.some((c) => c.binary === '/bin/umount'));
    if (options.blockTimeout)
      assert.ok(f.calls.some((c) => c.args[4]?.endsWith('/releaseBlockSystemApp')));
  }
});
test('release failures remain pending and recover after firmware changes without a new block', (t) => {
  const f = fixture(t, { releaseFails: true });
  assert.equal(f.apply().status, 'restored-pending');
  assert.equal(f.state.mounted, false);
  f.state.releaseFails = false;
  f.state.newFirmware = true;
  const count = f.calls.filter((c) => c.args[4]?.endsWith('/blockSystemApp')).length;
  assert.equal(f.adapter.dispatch(f.request('restore')).status, 'restored');
  assert.equal(f.calls.filter((c) => c.args[4]?.endsWith('/blockSystemApp')).length, count);
});
test('ownership tampering and live locks never unmount or terminate another process', (t) => {
  const f = fixture(t);
  f.apply();
  f.put(`${root}/app/settings.json`, { ...config, hour_24: false });
  assert.throws(() => f.adapter.dispatch(f.request('restore')), /conflict/);
  assert.equal(f.state.mounted, true);
  assert.equal(
    f.calls.some((c) => c.binary === '/bin/umount'),
    false,
  );
  const g = fixture(t);
  g.put(`${root}/lock/owner.json`, { owner: 'org.lumaglass.app', pid: 8888 });
  assert.throws(() => g.apply(), /conflict/);
  assert.equal(g.state.mounted, false);
  g.state.deadLock = true;
  assert.equal(g.apply().status, 'owned');
});
test('only active LG policy requests are allowed; rejected starts never launch an app', (t) => {
  const f = fixture(t);
  assert.equal(
    f.adapter.dispatch(f.request('request', { expiresAt: Date.now() + 8000 })).status,
    'requested',
  );
  f.state.power = 'Standby';
  assert.throws(
    () => f.adapter.dispatch(f.request('request', { expiresAt: Date.now() + 8000 })),
    /unconfirmed/,
  );
  f.state.power = 'Active';
  f.state.startRefused = true;
  assert.equal(
    f.adapter.dispatch(f.request('request', { expiresAt: Date.now() + 8000 })).status,
    'unconfirmed',
  );
  assert.equal(f.writes.length, 0);
});
test('photo inputs reject traversal, duplicate IDs, symlinks, unapproved roots and oversized files', (t) => {
  const f = fixture(t);
  for (const extra of [
    { config: { ...config, photos: true, images: ['../private'] }, photo_root: cache },
    { config: { ...config, photos: true, images: [photo, photo] }, photo_root: cache },
    { config: { ...config, photos: true, images: [photo] }, photo_root: '/tmp/private' },
  ])
    assert.throws(() => f.adapter.dispatch(f.request('inspect', extra)), /conflict/);
  fs.truncateSync(f.actual(`${cache}/${photo}.jpg`), 8 * 1024 * 1024 + 1);
  assert.throws(
    () =>
      f.adapter.dispatch(
        f.request('inspect', {
          config: { ...config, photos: true, images: [photo] },
          photo_root: cache,
        }),
      ),
    /oversized/,
  );
  fs.unlinkSync(f.actual(`${cache}/${photo}.jpg`));
  fs.symlinkSync(f.actual(`${install}/lumaglass`), f.actual(`${cache}/${photo}.jpg`));
  assert.throws(
    () =>
      f.adapter.dispatch(
        f.request('inspect', {
          config: { ...config, photos: true, images: [photo] },
          photo_root: cache,
        }),
      ),
    /unsafe/,
  );
  assert.equal(f.writes.length, 0);
});
test('standalone recovery dispatches its documented file invocation', (t) => {
  const f = fixture(t);
  f.apply();
  f.context.__filename = `${root}/screensaver.cjs`;
  f.context.process.argv = ['node', `${root}/screensaver.cjs`, 'restore'];
  vm.runInNewContext(fs.readFileSync(f.actual(`${root}/screensaver.cjs`), 'utf8'), f.context);
  assert.equal(f.state.mounted, false);
});

test('recovery preserves updated stock files and releases its interrupted block without a refresh', (t) => {
  const f = fixture(t);
  f.apply();
  const updated = { ...stock, main: 'qml/updated.qml', version: '2' };
  f.put(`${target}/appinfo.json`, updated);
  f.state.newFirmware = true;
  const journal = JSON.parse(fs.readFileSync(f.actual(`${root}/state.json`), 'utf8'));
  journal.blockIntent = true;
  f.put(`${root}/state.json`, journal);
  const blocks = f.calls.filter((c) => c.args[4]?.endsWith('/blockSystemApp')).length;
  assert.equal(f.adapter.dispatch(f.request('restore')).status, 'restored');
  assert.deepEqual(
    JSON.parse(fs.readFileSync(f.actual(`${target}/appinfo.json`), 'utf8')),
    updated,
  );
  assert.equal(f.calls.filter((c) => c.args[4]?.endsWith('/blockSystemApp')).length, blocks);
  assert.equal(
    JSON.parse(fs.readFileSync(f.actual(`${root}/state.json`), 'utf8')).stockChanged,
    true,
  );
});

test('a newly qualified firmware still cannot refresh a transaction from an older firmware', (t) => {
  const f = fixture(t);
  f.apply();
  f.state.newFirmware = true;
  const adapter = f.context.module.exports.createAdapter([
    profile,
    { ...profile, firmware: '2.0.0' },
  ]);
  const blocks = f.calls.filter((c) => c.args[4]?.endsWith('/blockSystemApp')).length;
  assert.equal(adapter.dispatch(f.request('restore')).status, 'restored');
  assert.equal(f.calls.filter((c) => c.args[4]?.endsWith('/blockSystemApp')).length, blocks);
});
