// Original Lumaglass adapter. Fixed LS2 endpoints; no filesystem deletion,
// shell input, service shutdown or retry of a removal. Keep ES5 / Node 8.
var spawnSync = require('child_process').spawnSync;
var PROTECTED = [
  'org.lumaglass.app',
  'com.ecklf.luma',
  'com.luma.native',
  'com.luma.launcher',
  'com.luma.launcher.native',
  'com.webos.app.home',
  'org.webosbrew.hbchannel',
  'org.webosbrew.inputhook',
  'com.webos.app.livetv',
  'com.webos.app.settings',
  'com.palm.app.settings',
  'com.webos.app.screensaver',
  'com.webos.app.inputpicker',
  'com.webos.app.externalinput',
  'com.tvweb.dashboard',
  'com.tvweb.home',
  'com.webos.app.browser',
  'com.palm.app.browser',
  'com.webos.app.firstuse',
  'com.webos.app.eula',
  'com.webos.app.container',
  'com.webos.app.inputcommon'
];

function validId(id) {
  return (
    typeof id === 'string' &&
    id.length > 0 &&
    id.length <= 128 &&
    /^[a-zA-Z0-9]+([._-][a-zA-Z0-9]+)*$/.test(id)
  );
}
function call(uri, payload) {
  var result = spawnSync(
    '/usr/bin/luna-send',
    ['-n', '1', '-w', '1200', uri, JSON.stringify(payload)],
    {
      encoding: 'utf8',
      timeout: 1500,
      maxBuffer: 2 * 1024 * 1024,
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
function inventory() {
  var reply = call('luna://com.webos.applicationManager/listApps', {});
  if (
    !reply ||
    reply.returnValue !== true ||
    reply.ready === false ||
    !Array.isArray(reply.apps) ||
    reply.apps.length > 1000
  )
    return null;
  if (
    !reply.apps.every(function (app) {
      return app && validId(app.id);
    })
  )
    return null;
  // An empty/filtered response must not prove a successful uninstall.
  if (
    !reply.apps.some(function (app) {
      return app.id === 'com.webos.app.home';
    })
  )
    return null;
  return reply.apps;
}
function currentIntent(request) {
  var now = Date.now();
  return (
    typeof request.expiresAt === 'number' &&
    isFinite(request.expiresAt) &&
    request.expiresAt >= now &&
    request.expiresAt <= now + 10000
  );
}
function manage(request) {
  var target = request && request.target;
  if (
    !target ||
    !validId(target.id) ||
    PROTECTED.indexOf(target.id) !== -1 ||
    target.id.indexOf('org.lumaglass.preview.') === 0 ||
    (request.mode !== 'remove' && request.mode !== 'verify') ||
    typeof process.getuid !== 'function' ||
    process.getuid() !== 0
  )
    return 'refused';
  if (request.mode === 'remove' && !currentIntent(request)) return 'refused';
  var apps = inventory();
  if (!apps) return request.mode === 'remove' ? 'refused' : 'unknown';
  var found = apps.filter(function (app) {
    return app.id === target.id;
  });
  if (!found.length) return 'removed';
  if (request.mode === 'verify') return 'present';
  if (found.length !== 1) return 'refused';
  var app = found[0];
  var packageId = app.packageId == null ? null : app.packageId;
  if (
    packageId !== target.package ||
    (packageId !== null && (!validId(packageId) || PROTECTED.indexOf(packageId) !== -1))
  )
    return 'refused';
  // The service can remove a whole package: refuse known multi-app bundles.
  if (
    packageId !== null &&
    apps.some(function (other) {
      return other.id !== target.id && other.packageId === packageId;
    })
  )
    return 'refused';
  if (
    app.removable !== true ||
    app.systemApp !== false ||
    app.folderPath !== target.folder ||
    (app.version == null ? null : app.version) !== target.version
  )
    return 'refused';
  var developer = '/media/developer/apps/usr/palm/applications/' + target.id;
  var store = '/media/cryptofs/apps/usr/palm/applications/' + target.id;
  var uri;
  if (target.route === 'developer' && target.folder === developer)
    uri = 'luna://com.webos.appInstallService/dev/remove';
  else if (target.route === 'store' && target.folder === store)
    uri = 'luna://com.webos.appInstallService/remove';
  else return 'refused';
  // This is the only mutation. Never fall back to another endpoint on failure.
  if (!currentIntent(request)) return 'refused';
  var result = call(uri, { id: target.id, subscribe: false });
  if (result && result.returnValue === true) return 'accepted';
  if (result && result.returnValue === false) return 'refused';
  return 'unknown';
}

var request;
var status = 'unknown';
try {
  request = JSON.parse(process.argv[1] || 'null');
  status = manage(request);
} catch (error) {
  if (!(error instanceof SyntaxError)) console.error('App management reply unavailable');
}
console.log(
  JSON.stringify({
    returnValue: true,
    schema: 1,
    id: request && request.target && validId(request.target.id) ? request.target.id : '',
    status: status
  })
);
