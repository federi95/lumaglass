// Read-only Lumaglass helper. Fixed service/file reads; no settings, mount,
// process-control or filesystem mutations. Keep compatible with webOS Node 8.
var fs = require('fs');
var spawnSync = require('child_process').spawnSync;

function read(file, limit) {
  var fd;
  try {
    fd = fs.openSync(file, 'r');
    var bytes = Buffer.alloc(limit + 1);
    var count = 0;
    while (count < bytes.length) {
      var next = fs.readSync(fd, bytes, count, bytes.length - count, null);
      if (!next) break;
      count += next;
    }
    return count > limit ? null : bytes.slice(0, count).toString('utf8');
  } catch (error) {
    if (error.code !== 'ENOENT' && error.code !== 'EACCES') console.error(error.message);
    return null;
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
}

function access(reply) {
  if (reply && reply.returnValue === true) return 'available';
  var error = String((reply && reply.errorText) || '');
  if (/permission|access denied|not permitted|not allowed/i.test(error)) return 'denied';
  if (/not exist|unknown service|not registered|not found/i.test(error)) return 'unavailable';
  return 'unknown';
}

var info = null;
try {
  var result = spawnSync(
    '/usr/bin/luna-send',
    [
      '-n',
      '1',
      '-w',
      '1800',
      'luna://com.webos.service.tv.systemproperty/getSystemInfo',
      '{"keys":["modelName","firmwareVersion","sdkVersion"]}'
    ],
    { encoding: 'utf8', timeout: 2500, maxBuffer: 64 * 1024 }
  );
  if (!result.error && result.status === 0) info = JSON.parse(String(result.stdout || ''));
} catch (error) {
  console.error(error.message);
}

var mounts = read('/proc/self/mountinfo', 256 * 1024);
var home = 'unknown';
if (mounts) {
  home = 'stock';
  mounts.split('\n').forEach(function (line) {
    var fields = line.split(' ');
    var target = fields[4] || '';
    if (
      target === '/usr/bin/com.webos.app.home' ||
      target === '/usr/palm/applications/com.webos.app.home' ||
      target.indexOf('/usr/palm/applications/com.webos.app.home/') === 0
    ) {
      var source = fields[3] || '';
      var own =
        /\/org\.lumaglass\.app\/lumaglass(\/\/deleted)?$/.test(source) ||
        source === '/var/lib/lumaglass-home/app' ||
        source.indexOf('/var/lib/lumaglass-home/app/') === 0;
      // Any foreign layer wins, even if another layer belongs to Lumaglass.
      if (!own) home = 'other';
      else if (home !== 'other') home = 'lumaglass';
    }
  });
}
var systemAccess = access(info);
console.log(
  JSON.stringify({
    returnValue: true,
    schema: 1,
    rootAccess:
      typeof process.getuid === 'function'
        ? process.getuid() === 0
          ? 'available'
          : 'denied'
        : 'unknown',
    systemAccess: systemAccess,
    model: systemAccess === 'available' ? info.modelName : null,
    firmware: systemAccess === 'available' ? info.firmwareVersion : null,
    sdk: systemAccess === 'available' ? info.sdkVersion : null,
    home: home
  })
);
