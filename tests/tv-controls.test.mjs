import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import vm from 'node:vm';
import test from 'node:test';

const source = await fs.readFile(new URL('../tv/control.cjs', import.meta.url), 'utf8');
function fixture({ value = 'on', refuse = false } = {}) {
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
