// Original Lumaglass hosts adapter; protocol/domain references: Glasshouse (MIT), NOTICE.md.
// Independent recovery: node /var/lib/lumaglass-blocker/blocker.cjs disable
var fs = require('fs');
var crypto = require('crypto');
var spawn = require('child_process').spawnSync;
var stateDir = '/var/lib/lumaglass-blocker';
var hook = '/var/lib/webosbrew/init.d/65-lumaglass-blocker';
var hookBody = '#!/bin/sh\nexec /usr/bin/node /var/lib/lumaglass-blocker/blocker.cjs boot\n';
var revision = '2026-10-lg-minimal-1';
var domains = [
  'ad.lgsmartad.com',
  'lgsmartad.com',
  'smartad.lge.com',
  'ibis.lgappstv.com',
  'ibs.lgappstv.com',
  'rdx.lgtvcommon.com',
  'aic.lgtvcommon.com',
  'prov-lg.alphonso.tv',
  'eulacheck.alphonso.tv',
  'acrdb.alphonso.tv',
  'ads.alphonso.tv'
];
['info.lgsmartad.com', 'rdx2.lgtvsdp.com', 'cdpbeacon.lgtvcommon.com'].forEach(function (name) {
  ['', 'aic.', 'eic.', 'kic.', 'eu.', 'it.'].forEach(function (prefix) {
    domains.push(prefix + name);
  });
});
function acquire() {
  fs.mkdirSync(stateDir + '/lock', 448);
  fs.writeFileSync(stateDir + '/lock/owner', String(process.pid), { mode: 384, flag: 'wx' });
}
function release() {
  fs.unlinkSync(stateDir + '/lock/owner');
  fs.rmdirSync(stateDir + '/lock');
}
function hash(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
function safe(path, directory) {
  var info = fs.lstatSync(path);
  if (
    info.isSymbolicLink() ||
    info.uid !== 0 ||
    info.mode & 18 ||
    (directory ? !info.isDirectory() : !info.isFile())
  )
    throw Error('Unsafe ownership');
  return info;
}
function mounts() {
  return fs
    .readFileSync('/proc/self/mountinfo', 'utf8')
    .trim()
    .split('\n')
    .filter(function (line) {
      return line.split(' ')[4] === '/etc/hosts';
    });
}
function owned() {
  var rows = mounts();
  if (rows.length !== 1) return false;
  try {
    var from = safe(stateDir + '/hosts', false);
    var to = safe('/etc/hosts', false);
    var journal = saved();
    return (
      journal.enabled &&
      from.size <= 600000 &&
      from.ino === to.ino &&
      from.dev === to.dev &&
      hash(fs.readFileSync(stateDir + '/hosts')) === journal.hash
    );
  } catch (error) {
    return error && false;
  }
}
function command(binary, args) {
  var reply = spawn(binary, args, { timeout: 2500, stdio: 'ignore' });
  if (reply.error || reply.status !== 0) throw Error('Mount operation failed');
}
function atomic(path, bytes, mode) {
  var fd = fs.openSync(path + '.new', 'wx', mode);
  try {
    fs.writeFileSync(fd, bytes);
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
  fs.renameSync(path + '.new', path);
  fd = fs.openSync(require('path').dirname(path), 'r');
  try {
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
}
function saved() {
  safe(stateDir, true);
  safe(stateDir + '/state.json', false);
  var data = JSON.parse(fs.readFileSync(stateDir + '/state.json', 'utf8'));
  if (data.schema !== 1 || typeof data.enabled !== 'boolean') throw Error('Invalid blocker state');
  return data;
}
function status() {
  var rows = mounts();
  if (rows.length) return owned() ? 'on' : 'conflict';
  return 'off';
}
function foreignBootEnabled() {
  var flag = '/var/lib/tvweb/adblock_enabled';
  return fs.existsSync(flag) && String(fs.readFileSync(flag, 'utf8')).trim().length > 0;
}
function run(action, source) {
  if (process.getuid() !== 0) throw Error('Root required');
  if (['status', 'enable', 'disable', 'boot'].indexOf(action) < 0) throw Error('Invalid action');
  if (action === 'status') return { blocker: status(), revision: revision, count: domains.length };
  if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, 448);
  safe(stateDir, true);
  acquire();
  try {
    var active = status();
    if (active === 'conflict') throw Error('Foreign or stacked hosts mount');
    if (fs.existsSync(hook)) {
      safe(hook, false);
      if (fs.readFileSync(hook, 'utf8') !== hookBody) throw Error('Foreign startup hook');
    }
    if (action === 'disable') {
      if (active === 'on') {
        command('/bin/umount', ['/etc/hosts']);
        if (mounts().length) throw Error('Hosts still mounted');
      }
      // No changes to the underlying LG hosts file or another integration.
      if (fs.existsSync(hook)) fs.unlinkSync(hook);
      if (fs.existsSync(stateDir + '/state.json')) {
        var old = saved();
        old.enabled = false;
        atomic(stateDir + '/state.json', JSON.stringify(old), 384);
      }
      return { blocker: status(), revision: revision, count: domains.length };
    }
    if (foreignBootEnabled()) throw Error('Glasshouse blocker is configured for startup');
    if (active === 'on') return { blocker: 'on', revision: revision, count: domains.length };
    if (action === 'boot') {
      var persisted = saved();
      if (!persisted.enabled) return { blocker: 'off' };
      safe(stateDir + '/hosts', false);
      if (hash(fs.readFileSync(stateDir + '/hosts')) !== persisted.hash)
        throw Error('Hosts integrity failure');
    } else {
      safe('/etc/hosts', false);
      var original = fs.readFileSync('/etc/hosts');
      if (original.length > 524288 || original.indexOf(0) >= 0) throw Error('Invalid hosts');
      if (
        /lumaglass|LG Ad & Telemetry Blackhole|lg-webos-dashboard|lg-webos-mqtt/i.test(
          original.toString()
        )
      )
        throw Error('Existing blocker entries');
      if (typeof source !== 'string' || source.length > 65536)
        throw Error('Missing recovery source');
      safe('/var/lib/webosbrew/init.d', true);
      var content =
        original.toString().replace(/\n?$/, '\n') +
        '# Lumaglass ' +
        revision +
        '\n' +
        domains
          .map(function (name) {
            return '0.0.0.0 ' + name + '\n:: ' + name;
          })
          .join('\n') +
        '\n';
      atomic(stateDir + '/original-hosts', original, 384);
      atomic(stateDir + '/hosts', content, 420);
      atomic(stateDir + '/blocker.cjs', source, 448);
      atomic(
        stateDir + '/state.json',
        JSON.stringify({ schema: 1, enabled: true, revision: revision, hash: hash(content) }),
        384
      );
      atomic(hook, hookBody, 448);
    }
    command('/bin/mount', ['--bind', stateDir + '/hosts', '/etc/hosts']);
    if (!owned()) throw Error('Mount ownership unverified');
    return { blocker: 'on', revision: revision, count: domains.length };
  } finally {
    release();
  }
}
module.exports = { run: run };
if (require.main === module) {
  try {
    console.log(JSON.stringify(run(process.argv[2], fs.readFileSync(__filename, 'utf8'))));
  } catch (error) {
    console.error('Lumaglass blocker refused: ' + error.message);
    process.exitCode = 1;
  }
}
