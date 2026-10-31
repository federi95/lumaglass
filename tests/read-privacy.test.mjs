import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import vm from 'node:vm';
import test from 'node:test';
const source = await fs.readFile(new URL('../tv/read-privacy.cjs', import.meta.url), 'utf8');
function observe(eula, errorCode) {
  const bytes = Buffer.from(JSON.stringify(eula));
  let offset = 0;
  let output;
  const calls = [];
  const readonly = {
    openSync(path, mode) {
      assert.equal(path, '/var/luna/preferences/eula');
      assert.equal(mode, 'r');
      if (errorCode) throw Object.assign(new Error('fixture read refusal'), { code: errorCode });
      return 1;
    },
    readSync(_fd, destination, start, length) {
      const count = bytes.copy(destination, start, offset, offset + length);
      offset += count;
      return count;
    },
    closeSync() {},
    writeFileSync() {
      assert.fail('Privacy observations must never write');
    },
  };
  vm.runInNewContext(
    source,
    {
      Buffer,
      console: {
        log(value) {
          output = JSON.parse(value);
        },
      },
      require(name) {
        if (name === 'fs') return readonly;
        assert.equal(name, 'child_process');
        return {
          spawnSync(binary, args) {
            assert.equal(binary, '/usr/bin/luna-send');
            assert.equal(args[4], 'luna://com.webos.settingsservice/getSystemSettings');
            calls.push(JSON.parse(args[5]));
            return {
              status: 0,
              stdout: JSON.stringify({
                returnValue: true,
                settings: { livePlus: 'on', lmt: 'off' },
              }),
            };
          },
        };
      },
    },
    { timeout: 1000 },
  );
  assert.deepEqual(
    calls.map((c) => [c.category, ...c.keys]),
    [
      ['option', 'livePlus'],
      ['general', 'lmt'],
    ],
  );
  return output;
}
test('webOS 25 nested eulaStatus exposes only the seven allowlisted booleans', () => {
  const report = observe({
    eulaInfo: { eulaList: [] },
    eulaStatus: {
      acrAllowed: true,
      acrOnAllowed: false,
      customAdAllowed: true,
      customadsAllowed: false,
      cookiesAllowed: true,
      remoteDiagAllowed: false,
      thirdPartySharingAllowed: true,
      unrelated: 'fixture-only',
    },
  });
  assert.equal(report.values.livePlus, 'on');
  assert.equal(report.values.lmt, 'off');
  assert.equal(report.values.acrAllowed, 'on');
  assert.equal(report.values.acrOnAllowed, 'off');
  assert.equal(report.values.thirdPartySharingAllowed, 'on');
  assert.equal(Object.hasOwn(report.values, 'unrelated'), false);
  assert.equal(Object.keys(report.values).length, 9);
});
test('legacy roots remain supported; malformed nested status cannot fall back to stale root data', () => {
  assert.equal(observe({ acrAllowed: false }).values.acrAllowed, 'off');
  for (const eulaStatus of [null, [], true, 'on', { acrAllowed: 'true' }]) {
    assert.equal(observe({ acrAllowed: true, eulaStatus }).values.acrAllowed, 'unknown');
  }
});
test('missing and denied consent files remain distinct from off', () => {
  assert.equal(observe({}, 'ENOENT').values.acrAllowed, 'unavailable');
  assert.equal(observe({}, 'EACCES').values.acrAllowed, 'denied');
});
