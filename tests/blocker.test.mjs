import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';
import test from 'node:test';
import vm from 'node:vm';

const source = await fs.readFile(new URL('../tv/blocker.cjs', import.meta.url), 'utf8');
const root = '/var/lib/lumaglass-blocker';
const hook = '/var/lib/webosbrew/init.d/65-lumaglass-blocker';
function fixture({ foreign = false, stacked = false, symlink = false } = {}) {
  const files = new Map([
    ['/etc/hosts', Buffer.from('127.0.0.1 localhost\n10.0.0.1 custom-service\n')],
  ]);
  const directories = new Set(['/var/lib/webosbrew/init.d']);
  const descriptors = new Map();
  let next = 1;
  let mounted = false;
  const calls = [];
  function actual(name) {
    return name === '/etc/hosts' && mounted ? `${root}/hosts` : name;
  }
  const fake = {
    existsSync: (name) => files.has(name) || directories.has(name),
    lstatSync(name) {
      name = actual(name);
      if (!files.has(name) && !directories.has(name))
        throw Object.assign(Error('missing'), { code: 'ENOENT' });
      return {
        uid: 0,
        mode: directories.has(name) ? 0o755 : 0o644,
        size: files.get(name)?.length || 0,
        ino: name,
        dev: 1,
        isDirectory: () => directories.has(name),
        isFile: () => files.has(name),
        isSymbolicLink: () => symlink && name === '/etc/hosts',
      };
    },
    readFileSync(name, encoding) {
      if (name === '/proc/self/mountinfo') {
        const row = `1 0 0:1 ${root}/hosts /etc/hosts rw - ext4 /dev/root rw`;
        return mounted ? `${row}\n` : foreign ? `${row}${stacked ? `\n${row}` : ''}\n` : '';
      }
      const bytes = files.get(actual(name));
      if (!bytes) throw Object.assign(Error('missing'), { code: 'ENOENT' });
      return encoding ? bytes.toString(encoding) : bytes;
    },
    mkdirSync(name) {
      if (directories.has(name)) throw Object.assign(Error('exists'), { code: 'EEXIST' });
      directories.add(name);
    },
    rmdirSync: (name) => directories.delete(name),
    unlinkSync: (name) => files.delete(name),
    openSync(name, mode) {
      if (mode === 'wx' && files.has(name)) throw Error('exists');
      const fd = next++;
      descriptors.set(fd, name);
      return fd;
    },
    closeSync() {},
    fsyncSync() {},
    writeFileSync(fd, bytes) {
      files.set(typeof fd === 'string' ? fd : descriptors.get(fd), Buffer.from(bytes));
    },
    renameSync(from, to) {
      files.set(to, files.get(from));
      files.delete(from);
    },
  };
  const module = { exports: {} };
  vm.runInNewContext(
    source,
    {
      module,
      process: { getuid: () => 0, pid: 4242 },
      console,
      require(name) {
        if (name === 'fs') return fake;
        if (name === 'crypto') return crypto;
        if (name === 'path') return path;
        assert.equal(name, 'child_process');
        return {
          spawnSync(binary, args) {
            calls.push([binary, args]);
            mounted = binary.endsWith('/mount');
            return { status: 0 };
          },
        };
      },
    },
    { timeout: 1000 },
  );
  return { files, calls, run: (action) => module.exports.run(action, source) };
}
test('blocker preserves existing hosts and disables only its verified mount/hook', () => {
  const f = fixture();
  assert.equal(f.run('status').blocker, 'off');
  assert.equal(f.files.has(`${root}/state.json`), false);
  assert.equal(f.run('enable').blocker, 'on');
  assert.match(
    f.files.get(`${root}/hosts`).toString(),
    /^127\.0\.0\.1 localhost\n10\.0\.0\.1 custom-service\n/,
  );
  assert.equal(f.files.has(hook), true);
  assert.equal(f.run('disable').blocker, 'off');
  assert.equal(f.files.has(hook), false);
  assert.equal(
    f.files.get('/etc/hosts').toString(),
    '127.0.0.1 localhost\n10.0.0.1 custom-service\n',
  );
  assert.deepEqual(
    f.calls.map(([binary]) => binary),
    ['/bin/mount', '/bin/umount'],
  );
});
test('foreign/stacked mounts, symlink hosts and foreign hooks refuse mutation', () => {
  for (const options of [{ foreign: true }, { foreign: true, stacked: true }, { symlink: true }]) {
    const f = fixture(options);
    assert.throws(() => f.run('enable'));
    assert.equal(f.calls.length, 0);
  }
  const f = fixture();
  f.files.set(hook, Buffer.from('# foreign owner\n'));
  assert.throws(() => f.run('enable'));
  assert.equal(f.files.get(hook).toString(), '# foreign owner\n');
  assert.equal(f.calls.length, 0);
});
