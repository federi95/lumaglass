// Original Lumaglass system-saver adapter (MIT). Node 8. No boot hook or panel-policy changes.
(function lumaglassScreensaver() {
  var fs = require('fs');
  var path = require('path');
  var crypto = require('crypto');
  var spawn = require('child_process').spawnSync;
  var ROOT = '/var/lib/lumaglass-screensaver';
  var OWNER = 'org.lumaglass.app';
  var ID = 'com.webos.app.screensaver';
  var TARGET = '/usr/palm/applications/' + ID;
  // Empty until native lifecycle, wake, rollback and panel policy are verified on exact hardware.
  var VERIFIED = [];
  var INSTALLS = [
    '/media/cryptofs/apps/usr/palm/applications/org.lumaglass.app',
    '/media/developer/apps/usr/palm/applications/org.lumaglass.app',
    '/usr/palm/applications/org.lumaglass.app'
  ];
  var CACHES = [
    '/media/internal/.org.lumaglass.app/preferences.wallpapers',
    '/media/developer/org.lumaglass.app/preferences.wallpapers',
    '/media/developer/temp/org.lumaglass.app/preferences.wallpapers'
  ];
  function reportFailure() {
    console.error('System screensaver: operation unconfirmed');
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
      var bytes = Buffer.alloc(Math.min(limit + 1, stat.size + 1));
      var offset = 0;
      var count;
      while (
        offset < bytes.length &&
        (count = fs.readSync(fd, bytes, offset, bytes.length - offset, null)) > 0
      )
        offset += count;
      if (offset > limit) throw Error('oversized');
      if (offset !== stat.size || fs.fstatSync(fd).size !== stat.size) throw Error('changed');
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
          mount.indexOf(target + '/') === 0 ||
          (mount !== '/' &&
            (target + '/').indexOf(mount + '/') === 0 &&
            mount.indexOf('/applications') !== -1)
        );
      });
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

  function configSnapshot(request) {
    var c = request.config;
    if (
      !c ||
      c.schema !== 1 ||
      typeof c.photos !== 'boolean' ||
      typeof c.hour_24 !== 'boolean' ||
      ['en', 'de', 'zh', 'es', 'pt', 'it', 'ru', 'pl', 'fr'].indexOf(c.language) === -1 ||
      !Array.isArray(c.images) ||
      c.images.length > 12 ||
      c.images.some(function (id, i) {
        return (
          typeof id !== 'string' ||
          !/^personal-[a-f0-9]{16}$/.test(id) ||
          c.images.indexOf(id) !== i
        );
      }) ||
      (c.photos ? !c.images.length : c.images.length !== 0)
    )
      throw Error('conflict');
    var snapshot = {
      schema: 1,
      photos: c.photos,
      language: c.language,
      hour_24: c.hour_24,
      images: c.images.slice()
    };
    var photos = [];
    var size = 0;
    if (c.photos) {
      if (CACHES.indexOf(request.photo_root) === -1) throw Error('conflict');
      parents(request.photo_root);
      safe(request.photo_root, true);
      c.images.forEach(function (id) {
        var bytes = read(request.photo_root + '/' + id + '.jpg', 8 * 1024 * 1024);
        size += bytes.length;
        if (size > 32 * 1024 * 1024) throw Error('oversized');
        photos.push([id, hash(bytes)]);
      });
    }
    return { config: snapshot, hashes: photos };
  }
  function installation() {
    var found = INSTALLS.filter(function (dir) {
      return fs.existsSync(dir + '/appinfo.json');
    });
    if (found.length !== 1) throw Error('conflict');
    var dir = found[0];
    parents(dir);
    safe(dir, true);
    var app = json(dir + '/appinfo.json');
    if (app.id !== OWNER || app.type !== 'native' || app.main !== 'lumaglass')
      throw Error('conflict');
    if (!(safe(dir + '/lumaglass', false).mode & 73)) throw Error('conflict');
    var bytes = read(dir + '/lumaglass', 64 * 1024 * 1024);
    if (
      bytes.length < 52 ||
      bytes.slice(0, 4).toString('hex') !== '7f454c46' ||
      bytes[4] !== 1 ||
      bytes[5] !== 1 ||
      bytes.readUInt16LE(18) !== 40 ||
      bytes.readUInt32LE(36) & 1024
    )
      throw Error('conflict');
    safe(dir + '/assets', true);
    safe(dir + '/lib', true);
    return { directory: dir, binaryHash: hash(bytes) };
  }
  function context() {
    var info = luna('luna://com.webos.service.tv.systemproperty/getSystemInfo', {
      keys: ['modelName', 'firmwareVersion', 'sdkVersion']
    });
    ['modelName', 'firmwareVersion', 'sdkVersion'].forEach(function (k) {
      if (
        typeof info[k] !== 'string' ||
        !info[k].length ||
        info[k].length > 80 ||
        info[k].split('').some(function (c) {
          return c.charCodeAt(0) < 32 || c.charCodeAt(0) === 127;
        })
      )
        throw Error('unavailable');
    });
    return { model: info.modelName, firmware: info.firmwareVersion, sdk: info.sdkVersion };
  }
  function reply(status) {
    return { schema: 1, returnValue: true, id: ID, status: status, canRestore: false };
  }
  function createAdapter(profiles) {
    function qualified(info, stock) {
      return (
        profiles.filter(function (p) {
          return (
            p.model === info.model &&
            p.firmware === info.firmware &&
            p.sdk === info.sdk &&
            p.stockType === stock.type &&
            p.stockMain === stock.main &&
            p.nativeLifecycle === 1
          );
        }).length === 1
      );
    }
    function record() {
      safe(ROOT, true);
      safe(ROOT + '/app', true);
      var j = json(ROOT + '/state.json');
      if (
        j.schema !== 1 ||
        j.owner !== OWNER ||
        j.id !== ID ||
        INSTALLS.indexOf(j.install) === -1 ||
        !Array.isArray(j.photoHashes)
      )
        throw Error('conflict');
      [
        ['original.json', 'originalHash'],
        ['app/appinfo.json', 'overrideHash'],
        ['app/settings.json', 'configHash']
      ].forEach(function (pair) {
        if (
          !/^[a-f0-9]{64}$/.test(j[pair[1]]) ||
          hash(read(ROOT + '/' + pair[0], 65536)) !== j[pair[1]]
        )
          throw Error('conflict');
      });
      if (
        ['model', 'firmware', 'sdk'].some(function (k) {
          return typeof j[k] !== 'string' || !j[k].length || j[k].length > 80;
        })
      )
        throw Error('conflict');
      var stock = json(ROOT + '/original.json');
      if (stock.id !== ID) throw Error('conflict');
      if (j.photoHashes.length > 12) throw Error('conflict');
      j.photoHashes.forEach(function (pair) {
        if (
          !Array.isArray(pair) ||
          !/^personal-[a-f0-9]{16}$/.test(pair[0]) ||
          hash(read(ROOT + '/app/photos/' + pair[0] + '.jpg', 8 * 1024 * 1024)) !== pair[1]
        )
          throw Error('conflict');
      });
      [
        ['lumaglass-screensaver', 'lumaglass'],
        ['assets', 'assets'],
        ['lib', 'lib']
      ].forEach(function (pair) {
        var name = ROOT + '/app/' + pair[0];
        if (
          !fs.lstatSync(name).isSymbolicLink() ||
          fs.readlinkSync(name) !== j.install + '/' + pair[1]
        )
          throw Error('conflict');
      });
      return j;
    }
    function own() {
      var j = record();
      var rows = mountRows(TARGET);
      var source = safe(ROOT + '/app', true);
      var dest = safe(TARGET, true);
      if (
        rows.length !== 1 ||
        rows[0].split(' ')[4] !== TARGET ||
        source.ino !== dest.ino ||
        source.dev !== dest.dev ||
        hash(read(TARGET + '/appinfo.json', 65536)) !== j.overrideHash
      )
        throw Error('conflict');
      return j;
    }
    function stockNow() {
      var stock = json(TARGET + '/appinfo.json');
      if (stock.id !== ID || typeof stock.type !== 'string' || typeof stock.main !== 'string')
        throw Error('conflict');
      return stock;
    }
    function save(j) {
      atomic(ROOT + '/state.json', JSON.stringify(j));
    }
    function inspect(request) {
      var report = reply('unavailable');
      if (typeof process.getuid !== 'function' || process.getuid() !== 0) {
        report.status = 'root-required';
        return report;
      }
      if (fs.existsSync(ROOT + '/state.json')) {
        var j = record();
        if (mountRows(TARGET).length) {
          own();
          report.status = 'owned';
          report.canRestore = true;
          return report;
        }
        if (j.status !== 'restored') {
          report.status = 'restored-pending';
          report.canRestore = stockNow().id === ID;
          return report;
        }
      }
      parents(TARGET);
      safe(TARGET, true);
      if (
        mountRows(TARGET).length ||
        fs.existsSync(TARGET + '/.tvweb-screensaver') ||
        fs.existsSync('/var/lib/tvweb/screensaver') ||
        fs.existsSync('/var/lib/tvweb/screensaver-stock-type') ||
        fs.existsSync('/var/run/tvweb-screensaver-launched')
      ) {
        report.status = 'conflict';
        return report;
      }
      var info = context();
      var bytes = read(TARGET + '/appinfo.json', 65536);
      var stock = JSON.parse(bytes.toString('utf8'));
      if (stock.id !== ID || typeof stock.type !== 'string' || typeof stock.main !== 'string')
        throw Error('conflict');
      var observed = luna('luna://com.webos.applicationManager/getAppInfo', { id: ID }).appInfo;
      if (
        !observed ||
        observed.id !== ID ||
        observed.type !== stock.type ||
        observed.main !== stock.main
      )
        throw Error('conflict');
      var installed = installation();
      var snapshot = configSnapshot(request);
      report.model = info.model;
      report.firmware = info.firmware;
      report.sdk = info.sdk;
      report.manifestHash = hash(bytes);
      report.snapshotHash = hash(JSON.stringify(snapshot));
      report.install = installed.directory;
      report.binaryHash = installed.binaryHash;
      report.plan = hash(JSON.stringify([info, report.manifestHash, installed, snapshot]));
      report.status = qualified(info, stock) ? 'ready' : 'unverified';
      if (report.status === 'ready') {
        var schema = json('/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema');
        if (!schema.properties || !schema.properties.id) throw Error('unavailable');
      }
      return report;
    }
    function matches(expected) {
      var app = luna('luna://com.webos.applicationManager/getAppInfo', { id: ID }).appInfo;
      return !!app && app.id === ID && app.type === expected.type && app.main === expected.main;
    }
    function refresh(expected) {
      var payload = { id: ID };
      var schema = json('/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema');
      if (schema.properties && schema.properties.forceBlockSystemApp)
        payload.forceBlockSystemApp = false;
      var j = record();
      j.blockIntent = true;
      save(j);
      var blocked = false;
      var released = false;
      try {
        luna('luna://com.webos.applicationManager/blockSystemApp', payload);
        blocked = true;
      } catch (error) {
        reportFailure(error);
      } finally {
        for (var i = 0; i < 2 && !released; i++) {
          try {
            luna('luna://com.webos.applicationManager/releaseBlockSystemApp', { id: ID });
            released = true;
          } catch (error) {
            reportFailure(error);
          }
        }
      }
      if (released) {
        j.blockIntent = false;
        save(j);
      }
      if (!blocked || !released || !matches(expected)) throw Error('unconfirmed');
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

    function restore() {
      var j = record();
      var mounted = mountRows(TARGET).length > 0;
      var wasPrepared = j.status === 'prepared';
      if (mounted) {
        own();
        j.status = 'restoring';
        save(j);
        own();
        try {
          command('/bin/umount', [TARGET]);
        } catch (error) {
          reportFailure(error);
          if (mountRows(TARGET).length) throw Error('unconfirmed');
        }
      }
      if (mountRows(TARGET).length) throw Error('conflict');
      var confirmed = false;
      var stock = stockNow();
      j.stockChanged = hash(read(TARGET + '/appinfo.json', 65536)) !== j.originalHash;
      save(j);
      try {
        if (j.blockIntent === true) {
          luna('luna://com.webos.applicationManager/releaseBlockSystemApp', { id: ID });
          j.blockIntent = false;
          save(j);
        }
        var observed = context();
        if (
          observed.model === j.model &&
          observed.firmware === j.firmware &&
          observed.sdk === j.sdk &&
          qualified(observed, stock)
        ) {
          refresh(stock);
          confirmed = true;
        } else confirmed = matches(stock);
      } catch (error) {
        reportFailure(error);
      }
      // Before any mount/refresh, SAM has never seen the candidate.
      if (!mounted && wasPrepared && j.blockIntent !== true) confirmed = true;
      j = record();
      j.status = confirmed ? 'restored' : 'restored-pending';
      save(j);
      var report = reply(j.status);
      report.canRestore = !confirmed;
      return report;
    }
    function stageLink(name, target) {
      try {
        var stat = fs.lstatSync(name);
        if (!stat.isSymbolicLink() || fs.readlinkSync(name) !== target) throw Error('conflict');
        return;
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      fs.symlinkSync(target, name);
    }
    function apply(request) {
      fresh(request);
      var report = inspect(request);
      if (report.status !== 'ready' || report.plan !== request.plan) throw Error('unverified');
      var original = read(TARGET + '/appinfo.json', 65536);
      if (hash(original) !== report.manifestHash) throw Error('changed');
      if (fs.existsSync(ROOT + '/state.json') && record().status !== 'restored')
        throw Error('conflict');
      var snapshot = configSnapshot(request);
      if (hash(JSON.stringify(snapshot)) !== report.snapshotHash) throw Error('changed');
      var manifest = JSON.parse(original.toString('utf8'));
      manifest.id = ID;
      manifest.type = 'native';
      manifest.main = 'lumaglass-screensaver';
      manifest.nativeLifeCycleInterfaceVersion = 1;
      manifest.visible = false;
      manifest.noSplashOnLaunch = true;
      manifest.handlesRelaunch = false;
      delete manifest.appCategory;
      if (!fs.existsSync(ROOT + '/app')) fs.mkdirSync(ROOT + '/app', 493);
      safe(ROOT + '/app', true);
      if (!fs.existsSync(ROOT + '/app/photos')) fs.mkdirSync(ROOT + '/app/photos', 493);
      safe(ROOT + '/app/photos', true);
      stageLink(ROOT + '/app/lumaglass-screensaver', report.install + '/lumaglass');
      stageLink(ROOT + '/app/assets', report.install + '/assets');
      stageLink(ROOT + '/app/lib', report.install + '/lib');
      snapshot.hashes.forEach(function (pair) {
        var bytes = read(request.photo_root + '/' + pair[0] + '.jpg', 8 * 1024 * 1024);
        if (hash(bytes) !== pair[1]) throw Error('changed');
        atomic(ROOT + '/app/photos/' + pair[0] + '.jpg', bytes);
        fs.chmodSync(ROOT + '/app/photos/' + pair[0] + '.jpg', 420);
      });
      var replacement = JSON.stringify(manifest);
      var config = JSON.stringify(snapshot.config);
      atomic(ROOT + '/original.json', original);
      atomic(ROOT + '/app/appinfo.json', replacement);
      atomic(ROOT + '/app/settings.json', config);
      fs.chmodSync(ROOT + '/app/appinfo.json', 420);
      fs.chmodSync(ROOT + '/app/settings.json', 420);
      var j = {
        schema: 1,
        owner: OWNER,
        id: ID,
        install: report.install,
        model: report.model,
        firmware: report.firmware,
        sdk: report.sdk,
        originalHash: hash(original),
        overrideHash: hash(replacement),
        configHash: hash(config),
        photoHashes: snapshot.hashes,
        status: 'prepared'
      };
      save(j);
      atomic(
        ROOT + '/screensaver.cjs',
        '// Original Lumaglass adapter (MIT).\n(' + lumaglassScreensaver.toString() + ')();\n'
      );
      try {
        fresh(request);
        var currentContext = context();
        if (
          currentContext.model !== report.model ||
          currentContext.firmware !== report.firmware ||
          currentContext.sdk !== report.sdk
        )
          throw Error('changed');
        if (
          mountRows(TARGET).length ||
          hash(read(TARGET + '/appinfo.json', 65536)) !== j.originalHash ||
          hash(read(report.install + '/lumaglass', 64 * 1024 * 1024)) !== report.binaryHash
        )
          throw Error('changed');
        if (luna('luna://com.webos.service.tvpower/power/getPowerState', {}).state !== 'Active')
          throw Error('unconfirmed');
        fresh(request);
        command('/bin/mount', ['--bind', ROOT + '/app', TARGET]);
        own();
        refresh(manifest);
        j = record();
        j.status = 'active';
        save(j);
        return inspect(request);
      } catch (error) {
        reportFailure(error);
        try {
          return restore();
        } catch (error) {
          reportFailure(error);
          throw Error('unconfirmed');
        }
      }
    }
    function dispatch(request) {
      if (!request || request.id !== ID) throw Error('conflict');
      if (request.action === 'inspect') return inspect(request);
      if (typeof process.getuid !== 'function' || process.getuid() !== 0)
        throw Error('root-required');
      if (request.action === 'request') {
        fresh(request);
        var before = luna('luna://com.webos.service.tvpower/power/getPowerState', {}).state;
        if (before !== 'Active' && before !== 'Screen Saver') throw Error('unconfirmed');
        fresh(request);
        if (before === 'Active')
          luna('luna://com.webos.service.tvpower/power/turnOnScreenSaver', {});
        return reply(
          luna('luna://com.webos.service.tvpower/power/getPowerState', {}).state === 'Screen Saver'
            ? 'requested'
            : 'unconfirmed'
        );
      }
      if (request.action !== 'apply' && request.action !== 'restore') throw Error('conflict');
      if (request.action === 'apply') {
        fresh(request);
        var report = inspect(request);
        if (report.status !== 'ready' || report.plan !== request.plan) throw Error('unverified');
      } else {
        record();
        if (mountRows(TARGET).length) own();
        else stockNow();
      }
      lock();
      try {
        return request.action === 'apply' ? apply(request) : restore();
      } finally {
        unlock();
      }
    }
    return { inspect: inspect, dispatch: dispatch };
  }
  if (typeof module === 'object') module.exports = { createAdapter: createAdapter };
  var result;
  try {
    var argument = typeof __filename === 'string' && process.argv[1] === __filename ? 2 : 1;
    var request =
      process.argv[argument] === 'restore'
        ? { action: 'restore', id: ID }
        : JSON.parse(process.argv[argument] || 'null');
    result = createAdapter(VERIFIED).dispatch(request);
  } catch (error) {
    var statuses = ['root-required', 'conflict', 'unverified', 'expired', 'changed', 'unconfirmed'];
    result = reply(statuses.indexOf(error.message) !== -1 ? error.message : 'unavailable');
  }
  console.log(JSON.stringify(result));
})();
