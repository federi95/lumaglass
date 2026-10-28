// CI-only: every path is redirected to a temporary tree; no real LS2/mount calls.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';

const source = fs.readFileSync(new URL('../tv/system-apps.cjs', import.meta.url), 'utf8');
const id = 'com.webos.optional.demo';
const root = '/var/lib/lumaglass-system-apps';
const folder = `/usr/palm/applications/${id}`;
const target = `${folder}/appinfo.json`;
const profile = { model: 'FixtureTV', firmware: '1.2.3', sdk: '10.0.0', apps: [id] };
function fixture(t, options = {}) {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'lumaglass-system-apps-'));
  t.after(() => fs.rmSync(temporary, { recursive: true, force: true }));
  const state = {
    mounted: false,
    foreign: false,
    releaseFails: false,
    staleReadback: false,
    deadLock: false,
    ...options,
  };
  const calls = [];
  const writes = [];
  function real(name) {
    if (name === target && state.mounted) name = `${root}/${id}/override.json`;
    return path.join(temporary, name);
  }
  function put(name, value) {
    const file = path.join(temporary, name);
    fs.mkdirSync(path.dirname(file), { recursive: true, mode: 0o755 });
    fs.writeFileSync(file, typeof value === 'string' ? value : JSON.stringify(value), {
      mode: 0o600,
    });
  }
  put(target, { id, visible: true, type: 'web', main: 'index.html', vendor: 'LG' });
  put('/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema', {
    properties: { id: {}, forceBlockSystemApp: { type: 'boolean' } },
  });
  fs.mkdirSync(real('/var/lib'), { recursive: true, mode: 0o755 });
  const original = fs.readFileSync(real(target));
  function updateMounts() {
    const mount = state.foreign ? folder : target;
    put(
      '/proc/self/mountinfo',
      state.mounted || state.foreign
        ? `1 0 0:1 ${root}/${id}/override.json ${mount} rw - ext4 /dev/root rw\n`
        : '',
    );
  }
  updateMounts();
  function writable(name) {
    assert.ok(
      name === root || name.startsWith(`${root}/`),
      `Unexpected filesystem mutation: ${name}`,
    );
    writes.push(name);
    return real(name);
  }
  const fake = {
    constants: fs.constants,
    existsSync: (name) => fs.existsSync(real(name)),
    lstatSync(name) {
      const stat = fs.lstatSync(real(name));
      return Object.assign(Object.create(stat), { uid: 0 });
    },
    fstatSync: (fd) => fs.fstatSync(fd),
    openSync(name, flags, mode) {
      if (flags === 'wx') writable(name);
      return fs.openSync(real(name), flags, mode);
    },
    readSync: (...args) => fs.readSync(...args),
    writeFileSync: (...args) => fs.writeFileSync(...args),
    fsyncSync: (fd) => fs.fsyncSync(fd),
    closeSync: (fd) => fs.closeSync(fd),
    mkdirSync: (name, mode) => fs.mkdirSync(writable(name), mode),
    rmdirSync: (name) => fs.rmdirSync(writable(name)),
    unlinkSync: (name) => fs.unlinkSync(writable(name)),
    renameSync: (from, to) => fs.renameSync(writable(from), writable(to)),
  };
  let output;
  const context = {
    Buffer,
    module: { exports: {} },
    process: {
      argv: ['node', JSON.stringify({ action: 'inspect', id })],
      pid: 4321,
      getuid: () => (state.rootDenied ? 1000 : 0),
      kill(pid, signal) {
        assert.equal(signal, 0, 'The adapter may only check liveness');
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
      if (name === 'crypto') return requireCrypto;
      assert.equal(name, 'child_process');
      return {
        spawnSync(binary, args) {
          calls.push({ binary, args });
          if (binary === '/bin/mount') {
            assert.deepEqual(Array.from(args), ['--bind', `${root}/${id}/override.json`, target]);
            assert.equal(state.foreign, false);
            state.mounted = true;
            updateMounts();
            return state.mountTimeout ? { error: Error('timeout') } : { status: 0 };
          }
          if (binary === '/bin/umount') {
            assert.deepEqual(Array.from(args), [target]);
            assert.equal(state.foreign, false);
            state.mounted = false;
            updateMounts();
            return { status: 0 };
          }
          assert.equal(binary, '/usr/bin/luna-send');
          const method = args[4].split('/').at(-1);
          const payload = JSON.parse(args[5]);
          let value = { returnValue: true };
          if (method === 'getSystemInfo')
            value = {
              ...value,
              modelName: profile.model,
              firmwareVersion: profile.firmware,
              sdkVersion: profile.sdk,
            };
          else if (method === 'listApps')
            value.apps = [
              { id: 'com.webos.app.home' },
              { id, systemApp: true, visible: true, folderPath: folder, version: '1.0.0' },
            ];
          else if (method === 'blockSystemApp') {
            assert.equal(payload.id, id);
            assert.equal(payload.forceBlockSystemApp, false);
          } else if (method === 'releaseBlockSystemApp') value.returnValue = !state.releaseFails;
          else if (method === 'getAppInfo')
            value.appInfo = {
              ...JSON.parse(fs.readFileSync(real(target))),
              ...(state.staleReadback ? { visible: true } : {}),
            };
          else assert.fail(`Unexpected endpoint ${method}`);
          return { status: 0, stdout: JSON.stringify(value) };
        },
      };
    },
  };
  vm.runInNewContext(source, context, { timeout: 2000 });
  const adapter = context.module.exports.createAdapter(options.qualified ? [profile] : []);
  function hide() {
    const plan = adapter.inspect(id);
    return adapter.dispatch({ action: 'hide', id, plan: plan.plan, expiresAt: Date.now() + 8000 });
  }
  return {
    adapter,
    hide,
    state,
    calls,
    writes,
    original,
    put,
    real,
    context,
    get output() {
      return output;
    },
  };
}
import requireCrypto from 'node:crypto';

