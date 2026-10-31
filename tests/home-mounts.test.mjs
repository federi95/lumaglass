import assert from 'node:assert/strict';
import test from 'node:test';
import reader from '../tv/home-mounts.cjs';

const target = '/usr/palm/applications/com.webos.app.home';
const stage = '/var/lib/lumaglass-home/app';
const exe = '/usr/bin/com.webos.app.home';
const executable = '/developer/apps/usr/palm/applications/org.lumaglass.app/lumaglass';
const base = '1 0 0:1 / / rw - rootfs rootfs rw\n';
const layer = (id, source, destination) =>
  `${id} 1 0:1 ${source} ${destination} rw - rootfs rootfs rw\n`;

test('Home ownership distinguishes stock and the two exclusive Lumaglass methods', () => {
  assert.equal(reader.inspect(base).home, 'stock');
  for (const mount of [layer(2, stage, target), layer(2, executable, exe)]) {
    const report = reader.inspect(base + mount);
    assert.equal(report.home, 'lumaglass');
    assert.equal(report.conflict, false);
  }
});

test('a foreign nested mount is a conflict even when the parent belongs to Lumaglass', () => {
  const report = reader.inspect(
    base + layer(2, stage, target) + layer(3, '/other/icon', `${target}/icon.png`),
  );
  assert.equal(report.home, 'other');
  assert.equal(report.conflict, true);
});

test('a staged app on a separate filesystem uses that filesystem root and device', () => {
  const mounts = base + layer(2, '/persist', '/var');
  const report = reader.inspect(mounts + layer(3, '/persist/lib/lumaglass-home/app', target));
  assert.equal(report.home, 'lumaglass');
  assert.equal(report.conflict, false);
  const differentDevice = layer(3, '/persist/lib/lumaglass-home/app', target).replace('0:1', '0:2');
  assert.equal(reader.inspect(mounts + differentDevice).conflict, true);
});

test('stacked /var containers follow mount ancestry independently of line order', () => {
  // QNED756RA/webOS 25: ext4 /var covers an overlay at the same mount point.
  const root = '19 1 179:27 / / ro - squashfs /dev/root ro\n';
  const overlay = '59 19 0:49 / /var rw - overlay overlay rw\n';
  const persistent = '74 59 179:57 /var /var rw - ext4 /dev/mmcblk0p57 rw\n';
  const own = `6364 19 179:57 ${stage} ${target} rw - ext4 /dev/mmcblk0p57 rw\n`;
  for (const containers of [root + overlay + persistent, persistent + overlay + root]) {
    assert.equal(reader.inspect(containers).home, 'stock');
    const report = reader.inspect(containers + own);
    assert.equal(report.home, 'lumaglass');
    assert.equal(report.conflict, false);
    assert.equal(reader.inspect(containers + own.replace('179:57', '0:49')).home, 'other');
    assert.equal(reader.inspect(containers + own + layer(77, '/foreign', target)).conflict, true);
    assert.equal(
      reader.inspect(containers + own + layer(78, '/foreign', `${target}/icon.png`)).conflict,
      true,
    );
  }
});

test('same-path containers without provable ancestry remain ambiguous', () => {
  const first = '2 1 0:2 / /var rw - ext4 first rw\n';
  const unrelated = '3 1 0:3 / /var rw - ext4 second rw\n';
  assert.throws(() => reader.inspect(base + first + unrelated), /Ambiguous containing mounts/);
});

test('a lower foreign executable layer is not hidden by a Lumaglass layer on top', () => {
  const report = reader.inspect(
    base + layer(2, '/other/launcher', exe) + layer(3, executable, exe),
  );
  assert.equal(report.home, 'other');
  assert.equal(report.conflict, true);
});

test('stacked own layers and simultaneous methods require manual recovery', () => {
  for (const mounts of [
    layer(2, stage, target) + layer(3, stage, target),
    layer(2, executable, exe) + layer(3, stage, target),
  ]) {
    const report = reader.inspect(base + mounts);
    assert.equal(report.home, 'unknown');
    assert.equal(report.conflict, true);
  }
});

test('mount paths are decoded before ownership checks and invalid replies are rejected', () => {
  const report = reader.inspect(base + layer(2, '/other/with\\040spaces', exe));
  assert.equal(report.executable[0].source, '/other/with spaces');
  assert.equal(report.conflict, true);
  for (const text of ['', 'not mountinfo', 'x'.repeat(256 * 1024 + 1)])
    assert.throws(() => reader.inspect(text));
});

function bootFiles({ luma = false, sharedHook = false, glasshouse, denied = false } = {}) {
  let offset = 0;
  const missing = () => Object.assign(new Error('missing'), { code: 'ENOENT' });
  const bytes = Buffer.from(
    typeof glasshouse === 'string' ? glasshouse : JSON.stringify(glasshouse) || '',
  );
  return {
    lstatSync(file) {
      if (file === '/var/lib/webosbrew/init.d/64-luma-home' && luma) return {};
      if (file === '/var/lib/webosbrew/init.d/50-tvweb.sh' && sharedHook) return {};
      throw missing();
    },
    openSync(file) {
      assert.equal(file, '/var/lib/tvweb/home.json');
      if (denied) throw Object.assign(new Error('denied'), { code: 'EACCES' });
      if (glasshouse === undefined) throw missing();
      return 1;
    },
    readSync(fd, buffer, start, length) {
      assert.equal(fd, 1);
      const count = bytes.copy(buffer, start, offset, offset + length);
      offset += count;
      return count;
    },
    closeSync() {},
    writeFileSync() {
      assert.fail('Startup observations must not write');
    },
  };
}

test('startup detection distinguishes shared Glasshouse service and enabled Home', () => {
  assert.equal(
    reader.boot(bootFiles({ sharedHook: true, glasshouse: { enabled: false } })).foreign,
    false,
  );
  assert.equal(reader.boot(bootFiles({ glasshouse: { enabled: true } })).foreign, true);
  assert.equal(reader.boot(bootFiles({ luma: true })).foreign, true);
  assert.equal(reader.boot(bootFiles()).uncertain, false);
});

test('unreadable, malformed or nonboolean startup records remain uncertain', () => {
  for (const options of [
    { denied: true },
    { glasshouse: '{' },
    { glasshouse: { enabled: 'false' } },
  ]) {
    const report = reader.boot(bootFiles(options));
    assert.equal(report.foreign, false);
    assert.equal(report.uncertain, true);
  }
});
