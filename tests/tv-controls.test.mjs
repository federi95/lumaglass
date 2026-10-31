import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import vm from 'node:vm';
import test from 'node:test';

const source = await fs.readFile(new URL('../tv/control.cjs', import.meta.url), 'utf8');
function fixture({ value = 'on', refuse = false, respond } = {}) {
  const files = new Map();
  const descriptors = new Map();
  const writes = [];
  let next = 1;
  let creates = 0;
  const fake = {
    existsSync: (path) => files.has(path),
    lstatSync: () => ({
      uid: 0,
      mode: 0o700,
      isSymbolicLink: () => false,
      isDirectory: () => true,
      isFile: () => true,
    }),
    mkdirSync(path) {
      if (files.has(path)) throw Error('locked');
      files.set(path, '');
      creates++;
    },
    rmdirSync: (path) => files.delete(path),
    unlinkSync: (path) => files.delete(path),
    readFileSync: (path) => {
      if (!files.has(path)) throw Object.assign(Error('missing'), { code: 'ENOENT' });
      return files.get(path);
    },
    openSync(path, mode) {
      if (mode === 'wx' && files.has(path)) throw Error('exists');
      const fd = next++;
      descriptors.set(fd, path);
      return fd;
    },
    writeFileSync: (fd, bytes) =>
      files.set(typeof fd === 'string' ? fd : descriptors.get(fd), bytes),
    fsyncSync() {},
    closeSync() {},
    renameSync(from, to) {
      files.set(to, files.get(from));
      files.delete(from);
    },
  };
  function run(command) {
    let output;
    vm.runInNewContext(
      source,
      {
        process: {
          argv: ['node', JSON.stringify({ expiresAt: Date.now() + 8000, ...command })],
          getuid: () => 0,
          pid: 4242,
        },
        console: {
          log: (line) => {
            output = JSON.parse(line);
          },
        },
        require(name) {
          if (name === 'fs') return fake;
          if (name === './blocker.cjs') return { run: () => ({ blocker: 'off' }) };
          assert.equal(name, 'child_process');
          return {
            spawnSync(_binary, args) {
              const payload = JSON.parse(args[5]);
              if (respond) return { status: 0, stdout: JSON.stringify(respond(args[4], payload)) };
              if (args[4].endsWith('/setSystemSettings')) {
                writes.push(payload);
                if (refuse) return { status: 0, stdout: '{"returnValue":false}' };
                value = Object.values(payload.settings)[0];
              }
              return {
                status: 0,
                stdout: JSON.stringify({
                  returnValue: true,
                  settings: { livePlus: value, lmt: value },
                }),
              };
            },
          };
        },
      },
      { timeout: 1000 },
    );
    return output;
  }
  return { run, writes, creates: () => creates };
}
test('privacy writes one fixed setting, verifies and restores the original value', () => {
  const f = fixture();
  assert.equal(
    f.run({ kind: 'privacySet', index: 0, expected: true, value: false }).verified,
    true,
  );
  assert.deepEqual(f.writes, [{ category: 'option', settings: { livePlus: 'off' } }]);
  assert.equal(f.run({ kind: 'privacyRestore', index: 0 }).verified, true);
  assert.deepEqual(f.writes[1], { category: 'option', settings: { livePlus: 'on' } });
});
test('stale, unknown, expired and non-allowlisted settings never write', () => {
  for (const command of [
    { kind: 'privacySet', index: 0, expected: false, value: true },
    { kind: 'privacySet', index: 2, expected: true, value: false },
    { kind: 'privacySet', index: 0, expected: true, value: false, expiresAt: 0 },
  ]) {
    const f = fixture();
    assert.equal(f.run(command).verified, false);
    assert.equal(f.writes.length, 0);
  }
  const unknown = fixture({ value: 'unknown' });
  assert.equal(
    unknown.run({ kind: 'privacySet', index: 0, expected: true, value: false }).verified,
    false,
  );
  assert.equal(unknown.writes.length, 0);
});
test('refused mutations are not retried and already-restored values need no write', () => {
  const f = fixture({ refuse: true });
  assert.equal(
    f.run({ kind: 'privacySet', index: 0, expected: true, value: false }).verified,
    false,
  );
  assert.equal(f.writes.length, 1);
  assert.equal(f.run({ kind: 'privacyRestore', index: 0 }).verified, true);
  assert.equal(f.writes.length, 1);
});
test('status creates no persistent directories or locks', () => {
  const f = fixture();
  assert.equal(f.run({ kind: 'blockerStatus' }).blocker, 'off');
  assert.equal(f.creates(), 0);
  assert.equal(f.writes.length, 0);
});

