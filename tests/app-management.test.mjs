import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

const source = await fs.readFile(new URL('../tv/manage-app.cjs', import.meta.url), 'utf8');
const home = { id: 'com.webos.app.home' };
const app = {
  id: 'org.example.player',
  removable: true,
  systemApp: false,
  version: '1.0.0',
  folderPath: '/media/developer/apps/usr/palm/applications/org.example.player',
};
const target = {
  id: app.id,
  folder: app.folderPath,
  version: app.version,
  route: 'developer',
  package: null,
};

function run({
  mode = 'remove',
  selected = target,
  apps = [home, app],
  timeout = false,
  expiresAt = Date.now() + 8000,
} = {}) {
  const calls = [];
  let output;
  vm.runInNewContext(
    source,
    {
      process: {
        argv: ['node', JSON.stringify({ mode, target: selected, expiresAt })],
        getuid: () => 0,
      },
      console: {
        log: (text) => {
          output = JSON.parse(text);
        },
        error() {},
      },
      require(name) {
        assert.equal(name, 'child_process');
        return {
          spawnSync(binary, args) {
            assert.equal(binary, '/usr/bin/luna-send');
            const uri = args[4];
            assert.ok(JSON.parse(args[5]));
            calls.push(uri);
            if (uri.endsWith('/listApps'))
              return { status: 0, stdout: JSON.stringify({ returnValue: true, apps }) };
            assert.equal(uri, 'luna://com.webos.appInstallService/dev/remove');
            if (timeout) return { error: new Error('timeout') };
            return { status: 0, stdout: '{"returnValue":true}' };
          },
        };
      },
    },
    { timeout: 2000 },
  );
  return { calls, output };
}

test('removal rechecks metadata and sends exactly one preselected request', () => {
  const { calls, output } = run();
  assert.equal(output.status, 'accepted');
  assert.deepEqual(calls, [
    'luna://com.webos.applicationManager/listApps',
    'luna://com.webos.appInstallService/dev/remove',
  ]);
});

test('unknown flags, changed versions and protected IDs prevent a mutation', () => {
  for (const changed of [
    { ...app, removable: 'true' },
    { ...app, systemApp: true },
    { ...app, version: '2.0.0' },
  ]) {
    const result = run({ apps: [home, changed] });
    assert.equal(result.output.status, 'refused');
    assert.equal(result.calls.length, 1);
  }
  const protectedResult = run({ selected: { ...target, id: 'org.webosbrew.hbchannel' } });
  assert.equal(protectedResult.output.status, 'refused');
  assert.equal(protectedResult.calls.length, 0);
});

test('a timed out mutation is not retried or sent to a fallback endpoint', () => {
  const result = run({ timeout: true });
  assert.equal(result.output.status, 'unknown');
  assert.equal(result.calls.length, 2);
});

test('expired or implausibly distant removal intents never reach LS2', () => {
  for (const expiresAt of [Date.now() - 1, Date.now() + 60000, null]) {
    const result = run({ expiresAt });
    assert.equal(result.output.status, 'refused');
    assert.equal(result.calls.length, 0);
  }
});

test('changed, protected and shared package identities prevent removal', () => {
  const packaged = { ...app, packageId: 'org.example.bundle' };
  const selected = { ...target, package: packaged.packageId };
  for (const options of [
    { apps: [home, packaged] },
    {
      selected: { ...selected, package: 'org.webosbrew.hbchannel' },
      apps: [home, { ...packaged, packageId: 'org.webosbrew.hbchannel' }],
    },
    {
      selected,
      apps: [home, packaged, { id: 'org.example.companion', packageId: packaged.packageId }],
    },
  ]) {
    const result = run(options);
    assert.equal(result.output.status, 'refused');
    assert.deepEqual(result.calls, ['luna://com.webos.applicationManager/listApps']);
  }
});

test('verification requires a valid complete inventory and never mutates', () => {
  assert.equal(run({ mode: 'verify', apps: [] }).output.status, 'unknown');
  assert.equal(run({ mode: 'verify', apps: [home, { id: 123 }] }).output.status, 'unknown');
  assert.equal(run({ mode: 'verify' }).output.status, 'present');
  const absent = run({ mode: 'verify', apps: [home] });
  assert.equal(absent.output.status, 'removed');
  assert.deepEqual(absent.calls, ['luna://com.webos.applicationManager/listApps']);
});
