// Original Lumaglass read-only adapter; no writes or ACR/ad service requests.
// Fixed key/path references: Glasshouse 8e252164 (MIT), see NOTICE.md.
// Keep ES5 syntax for webOS Node 8. Return only allowlisted boolean observations.
var fs = require('fs');
var spawnSync = require('child_process').spawnSync;

function booleanValue(value) {
  if (value === true || value === 'on') return 'on';
  if (value === false || value === 'off') return 'off';
  return 'unknown';
}

function setting(category, key) {
  var result = spawnSync(
    '/usr/bin/luna-send',
    [
      '-n',
      '1',
      '-w',
      '1200',
      'luna://com.webos.settingsservice/getSystemSettings',
      JSON.stringify({ category: category, keys: [key], subscribe: false })
    ],
    { encoding: 'utf8', timeout: 1500, maxBuffer: 64 * 1024, stdio: ['ignore', 'pipe', 'ignore'] }
  );
  if (result.error || result.status !== 0) return 'unknown';
  try {
    var reply = JSON.parse(String(result.stdout || ''));
    if (!reply || typeof reply !== 'object' || Array.isArray(reply)) return 'unknown';
    if (reply.returnValue === true) return booleanValue(reply.settings && reply.settings[key]);
    if (reply.returnValue !== false) return 'unknown';
    var error = String(reply.errorText || '');
    if (/permission|access denied|not permitted|not allowed/i.test(error)) return 'denied';
    if (/no matched result|unknown service|not registered|service.*not exist/i.test(error))
      return 'unavailable';
  } catch (error) {
    // An invalid reply is not evidence of an off value or of missing support.
    if (!(error instanceof SyntaxError)) throw error;
  }
  return 'unknown';
}

function consentFile() {
  var fd;
  try {
    fd = fs.openSync('/var/luna/preferences/eula', 'r');
    var bytes = Buffer.alloc(256 * 1024 + 1);
    var size = 0;
    while (size < bytes.length) {
      var count = fs.readSync(fd, bytes, size, bytes.length - size, null);
      if (!count) break;
      size += count;
    }
    if (size === bytes.length) return { status: 'unknown' };
    var parsed = JSON.parse(bytes.slice(0, size).toString('utf8'));
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
      return { status: 'unknown' };
    return { status: 'available', values: parsed };
  } catch (error) {
    return {
      status:
        error.code === 'EACCES' || error.code === 'EPERM'
          ? 'denied'
          : error.code === 'ENOENT'
            ? 'unavailable'
            : 'unknown'
    };
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
}

var values = { livePlus: setting('option', 'livePlus'), lmt: setting('general', 'lmt') };
var consent = consentFile();
[
  'acrAllowed',
  'acrOnAllowed',
  'customAdAllowed',
  'customadsAllowed',
  'cookiesAllowed',
  'remoteDiagAllowed',
  'thirdPartySharingAllowed'
].forEach(function (key) {
  // Consent records accept actual JSON booleans only; do not coerce strings/numbers.
  var value = consent.values && consent.values[key];
  values[key] =
    consent.status === 'available'
      ? typeof value === 'boolean'
        ? booleanValue(value)
        : 'unknown'
      : consent.status;
});
console.log(JSON.stringify({ returnValue: true, schema: 1, values: values }));
