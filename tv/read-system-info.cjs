// Read-only Lumaglass helper. Fixed service/file reads; no settings, mount,
// process-control or filesystem mutations. Keep compatible with webOS Node 8.
var fs = require('fs');
var spawnSync = require('child_process').spawnSync;
var homeMountsReader = require('./home-mounts.cjs');

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

var home = 'unknown';
try {
  home = homeMountsReader.snapshot(fs).home;
} catch (error) {
  console.error(error.message);
}
var systemAccess = access(info);
var homeBoot = 'unknown';
try {
  var boot = homeMountsReader.boot(fs);
  homeBoot = boot.foreign ? 'other' : boot.uncertain ? 'unknown' : 'clear';
} catch (error) {
  console.error(error.message);
}
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
    home: home,
    homeBoot: homeBoot
  })
);