function quickFixture({
  muted = false,
  timer = 'off',
  power = 'Active',
  unavailable = false,
  refuse = false,
} = {}) {
  const calls = [];
  const f = fixture({
    respond(uri, payload) {
      calls.push({ uri, payload });
      if (unavailable) return { returnValue: false };
      if (uri.endsWith('/getStatus')) return { returnValue: true, wifi: { state: 'connected' } };
      if (uri.endsWith('/getVolume'))
        return { returnValue: true, volumeStatus: { volume: 35, muted } };
      if (uri.endsWith('/getPowerState')) return { returnValue: true, state: power };
      if (uri.endsWith('/getSystemSettings'))
        return { returnValue: true, settings: { sleepTimer: timer } };
      if (refuse) return { returnValue: false };
      if (uri.endsWith('/setMuted')) muted = payload.muted;
      if (uri.endsWith('/setSystemSettings')) timer = payload.settings.sleepTimer;
      return { returnValue: true };
    },
  });
  return {
    ...f,
    calls,
    mutations: () =>
      calls.filter(({ uri }) => /\/(setMuted|setSystemSettings|powerOff)$/.test(uri)),
  };
}
test('quick status is read-only, bounded and never invents an SSID or exposes addresses', () => {
  const f = quickFixture();
  const report = f.run({ kind: 'quickStatus' });
  assert.equal(report.network, 'wifi');
  assert.equal(report.volume, 35);
  assert.equal(report.muted, false);
  assert.equal(report.power_active, true);
  assert.equal(report.sleep_index, 0);
  assert.equal(f.creates(), 0);
  assert.equal(f.mutations().length, 0);
  assert.deepEqual(
    Object.keys(report).filter((k) => /address|ip$/.test(k)),
    [],
  );
  assert.equal(report.ssid, null);
  const unknown = quickFixture({ unavailable: true }).run({ kind: 'quickStatus' });
  assert.equal(unknown.muted, null);
  assert.equal(unknown.sleep_index, null);
  assert.equal(unknown.power_active, false);
});
test('mute rechecks the expected state, sends one write and verifies readback', () => {
  const f = quickFixture();
  assert.equal(f.run({ kind: 'audioMute', expected: false, value: true }).verified, true);
  assert.equal(f.mutations().length, 1);
  assert.deepEqual(f.mutations()[0], {
    uri: 'luna://com.webos.audio/setMuted',
    payload: { muted: true },
  });
  assert.equal(f.run({ kind: 'audioMute', expected: false, value: true }).verified, false);
  assert.equal(f.mutations().length, 1);
  const refused = quickFixture({ refuse: true });
  assert.equal(refused.run({ kind: 'audioMute', expected: false, value: true }).verified, false);
  assert.equal(refused.mutations().length, 1);
});
test('LG sleep timer accepts only fixed slots and never writes a stale or unknown timer', () => {
  const f = quickFixture();
  assert.equal(f.run({ kind: 'sleepTimer', expected: 0, value: 3 }).verified, true);
  assert.deepEqual(f.mutations()[0].payload, { category: 'time', settings: { sleepTimer: '60' } });
  assert.equal(f.run({ kind: 'sleepTimer', expected: 0, value: 2 }).verified, false);
  assert.equal(f.mutations().length, 1);
  for (const value of [-1, 6, 1.5, '1', null]) {
    const invalid = quickFixture();
    assert.equal(invalid.run({ kind: 'sleepTimer', expected: 0, value }).verified, false);
    assert.equal(invalid.mutations().length, 0);
  }
  const unknown = quickFixture({ timer: '15' });
  assert.equal(unknown.run({ kind: 'sleepTimer', expected: 0, value: 1 }).verified, false);
  assert.equal(unknown.mutations().length, 0);
});
test('standby is a single acknowledged request and never claims verified completion', () => {
  const f = quickFixture();
  const report = f.run({ kind: 'standby' });
  assert.equal(report.accepted, true);
  assert.equal(report.verified, false);
  assert.deepEqual(f.mutations(), [
    { uri: 'luna://com.webos.service.tvpower/power/powerOff', payload: { reason: 'remoteKey' } },
  ]);
  const unavailable = quickFixture({ power: 'unknown' });
  assert.equal(unavailable.run({ kind: 'standby' }).verified, false);
  assert.equal(unavailable.mutations().length, 0);
});

test('quick Wi-Fi status exposes only a bounded cleaned SSID and performs no writes', () => {
  const f = fixture({
    respond(uri) {
      if (uri.endsWith('/getStatus'))
        return {
          returnValue: true,
          wifi: { state: 'connected', ssid: 'Demo\u0000 Wi-Fi' },
          isInternetConnectionAvailable: true,
        };
      if (uri.endsWith('/getVolume'))
        return { returnValue: true, volumeStatus: { volume: 25, mute: false } };
      if (uri.endsWith('/getPowerState')) return { returnValue: true, state: 'Active' };
      return { returnValue: true, settings: { sleepTimer: 'off' } };
    },
  });
  const result = f.run({ kind: 'quickStatus' });
  assert.equal(result.network, 'wifi');
  assert.equal(result.ssid, 'Demo Wi-Fi');
  assert.equal(f.writes.length, 0);
  assert.equal(f.creates(), 0);
});