test('production inspection is read-only and an unqualified firmware cannot hide even with injected request fields', (t) => {
  const f = fixture(t);
  assert.equal(f.output.status, 'unverified');
  assert.equal(f.output.firmware, profile.firmware);
  assert.match(f.output.manifestHash, /^[a-f0-9]{64}$/);
  assert.equal(f.writes.length, 0);
  assert.throws(
    () =>
      f.adapter.dispatch({
        action: 'hide',
        id,
        profile,
        verified: true,
        plan: f.output.plan,
        expiresAt: Date.now() + 8000,
      }),
    /unverified/,
  );
  assert.equal(f.writes.length, 0);
  assert.equal(
    f.calls.some((call) => call.binary !== '/usr/bin/luna-send'),
    false,
  );
});

test('a qualified fixture preserves stock bytes, verifies the mount and restores without deleting app files', (t) => {
  const f = fixture(t, { qualified: true });
  assert.equal(f.hide().status, 'owned');
  assert.equal(f.state.mounted, true);
  assert.deepEqual(fs.readFileSync(f.real(`${root}/${id}/original.json`)), f.original);
  assert.ok(
    fs.existsSync(f.real(`${root}/system-apps.cjs`)),
    'Recovery must remain independent of the installed app',
  );
  assert.equal(f.adapter.dispatch({ action: 'restore', id }).status, 'restored');
  assert.equal(f.state.mounted, false);
  assert.deepEqual(fs.readFileSync(f.real(target)), f.original);
  assert.equal(f.hide().status, 'owned', 'A clean restore must permit a later qualified action');
});

