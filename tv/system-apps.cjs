// Original Lumaglass adapter: no app deletion, SAM restart or automatic boot hook.
(function lumaglassSystemApps() {
  // Original Lumaglass adapter. Node 8 / fixed endpoints / no SAM restart or deletion.
  // Firmware profiles remain empty until exact model/firmware hardware qualification.
  // Standalone recovery: node system-apps.cjs restore APP_ID
  var fs = require('fs');
  var path = require('path');
  var crypto = require('crypto');
  var spawn = require('child_process').spawnSync;
  var ROOT = '/var/lib/lumaglass-system-apps';
  var OWNER = 'org.lumaglass.app';
  var VERIFIED = [];
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
    'com.webos.app.inputcommon',
    'com.webos.app.webapphost',
    'webapphost',
    'container',
    'inputcommon',
    'firstuse',
    'eula',
    'airplay'
  ];
  function reportFailure(error) {
    var known = ['conflict', 'unconfirmed', 'unavailable', 'changed'];
    console.error(
      'System visibility: ' +
        (error && known.indexOf(error.message) !== -1 ? error.message : 'unavailable')
    );
  }
  function valid(id) {
    return (
      typeof id === 'string' && id.length <= 128 && /^[a-zA-Z0-9]+([._-][a-zA-Z0-9]+)*$/.test(id)
    );
  }
  function protectedId(id) {
    return PROTECTED.indexOf(id) !== -1 || id.indexOf('org.lumaglass.preview.') === 0;
  }
  function hash(bytes) {
    return crypto.createHash('sha256').update(bytes).digest('hex');
  }
  function safe(name, directory) {
    var stat = fs.lstatSync(name);
    if (
      stat.isSymbolicLink() ||
      stat.uid !== 0 ||
      stat.mode & 18 ||
      (directory ? !stat.isDirectory() : !stat.isFile())
    )
      throw Error('unsafe');
    return stat;
  }
  function parents(name) {
    var current = path.dirname(name);
    while (current !== '/') {
      safe(current, true);
      current = path.dirname(current);
    }
  }
  function read(name, limit) {
    parents(name);
    var stat = safe(name, false);
    if (stat.size > limit) throw Error('oversized');
    var fd = fs.openSync(name, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
    try {
      var opened = fs.fstatSync(fd);
      if (stat.ino !== opened.ino || stat.dev !== opened.dev) throw Error('changed');
      var bytes = Buffer.alloc(limit + 1);
      var offset = 0;
      var count;
      while (
        offset < bytes.length &&
        (count = fs.readSync(fd, bytes, offset, bytes.length - offset, null)) > 0
      )
        offset += count;
      if (offset > limit) throw Error('oversized');
      return bytes.slice(0, offset);
    } finally {
      fs.closeSync(fd);
    }
  }
  function json(name) {
    return JSON.parse(read(name, 65536).toString('utf8'));
  }
  function atomic(name, bytes) {
    var fd = fs.openSync(name + '.new', 'wx', 384);
    try {
      fs.writeFileSync(fd, bytes);
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
    fs.renameSync(name + '.new', name);
    fd = fs.openSync(path.dirname(name), 'r');
    try {
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
  }
  function luna(uri, payload) {
    var result = spawn(
      '/usr/bin/luna-send',
      ['-n', '1', '-w', '1200', uri, JSON.stringify(payload)],
      {
        encoding: 'utf8',
        timeout: 1500,
        maxBuffer: 2 * 1024 * 1024,
        stdio: ['ignore', 'pipe', 'ignore']
      }
    );
    if (result.error || result.status !== 0) throw Error('unavailable');
    var reply = JSON.parse(result.stdout || 'null');
    if (!reply || reply.returnValue !== true) throw Error('unavailable');
    return reply;
  }
  function mountRows(target) {
    // A bounded procfs read, with no regular-file size assumption for pseudo-files.
    var fd = fs.openSync('/proc/self/mountinfo', 'r');
    var bytes = Buffer.alloc(1024 * 1024 + 1);
    var offset = 0;
    try {
      var count;
      while (
        offset < bytes.length &&
        (count = fs.readSync(fd, bytes, offset, bytes.length - offset, null)) > 0
      )
        offset += count;
    } finally {
      fs.closeSync(fd);
    }
    if (offset > 1024 * 1024) throw Error('oversized');
    function unescape(value) {
      return value.replace(/\\([0-7]{3})/g, function (_, octal) {
        return String.fromCharCode(parseInt(octal, 8));
      });
    }
    return bytes
      .slice(0, offset)
      .toString('utf8')
      .trim()
      .split('\n')
      .filter(function (line) {
        var fields = line.split(' ');
        if (fields.length < 6) return false;
        var mount = unescape(fields[4]);
        // Ancestor app-directory mounts are conflicts too; never unmount them.
        return (
          mount === target ||
          mount === path.dirname(target) ||
          (mount !== '/' &&
            (target + '/').indexOf(mount + '/') === 0 &&
            mount.indexOf('/applications') !== -1)
        );
      });
  }
  function targetFor(id, folder) {
    if (
      folder !== '/usr/palm/applications/' + id &&
      folder !== '/media/system/apps/usr/palm/applications/' + id
    )
      throw Error('not-system');
    return folder + '/appinfo.json';
  }
  function command(binary, args) {
    var result = spawn(binary, args, { timeout: 2000, stdio: 'ignore' });
    if (result.error || result.status !== 0) throw Error('unconfirmed');
  }
  function fresh(request) {
    var now = Date.now();
    if (
      typeof request.expiresAt !== 'number' ||
      !isFinite(request.expiresAt) ||
      request.expiresAt < now ||
      request.expiresAt > now + 10000
    )
      throw Error('expired');
  }
  function createAdapter(profiles) {
    function entry(id) {
      return ROOT + '/' + id;
    }
    function record(id) {
      var directory = entry(id);
      safe(ROOT, true);
      safe(directory, true);
      var journal = json(directory + '/state.json');
      if (
        journal.schema !== 1 ||
        journal.owner !== OWNER ||
        journal.id !== id ||
        typeof journal.folder !== 'string' ||
        !/^[a-f0-9]{64}$/.test(journal.originalHash) ||
        !/^[a-f0-9]{64}$/.test(journal.overrideHash)
      )
        throw Error('conflict');
      var target = targetFor(id, journal.folder);
      if (
        hash(read(directory + '/override.json', 65536)) !== journal.overrideHash ||
        hash(read(directory + '/original.json', 65536)) !== journal.originalHash
      )
        throw Error('conflict');
      return { journal: journal, target: target, directory: directory };
    }
    function owned(id) {
      var current = record(id);
      var rows = mountRows(current.target);
      var source = safe(current.directory + '/override.json', false);
      var target = safe(current.target, false);
      if (
        rows.length !== 1 ||
        rows[0].split(' ')[4] !== current.target ||
        source.ino !== target.ino ||
        source.dev !== target.dev ||
        hash(read(current.target, 65536)) !== current.journal.overrideHash
      )
        throw Error('conflict');
      return current;
    }
    function inspect(id) {
      var report = {
        schema: 1,
        returnValue: true,
        id: id,
        status: 'unavailable',
        canRestore: false
      };
      if (!valid(id)) throw Error('invalid-id');
      if (protectedId(id)) {
        report.status = 'protected';
        return report;
      }
      if (typeof process.getuid !== 'function' || process.getuid() !== 0) {
        report.status = 'root-required';
        return report;
      }
      if (fs.existsSync(entry(id))) {
        try {
          var saved = record(id);
          if (mountRows(saved.target).length) {
            owned(id);
            report.status = 'owned';
            report.canRestore = true;
            return report;
          }
          if (saved.journal.status !== 'restored') {
            report.status = 'restored-pending';
            report.canRestore = hash(read(saved.target, 65536)) === saved.journal.originalHash;
            return report;
          }
        } catch (error) {
          reportFailure(error);
          report.status = 'conflict';
          return report;
        }
      }
      var info = luna('luna://com.webos.service.tv.systemproperty/getSystemInfo', {
        keys: ['modelName', 'firmwareVersion', 'sdkVersion']
      });
      ['modelName', 'firmwareVersion', 'sdkVersion'].forEach(function (key) {
        if (
          typeof info[key] !== 'string' ||
          info[key].length > 80 ||
          info[key].split('').some(function (letter) {
            return letter.charCodeAt(0) < 32 || letter.charCodeAt(0) === 127;
          })
        )
          throw Error('unavailable');
      });
      report.model = info.modelName;
      report.firmware = info.firmwareVersion;
      report.sdk = info.sdkVersion;
      var reply = luna('luna://com.webos.applicationManager/listApps', {});
      if (
        reply.ready === false ||
        !Array.isArray(reply.apps) ||
        reply.apps.length > 1000 ||
        !reply.apps.some(function (app) {
          return app.id === 'com.webos.app.home';
        })
      )
        throw Error('unavailable');
      var matches = reply.apps.filter(function (app) {
        return app.id === id;
      });
      if (matches.length !== 1 || matches[0].systemApp !== true) {
        report.status = 'not-system';
        return report;
      }
      var app = matches[0];
      var target = targetFor(id, app.folderPath);
      if (mountRows(target).length) {
        report.status = 'conflict';
        return report;
      }
      // Preserve both active overlays and queued Glasshouse boot integrations.
      var foreignList = '/var/lib/tvweb/hidden_apps';
      var foreignOverride = '/var/lib/tvweb/appinfo-overrides/' + id + '.json';
      if (
        fs.existsSync(foreignOverride) ||
        (fs.existsSync(foreignList) &&
          read(foreignList, 65536)
            .toString('utf8')
            .split('\n')
            .some(function (line) {
              return line.trim() === id;
            }))
      ) {
        report.status = 'conflict';
        return report;
      }
      var bytes = read(target, 65536);
      var manifest = JSON.parse(bytes.toString('utf8'));
      if (
        !manifest ||
        Array.isArray(manifest) ||
        manifest.id !== id ||
        (manifest.visible !== undefined && manifest.visible !== true) ||
        app.visible !== true
      ) {
        report.status = 'conflict';
        return report;
      }
      report.folder = app.folderPath;
      report.manifestHash = hash(bytes);
      report.plan = hash(
        JSON.stringify([
          id,
          report.folder,
          report.manifestHash,
          report.model,
          report.firmware,
          report.sdk,
          app.version
        ])
      );
      var profile = profiles.filter(function (item) {
        return (
          item.model === report.model &&
          item.firmware === report.firmware &&
          item.sdk === report.sdk &&
          Array.isArray(item.apps) &&
          item.apps.indexOf(id) !== -1
        );
      });
      if (profile.length === 1) {
        var schema = json('/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema');
        if (!schema.properties || !schema.properties.id) throw Error('unavailable');
      }
      report.status = profile.length === 1 ? 'ready' : 'unverified';
      return report;
    }
    function refresh(id, expected) {
      var payload = { id: id };
      var schema = json('/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema');
      if (schema.properties && schema.properties.forceBlockSystemApp)
        payload.forceBlockSystemApp = false;
      var current = record(id);
      current.journal.blockIntent = true;
      atomic(current.directory + '/state.json', JSON.stringify(current.journal));
      var blocked = false;
      var released = false;
      try {
        luna('luna://com.webos.applicationManager/blockSystemApp', payload);
        blocked = true;
      } catch (error) {
        reportFailure(error);
        /* A timeout can still mean the block took effect. */
      } finally {
        for (var tries = 0; tries < 2 && !released; tries++) {
          try {
            luna('luna://com.webos.applicationManager/releaseBlockSystemApp', { id: id });
            released = true;
          } catch (error) {
            reportFailure(error);
            /* Preserve the intent for manual recovery, never restart SAM. */
          }
        }
      }
      if (released) {
        current.journal.blockIntent = false;
        atomic(current.directory + '/state.json', JSON.stringify(current.journal));
      }
      if (!blocked || !released) throw Error('unconfirmed');
      var info = luna('luna://com.webos.applicationManager/getAppInfo', { id: id }).appInfo;
      if (!info || info.id !== id || info.visible !== expected) throw Error('unconfirmed');
    }
    function lock() {
      if (!fs.existsSync(ROOT)) {
        parents(ROOT);
        fs.mkdirSync(ROOT, 448);
      }
      safe(ROOT, true);
      if (fs.existsSync(ROOT + '/lock')) {
        safe(ROOT + '/lock', true);
        var previous = json(ROOT + '/lock/owner.json');
        if (previous.owner !== OWNER || !Number.isInteger(previous.pid) || previous.pid <= 1)
          throw Error('conflict');
        try {
          process.kill(previous.pid, 0);
          throw Error('busy');
        } catch (error) {
          if (error.code !== 'ESRCH') throw Error('conflict');
        }
        // A dead owner can be recovered; live/reused PIDs and unknown locks stay untouched.
        fs.unlinkSync(ROOT + '/lock/owner.json');
        fs.rmdirSync(ROOT + '/lock');
      }
      fs.mkdirSync(ROOT + '/lock', 448);
      atomic(ROOT + '/lock/owner.json', JSON.stringify({ owner: OWNER, pid: process.pid }));
    }
    function unlock() {
      var owner = json(ROOT + '/lock/owner.json');
      if (owner.owner !== OWNER || owner.pid !== process.pid) throw Error('conflict');
      fs.unlinkSync(ROOT + '/lock/owner.json');
      fs.rmdirSync(ROOT + '/lock');
    }
    function restore(id) {
      var current = record(id);
      var rows = mountRows(current.target);
      if (rows.length) {
        current = owned(id);
        current.journal.status = 'restoring';
        atomic(current.directory + '/state.json', JSON.stringify(current.journal));
        owned(id);
        try {
          command('/bin/umount', [current.target]);
        } catch (error) {
          reportFailure(error);
          if (mountRows(current.target).length) throw Error('unconfirmed');
        }
        if (mountRows(current.target).length) throw Error('unconfirmed');
      } else if (hash(read(current.target, 65536)) !== current.journal.originalHash)
        throw Error('conflict');
      if (
        !rows.length &&
        current.journal.blockIntent !== true &&
        ['prepared', 'restored'].indexOf(current.journal.status) !== -1
      ) {
        current.journal.status = 'restored';
        atomic(current.directory + '/state.json', JSON.stringify(current.journal));
        return { schema: 1, returnValue: true, id: id, status: 'restored', canRestore: false };
      }
      var visible = false;
      try {
        var info = luna('luna://com.webos.service.tv.systemproperty/getSystemInfo', {
          keys: ['modelName', 'firmwareVersion', 'sdkVersion']
        });
        var qualified = profiles.some(function (profile) {
          return (
            profile.model === info.modelName &&
            profile.firmware === info.firmwareVersion &&
            profile.sdk === info.sdkVersion &&
            Array.isArray(profile.apps) &&
            profile.apps.indexOf(id) !== -1
          );
        });
        // Release only an intent journalled by this adapter before its own request.
        if (current.journal.blockIntent === true) {
          luna('luna://com.webos.applicationManager/releaseBlockSystemApp', { id: id });
          current.journal.blockIntent = false;
          atomic(current.directory + '/state.json', JSON.stringify(current.journal));
        }
        // Firmware changes never authorize a new block/refresh sequence.
        if (qualified) {
          refresh(id, true);
          visible = true;
        }
      } catch (error) {
        reportFailure(error);
        /* retain a pending result; never restart SAM */
      }
      var journal = record(id).journal;
      journal.status = visible ? 'restored' : 'restored-pending';
      atomic(current.directory + '/state.json', JSON.stringify(journal));
      return { schema: 1, returnValue: true, id: id, status: journal.status, canRestore: !visible };
    }
    function hide(request) {
      fresh(request);
      var report = inspect(request.id);
      if (report.status !== 'ready' || request.plan !== report.plan) throw Error('unverified');
      var target = targetFor(request.id, report.folder);
      var directory = entry(request.id);
      // Revalidate immediately before staging; no automatic retries after uncertainty.
      fresh(request);
      if (mountRows(target).length) throw Error('conflict');
      if (fs.existsSync(directory) && record(request.id).journal.status !== 'restored')
        throw Error('conflict');
      var original = read(target, 65536);
      if (hash(original) !== report.manifestHash) throw Error('changed');
      var manifest = JSON.parse(original.toString('utf8'));
      manifest.visible = false;
      var replacement = JSON.stringify(manifest);
      if (Buffer.byteLength(replacement) > 65536) throw Error('oversized');
      if (!fs.existsSync(directory)) fs.mkdirSync(directory, 448);
      atomic(directory + '/original.json', original);
      atomic(directory + '/override.json', replacement);
      var journal = {
        schema: 1,
        owner: OWNER,
        id: request.id,
        folder: report.folder,
        originalHash: report.manifestHash,
        overrideHash: hash(replacement),
        status: 'prepared'
      };
      atomic(directory + '/state.json', JSON.stringify(journal));
      atomic(
        ROOT + '/system-apps.cjs',
        '// Original Lumaglass adapter (MIT).\n(' + lumaglassSystemApps.toString() + ')();\n'
      );
      try {
        fresh(request);
        command('/bin/mount', ['--bind', directory + '/override.json', target]);
        owned(request.id);
        refresh(request.id, false);
        journal.blockIntent = false;
        journal.status = 'hidden';
        atomic(directory + '/state.json', JSON.stringify(journal));
        return inspect(request.id);
      } catch (error) {
        reportFailure(error);
        try {
          return restore(request.id);
        } catch (error) {
          reportFailure(error);
          throw Error('unconfirmed');
        }
      }
    }
    function dispatch(request) {
      if (!request || !valid(request.id) || protectedId(request.id)) throw Error('protected');
      if (request.action === 'inspect') return inspect(request.id);
      if (typeof process.getuid !== 'function' || process.getuid() !== 0)
        throw Error('root-required');
      // An unqualified hide is refused before even creating a lock or state directory.
      if (request.action === 'hide') {
        fresh(request);
        var checked = inspect(request.id);
        if (checked.status !== 'ready' || checked.plan !== request.plan) throw Error('unverified');
      }
      if (request.action !== 'hide' && request.action !== 'restore') throw Error('invalid-action');
      if (request.action === 'restore') {
        var current = record(request.id);
        if (mountRows(current.target).length) owned(request.id);
        else if (hash(read(current.target, 65536)) !== current.journal.originalHash)
          throw Error('conflict');
      }
      lock();
      try {
        return request.action === 'hide' ? hide(request) : restore(request.id);
      } finally {
        unlock();
      }
    }
    return { inspect: inspect, dispatch: dispatch };
  }
  if (typeof module === 'object') module.exports = { createAdapter: createAdapter };
  var request;
  var result;
  try {
    var argument = typeof __filename === 'string' && process.argv[1] === __filename ? 2 : 1;
    request =
      process.argv[argument] === 'restore'
        ? { action: 'restore', id: process.argv[argument + 1] }
        : JSON.parse(process.argv[argument] || 'null');
    result = createAdapter(VERIFIED).dispatch(request);
  } catch (error) {
    var allowed = [
      'protected',
      'root-required',
      'not-system',
      'conflict',
      'unverified',
      'expired',
      'changed',
      'unconfirmed'
    ];
    result = {
      schema: 1,
      returnValue: true,
      id: request && valid(request.id) ? request.id : '',
      status: allowed.indexOf(error.message) === -1 ? 'unavailable' : error.message,
      canRestore: false
    };
  }
  console.log(JSON.stringify(result));
})();
