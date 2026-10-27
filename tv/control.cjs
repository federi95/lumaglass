// Original Lumaglass adapter. Fixed LS2 protocol references: Glasshouse (MIT), NOTICE.md.
// No agreement edits, service activation, process killing or general shell interface.
var fs = require('fs');
var spawn = require('child_process').spawnSync;
var blocker = require('./blocker.cjs');
/* BLOCKER_SOURCE */
var root = '/var/lib/lumaglass-controls';
var locked = false;
var result = {
  returnValue: true,
  schema: 1,
  verified: false,
  blocker: 'unknown',
  restore: [false, false]
};
function checked(path, directory) {
  var stat = fs.lstatSync(path);
  if (
    stat.isSymbolicLink() ||
    stat.uid !== 0 ||
    stat.mode & 18 ||
    (directory ? !stat.isDirectory() : !stat.isFile())
  )
    throw Error('Unsafe state');
}
function acquire() {
  fs.mkdirSync(root + '/lock', 448);
  fs.writeFileSync(root + '/lock/owner', String(process.pid), { mode: 384, flag: 'wx' });
}
function release() {
  fs.unlinkSync(root + '/lock/owner');
  fs.rmdirSync(root + '/lock');
}
function save(name, value) {
  var temporary = root + '/' + name + '.new';
  var fd = fs.openSync(temporary, 'wx', 384);
  try {
    fs.writeFileSync(fd, JSON.stringify(value));
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
  fs.renameSync(temporary, root + '/' + name);
  fd = fs.openSync(root, 'r');
  try {
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
}
function load(name) {
  var path = root + '/' + name;
  try {
    checked(path, false);
    var data = JSON.parse(fs.readFileSync(path, 'utf8'));
    if (!data || typeof data !== 'object' || Array.isArray(data)) throw Error('Invalid journal');
    return data;
  } catch (error) {
    if (error.code === 'ENOENT') return {};
    throw error;
  }
}
function callFixed(uri, payload, observation) {
  var reply = spawn(
    '/usr/bin/luna-send',
    ['-n', '1', '-w', observation ? '700' : '1500', 'luna://' + uri, JSON.stringify(payload)],
    {
      encoding: 'utf8',
      timeout: observation ? 900 : 1800,
      maxBuffer: 65536,
      stdio: ['ignore', 'pipe', 'ignore']
    }
  );
  if (reply.error || reply.status !== 0) throw Error('Service unavailable');
  var data = JSON.parse(reply.stdout);
  if (data.returnValue !== true) throw Error('Service refused');
  return data;
}
function luna(method, payload) {
  return callFixed('com.webos.settingsservice/' + method, payload);
}
var sleepValues = ['off', '10', '30', '60', '90', '120'];
function optional(uri, payload) {
  try {
    return callFixed(uri, payload, true);
  } catch (error) {
    void error;
    return null;
  }
}
function audio() {
  var data = optional('com.webos.audio/getVolume', {});
  if (!data) data = optional('com.webos.service.audio/master/getVolume', {});
  var status = data && (data.volumeStatus || data);
  return {
    volume:
      status && typeof status.volume === 'number' && status.volume >= 0 && status.volume <= 100
        ? status.volume
        : null,
    muted:
      status && typeof status.muted === 'boolean'
        ? status.muted
        : status && typeof status.mute === 'boolean'
          ? status.mute
          : null
  };
}
function sleepIndex() {
  var data = luna('getSystemSettings', {
    category: 'time',
    keys: ['sleepTimer'],
    subscribe: false
  });
  var value = data.settings && data.settings.sleepTimer;
  var index = sleepValues.indexOf(String(value));
  return index >= 0 ? index : null;
}
function activePower() {
  var data = optional('com.webos.service.tvpower/power/getPowerState', {});
  return !!(data && ['active', 'screen off'].indexOf(String(data.state).toLowerCase()) >= 0);
}
function quickStatus() {
  var network = optional('com.webos.service.connectionmanager/getStatus', {});
  result.network =
    network && network.wifi && network.wifi.state === 'connected'
      ? 'wifi'
      : network && network.wired && network.wired.state === 'connected'
        ? 'wired'
        : network && network.isInternetConnectionAvailable === false
          ? 'offline'
          : 'unknown';
  var sound = audio();
  result.volume = sound.volume;
  result.muted = sound.muted;
  result.power_active = activePower();
  try {
    result.sleep_index = sleepIndex();
  } catch (error) {
    void error;
    result.sleep_index = null;
  }
}
function read(index) {
  var category = index === 0 ? 'option' : 'general';
  var key = index === 0 ? 'livePlus' : 'lmt';
  var settings = luna('getSystemSettings', {
    category: category,
    keys: [key],
    subscribe: false
  }).settings;
  var value = settings && settings[key];
  if (value !== 'on' && value !== 'off' && value !== true && value !== false)
    throw Error('Unknown setting');
  return { category: category, key: key, raw: value, value: value === true || value === 'on' };
}
try {
  var command = JSON.parse(process.argv[1]);
  if (
    process.getuid() !== 0 ||
    !command ||
    typeof command.expiresAt !== 'number' ||
    command.expiresAt < Date.now() ||
    command.expiresAt > Date.now() + 10000
  )
    throw Error('Expired intent');
  if (command.kind === 'blockerStatus' || command.kind === 'quickStatus') {
    result.blocker = blocker.run('status').blocker;
    if (fs.existsSync(root)) checked(root, true);
    var journal = fs.existsSync(root) ? load('privacy.json') : {};
    result.restore = ['livePlus', 'lmt'].map(function (key) {
      return Object.prototype.hasOwnProperty.call(journal, key);
    });
    result.verified = true;
    if (command.kind === 'quickStatus') quickStatus();
  } else {
    if (!fs.existsSync(root)) fs.mkdirSync(root, 448);
    checked(root, true);
    acquire();
    locked = true;
    var saved = load('privacy.json');
    if (command.kind === 'standby') {
      if (!activePower()) throw Error('Power state unavailable');
      callFixed('com.webos.service.tvpower/power/powerOff', { reason: 'remoteKey' });
      // An acknowledgement cannot prove standby; never retry this mutation.
      result.accepted = true;
    } else if (command.kind === 'audioMute') {
      var soundBefore = audio();
      if (
        typeof command.expected !== 'boolean' ||
        typeof command.value !== 'boolean' ||
        soundBefore.muted !== command.expected
      )
        throw Error('Audio state changed');
      callFixed('com.webos.audio/setMuted', { muted: command.value });
      result.verified = audio().muted === command.value;
    } else if (command.kind === 'sleepTimer') {
      if (
        command.expected !== sleepIndex() ||
        !Number.isInteger(command.value) ||
        command.value < 0 ||
        command.value >= sleepValues.length
      )
        throw Error('Unsupported sleep timer');
      luna('setSystemSettings', {
        category: 'time',
        settings: { sleepTimer: sleepValues[command.value] }
      });
      result.verified = sleepIndex() === command.value;
    } else if (command.kind === 'privacySet' || command.kind === 'privacyRestore') {
      if (command.index !== 0 && command.index !== 1) throw Error('Invalid setting');
      var current = read(command.index);
      var wanted;
      if (command.kind === 'privacySet') {
        if (
          typeof command.expected !== 'boolean' ||
          typeof command.value !== 'boolean' ||
          current.value !== command.expected
        )
          throw Error('Setting changed');
        wanted = typeof current.raw === 'boolean' ? command.value : command.value ? 'on' : 'off';
        if (!Object.prototype.hasOwnProperty.call(saved, current.key)) {
          saved[current.key] = { original: current.raw, applied: wanted };
        } else {
          if (
            saved[current.key].applied !== current.raw &&
            saved[current.key].original !== current.raw
          )
            throw Error('External change');
          saved[current.key].applied = wanted;
        }
      } else {
        var backup = saved[current.key];
        if (backup && [true, false, 'on', 'off'].indexOf(backup.original) < 0)
          throw Error('Invalid original value');
        if (!backup || (backup.applied !== current.raw && backup.original !== current.raw))
          throw Error('No owned value to restore');
        wanted = backup.original;
      }
      // Journal the intent before a potentially ambiguous timeout; never retry a write.
      save('privacy.json', saved);
      var update = {};
      update[current.key] = wanted;
      if (current.raw !== wanted)
        luna('setSystemSettings', { category: current.category, settings: update });
      result.verified = read(command.index).raw === wanted;
      if (result.verified && command.kind === 'privacyRestore') {
        delete saved[current.key];
        save('privacy.json', saved);
      }
    } else if (command.kind === 'blockerEnable' || command.kind === 'blockerDisable') {
      var block = blocker.run(
        command.kind === 'blockerEnable' ? 'enable' : 'disable',
        blockerSource
      );
      result.blocker = block.blocker;
      result.verified = block.blocker === (command.kind === 'blockerEnable' ? 'on' : 'off');
    } else {
      throw Error('Unsupported intent');
    }
    result.restore = ['livePlus', 'lmt'].map(function (key) {
      return Object.prototype.hasOwnProperty.call(saved, key);
    });
  }
} catch (error) {
  void error;
  // Do not return payloads, settings values or private service diagnostics.
  result.verified = false;
} finally {
  if (locked) release();
}
console.log(JSON.stringify(result));