test('protected IDs, root denial and foreign ancestor mounts never authorize changes', (t) => {
  const f = fixture(t, { qualified: true });
  for (const protectedId of [
    'com.webos.app.home',
    'com.webos.app.webapphost',
    'airplay',
    'org.lumaglass.app',
  ])
    assert.throws(() => f.adapter.dispatch({ action: 'hide', id: protectedId }), /protected/);
  assert.equal(f.writes.length, 0);
  f.state.rootDenied = true;
  assert.equal(f.adapter.inspect(id).status, 'root-required');
  f.state.rootDenied = false;
  const conflict = fixture(t, { qualified: true, foreign: true });
  assert.equal(conflict.adapter.inspect(id).status, 'conflict');
  assert.throws(conflict.hide, /unverified/);
  assert.equal(conflict.writes.length, 0);
  const boot = fixture(t, { qualified: true });
  boot.put('/var/lib/tvweb/hidden_apps', `${id}\n`);
  assert.equal(boot.adapter.inspect(id).status, 'conflict');
  assert.equal(boot.writes.length, 0);
});

test('expired intents and changed manifests cannot reuse a previously inspected plan', (t) => {
  const f = fixture(t, { qualified: true });
  const plan = f.adapter.inspect(id).plan;
  assert.throws(
    () => f.adapter.dispatch({ action: 'hide', id, plan, expiresAt: Date.now() - 1 }),
    /expired/,
  );
  f.put(target, { id, visible: true, type: 'web', main: 'changed.html' });
  assert.throws(
    () => f.adapter.dispatch({ action: 'hide', id, plan, expiresAt: Date.now() + 8000 }),
    /unverified/,
  );
  assert.equal(
    f.calls.some((call) => call.binary === '/bin/mount'),
    false,
  );
});

test('ambiguous mount results and stale SAM readback roll back rather than claiming a hide', (t) => {
  for (const options of [{ mountTimeout: true }, { staleReadback: true }]) {
    const f = fixture(t, { qualified: true, ...options });
    assert.equal(f.hide().status, 'restored');
    assert.equal(f.state.mounted, false);
    assert.deepEqual(fs.readFileSync(f.real(target)), f.original);
  }
});

test('failed releases remain pending and can be recovered later', (t) => {
  const f = fixture(t, { qualified: true, releaseFails: true });
  assert.equal(f.hide().status, 'restored-pending');
  assert.equal(f.state.mounted, false);
  assert.equal(f.adapter.inspect(id).canRestore, true);
  f.state.releaseFails = false;
  assert.equal(f.adapter.dispatch({ action: 'restore', id }).status, 'restored');
  assert.equal(JSON.parse(fs.readFileSync(f.real(`${root}/${id}/state.json`))).blockIntent, false);
});

test('tampered journals and live lock owners cannot trigger an unmount', (t) => {
  const f = fixture(t, { qualified: true });
  f.hide();
  f.put(`${root}/${id}/state.json`, { owner: 'another-launcher' });
  assert.throws(() => f.adapter.dispatch({ action: 'restore', id }), /conflict/);
  assert.equal(
    f.calls.some((call) => call.binary === '/bin/umount'),
    false,
  );
  const locked = fixture(t, { qualified: true });
  locked.put(`${root}/lock/owner.json`, { owner: 'org.lumaglass.app', pid: 8888 });
  assert.throws(locked.hide, /conflict/);
  assert.equal(
    locked.calls.some((call) => call.binary === '/bin/mount'),
    false,
  );
  locked.state.deadLock = true;
  assert.equal(locked.hide().status, 'owned');
});

test('the documented standalone file invocation restores only the owned mount without a qualified refresh', (t) => {
  const f = fixture(t, { qualified: true });
  assert.equal(f.hide().status, 'owned');
  const blocks = f.calls.filter((call) => call.args[4]?.endsWith('/blockSystemApp')).length;
  f.context.__filename = `${root}/system-apps.cjs`;
  f.context.process.argv = ['node', f.context.__filename, 'restore', id];
  vm.runInNewContext(source, f.context, { timeout: 2000 });
  assert.equal(f.output.status, 'restored-pending');
  assert.equal(f.state.mounted, false);
  assert.deepEqual(fs.readFileSync(f.real(target)), f.original);
  assert.equal(f.calls.filter((call) => call.args[4]?.endsWith('/blockSystemApp')).length, blocks);
});
