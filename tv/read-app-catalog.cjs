// Original Lumaglass read-only adapter. Node 8 / fixed endpoints / no mutations.
var spawn = require('child_process').spawnSync;
function read(uri, payload, limit) {
  var result = spawn(
    '/usr/bin/luna-send',
    ['-n', '1', '-w', '1200', uri, JSON.stringify(payload)],
    {
      encoding: 'utf8',
      timeout: 1500,
      maxBuffer: limit,
      stdio: ['ignore', 'pipe', 'ignore']
    }
  );
  if (result.error || result.status !== 0) return null;
  try {
    return JSON.parse(String(result.stdout || ''));
  } catch (error) {
    if (!(error instanceof SyntaxError)) throw error;
    return null;
  }
}
function access(reply) {
  if (reply && reply.returnValue === true && reply.ready !== false) return 'available';
  var message = String((reply && reply.errorText) || '');
  if (/permission|access denied|not permitted|not allowed/i.test(message)) return 'denied';
  if (/not exist|unknown service|not registered|not found/i.test(message)) return 'unavailable';
  return 'unknown';
}
var inventory = read('luna://com.webos.applicationManager/listApps', {}, 2 * 1024 * 1024);
var sdk = read(
  'luna://com.webos.service.tv.systemproperty/getSystemInfo',
  { keys: ['sdkVersion'] },
  65536
);
var inventoryAccess = access(inventory);
var apps = [];
if (inventoryAccess === 'available') {
  if (!Array.isArray(inventory.apps) || inventory.apps.length > 1000) inventoryAccess = 'unknown';
  else
    apps = inventory.apps.map(function (app) {
      var entry = {};
      [
        'id',
        'title',
        'systemApp',
        'visible',
        'removable',
        'folderPath',
        'version',
        'packageId'
      ].forEach(function (key) {
        if (app && Object.prototype.hasOwnProperty.call(app, key)) entry[key] = app[key];
      });
      return entry;
    });
}
console.log(
  JSON.stringify({
    returnValue: true,
    schema: 1,
    inventoryAccess: inventoryAccess,
    rootAccess:
      typeof process.getuid === 'function'
        ? process.getuid() === 0
          ? 'available'
          : 'denied'
        : 'unknown',
    sdk: access(sdk) === 'available' ? sdk.sdkVersion : null,
    apps: apps
  })
);
