// Derived from Luma (MIT), with Glasshouse Home takeover as an upstream reference.
// Copyright notices and licenses: LICENSE, NOTICE.md, assets/licenses/glasshouse.txt.
// Root-only TV integration. Keep ES5 syntax for the TV's bundled Node runtime.
// The native executable remains in its installed package so its LS2 role,
// libraries, assets and preference identity continue to use that installation.
var fs = require('fs');
var path = require('path');
var execFileSync = require('child_process').execFileSync;
var spawnSync = require('child_process').spawnSync;
var homeMountsReader = require('./home-mounts.cjs');
var APP = 'org.lumaglass.app';
var HOME = 'com.webos.app.home';
var STATE = '/var/lib/lumaglass-home';
var STAGE = STATE + '/app';
var TARGET = '/usr/palm/applications/' + HOME;
var HOOK = '/var/lib/webosbrew/init.d/64-lumaglass-home';
var CONFIG = STATE + '/state.json';
// webOS 5/6 (method "executable"): SAM cannot reread a mounted manifest there, but LG Home is a
// native lifecycle-v2 program whose LS2 role is bound to this executable path. Mounting Lumaglass's
// executable over it keeps LG's manifest and role; Lumaglass then registers with SAM itself.
var EXE = '/usr/bin/' + HOME;
// Tells a normal tile launch to open the Home instance. The tile's jail sees /usr/bin without
// this mount but shares /var/run, which is volatile like the mount itself.
var MARKER = '/var/run/lumaglass-home';

function read(file) {
  return fs.readFileSync(file, 'utf8');
}

function json(file) {
  return JSON.parse(read(file));
}

function exists(file) {
  try {
    fs.lstatSync(file);
    return true;
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}

function writeJson(file, value) {
  fs.writeFileSync(file + '.new', JSON.stringify(value, null, 2) + '\n', { mode: 420 });
  fs.renameSync(file + '.new', file);
}

function run(file, args) {
  return String(execFileSync(file, args, { timeout: 8000, encoding: 'utf8' }));
}

function pause() {
  run('/bin/sleep', ['1']);
}

function excerpt(output) {
  return (
    String(output || '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 480) || '<empty>'
  );
}

function luna(method, payload) {
  // -n prints one JSON reply to stdout. -t is timing mode and logs the
  // payload to stderr instead, leaving timing summaries on stdout.
  var result = spawnSync(
    '/usr/bin/luna-send',
    [
      '-n',
      '1',
      '-w',
      '4000',
      'luna://com.webos.applicationManager/' + method,
      JSON.stringify(payload)
    ],
    {
      encoding: 'utf8',
      timeout: 8000,
      maxBuffer: 2 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'pipe']
    }
  );
  var stdout = String(result.stdout || '').trim();
  var diagnostics = '; stdout: ' + excerpt(stdout) + '; stderr: ' + excerpt(result.stderr);
  if (result.error || result.status !== 0) {
    throw new Error(
      'Luna ' +
        method +
        ' failed (' +
        (result.error
          ? result.error.message
          : 'exit ' + result.status + ', signal ' + result.signal) +
        ')' +
        diagnostics
    );
  }
  if (!stdout) throw new Error('No Luna JSON reply from ' + method + diagnostics);
  var reply;
  try {
    reply = JSON.parse(stdout);
  } catch (error) {
    throw new Error('Invalid Luna JSON reply from ' + method + ': ' + error.message + diagnostics);
  }
  if (
    !reply ||
    typeof reply !== 'object' ||
    Array.isArray(reply) ||
    typeof reply.returnValue !== 'boolean'
  ) {
    throw new Error('Invalid Luna response from ' + method + diagnostics);
  }
  return reply;
}

function accepted(reply, action) {
  if (!reply || reply.returnValue !== true) {
    throw new Error(action + ': ' + JSON.stringify(reply));
  }
  return reply;
}

function mkdir(directory) {
  if (!fs.existsSync(directory)) fs.mkdirSync(directory, 493);
}

function mounted() {
  if (!fs.existsSync(STAGE)) return false;
  var a = fs.statSync(STAGE);
  var b = fs.statSync(TARGET);
  return a.dev === b.dev && a.ino === b.ino;
}

function foreignMount() {
  return homeMounts().conflict;
}

function homeMounts() {
  return homeMountsReader.snapshot(fs);
}

function bootIntegrations() {
  return homeMountsReader.boot(fs);
}

function assertNoForeignBoot() {
  var report = bootIntegrations();
  if (report.foreign)
    throw new Error(
      'Another Home replacement is configured at startup (Luma or Glasshouse). Disable it with its own restore command first'
    );
  if (report.uncertain)
    throw new Error(
      'Other Home startup state could not be read; resolve it before enabling Lumaglass'
    );
}

function assertHomeOwnership(method) {
  var report = homeMounts();
  if (report.conflict)
    throw new Error(
      'Another integration or stacked mounts are present over LG Home; restore their owner first'
    );
  if (
    (method === 'executable' && report.manifest.length) ||
    (method === 'manifest' && report.executable.length)
  )
    throw new Error(
      'A different Lumaglass Home method is active; restore it before switching methods'
    );
  return report;
}

// The mount source of an active bind mount over LG's Home executable, or null.
function exeMount() {
  var report = assertHomeOwnership();
  return report.executable.length ? report.executable[0].source : null;
}

// Lumaglass's executable, also after a package update replaced the mounted file.
function exeOwned(root) {
  return homeMountsReader.executableOwned(root);
}

function exeMounted(appDir) {
  try {
    var a = fs.statSync(appDir + '/lumaglass');
    var b = fs.statSync(EXE);
    return a.dev === b.dev && a.ino === b.ino;
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}

function unmountExe() {
  try {
    run('/bin/umount', [EXE]);
  } catch (error) {
    // The running Home executes the mounted file.
    console.error('Lumaglass Home: ' + error.message + ', detaching the mount');
    run('/bin/umount', ['-l', EXE]);
  }
  if (exeMount()) throw new Error('The Lumaglass Home mount is still active');
}

function homeMethod(release, stock) {
  if (/release\s+(10|11)\./.test(release)) return 'manifest';
  if (
    /release\s+[56]\./.test(release) &&
    stock &&
    stock.type === 'native' &&
    stock.main === 'file://' + EXE &&
    stock.nativeLifeCycleInterfaceVersion === 2
  )
    return 'executable';
  throw new Error('This Home integration currently targets webOS 5/6 and 25/26 (10.x/11.x)');
}

function releaseHome() {
  var result;
  for (var i = 0; i < 3; i++) {
    try {
      result = luna('releaseBlockSystemApp', { id: HOME });
      if (result.returnValue === true || /not.*blocked/i.test(result.errorText || '')) return;
    } catch (error) {
      result = { errorText: error.message };
    }
    pause();
  }
  throw new Error('Could not release Home: ' + JSON.stringify(result));
}

function reread(expected) {
  var payload = { id: HOME };
  var schema = '/etc/palm/schemas/sam/applicationManager.blockSystemApp.schema';
  if (fs.existsSync(schema) && read(schema).indexOf('"forceBlockSystemApp"') !== -1) {
    payload.forceBlockSystemApp = false;
  }
  // Always release, even if the block request times out after taking effect.
  try {
    var reply = luna('blockSystemApp', payload);
    if (!/already\s*blocked/i.test(reply.errorText || '')) {
      accepted(reply, 'Block Home for manifest refresh');
    }
  } finally {
    releaseHome();
  }
  for (var i = 0; i < 15; i++) {
    var info = accepted(luna('getAppInfo', { id: HOME }), 'Read Home manifest').appInfo;
    if (
      info &&
      info.type === expected.type &&
      (typeof expected.main !== 'string' ||
        path.basename(info.main || '') === path.basename(expected.main)) &&
      (expected.main !== 'lumaglass' ||
        (Array.isArray(info.appCategory) && info.appCategory.indexOf('home') !== -1))
    )
      return;
    pause();
  }
  throw new Error('SAM did not read the expected Home manifest');
}

function foreground() {
  return accepted(luna('getForegroundAppInfo', {}), 'Read foreground app').appId;
}

function normalRunning() {
  return (accepted(luna('running', {}), 'Read running apps').running || []).some(function (entry) {
    return entry.id === APP;
  });
}

function closeNormal() {
  if (!normalRunning()) return;
  try {
    // On TV, close's id identifies a process. Use the app-ID endpoint for APP.
    accepted(luna('closeByAppId', { id: APP }), 'Close the normal Lumaglass instance');
  } catch (error) {
    // The app may have exited between the running query and the close request.
    if (normalRunning()) throw error;
    return;
  }
  for (var attempt = 0; attempt <= 5; attempt++) {
    if (!normalRunning()) return;
    if (attempt < 5) pause();
  }
  throw new Error('Close Lumaglass before enabling Home integration');
}

function stopHome(appDir) {
  var stopped = [];
  fs.readdirSync('/proc').forEach(function (pid) {
    if (!/^\d+$/.test(pid)) return;
    try {
      var args = read('/proc/' + pid + '/cmdline').split('\0');
      var oldNative = args[0] === '/usr/bin/' + HOME;
      var flutter =
        path.basename(args[0]) === 'flutter-client' &&
        args.indexOf('-i') !== -1 &&
        args[args.indexOf('-i') + 1] === HOME;
      var executable = fs.readlinkSync('/proc/' + pid + '/exe').replace(/ \(deleted\)$/, '');
      var lumaglass = appDir ? executable === appDir + '/lumaglass' : exeOwned(executable);
      var metadata = null;
      try {
        metadata = JSON.parse(args[1]);
      } catch (error) {
        if (!(error instanceof SyntaxError)) throw error;
      }
      if (oldNative || flutter || (lumaglass && metadata && metadata.nid === HOME)) {
        process.kill(Number(pid), 'SIGTERM');
        stopped.push(pid);
      }
    } catch (error) {
      // A process may exit while /proc is being read. Kernel threads have no exe.
      if (error.code !== 'ENOENT' && error.code !== 'ESRCH') throw error;
    }
  });
  for (var i = 0; i < 5; i++) {
    if (
      !stopped.some(function (pid) {
        return fs.existsSync('/proc/' + pid);
      })
    )
      return;
    pause();
  }
  throw new Error('The previous Home process has not stopped');
}

function launchHome() {
  accepted(luna('launch', { id: HOME }), 'Open Home');
}

// SAM may answer a cold native launch late, and right after the previous Home process stopped it
// can accept a launch without starting anything: Lumaglass in front is the success condition.
function openExecutableHome() {
  for (var i = 0; i < 15; i++) {
    if (i === 0 || i === 6) {
      try {
        launchHome();
      } catch (error) {
        console.error('Lumaglass Home: ' + error.message);
      }
    }
    if (foreground() === HOME) return;
    pause();
  }
  throw new Error('Home did not come to the front');
}

function hookOwned() {
  try {
    return fs.readlinkSync(HOOK) === STATE + '/home.sh';
  } catch (error) {
    if (error.code === 'ENOENT' || error.code === 'EINVAL') return false;
    throw error;
  }
}

function clearHomeMarker() {
  if (fs.existsSync(MARKER) && read(MARKER).trim() === APP) fs.unlinkSync(MARKER);
}

function restoreExecutable(config) {
  assertHomeOwnership('executable');
  var wasHome = false;
  try {
    wasHome = foreground() === HOME;
  } catch (error) {
    console.error('Lumaglass Home: ' + error.message + ', restoring the files first');
  }
  var root = exeMount();
  if (root && !exeOwned(root)) throw new Error('Another integration is mounted over LG Home');
  clearHomeMarker();
  if (root) unmountExe();
  stopHome(config.appDir);
  // LG Home on webOS 5/6 is an overlay above the current app, not a foreground app.
  if (wasHome) launchHome();
}

function applyExecutable(config, open) {
  assertHomeOwnership('executable');
  assertNoForeignBoot();
  var root = exeMount();
  if (root && !exeOwned(root)) throw new Error('Another integration is mounted over LG Home');
  if (!fs.existsSync(config.appDir + '/lumaglass'))
    throw new Error('Lumaglass is no longer installed');
  var front = foreground();
  try {
    // After a package update the mount still holds the replaced file.
    if (root && !exeMounted(config.appDir)) unmountExe();
    if (!exeMounted(config.appDir))
      run('/bin/mount', ['--bind', config.appDir + '/lumaglass', EXE]);
    if (!exeMounted(config.appDir)) throw new Error('The Home bind mount did not take');
    fs.writeFileSync(MARKER, APP + '\n', { mode: 420 });
    stopHome(config.appDir);
    if (open || (front === HOME && foreground() === HOME)) openExecutableHome();
  } catch (error) {
    try {
      restoreExecutable(config);
    } catch (recovery) {
      throw new Error(error.message + '\nRestore also failed: ' + recovery.message);
    }
    throw error;
  }
}

function restore(config) {
  if (config.method === 'executable') return restoreExecutable(config);
  assertHomeOwnership('manifest');
  var wasHome = false;
  try {
    wasHome = foreground() === HOME;
  } catch (error) {
    console.error('Lumaglass Home: ' + error.message + ', restoring the files first');
  }
  if (foreignMount()) throw new Error('Another integration is mounted over LG Home');
  if (mounted()) {
    try {
      run('/bin/umount', [TARGET]);
    } catch (error) {
      // A running Home may still hold its working directory in the mount.
      console.error('Lumaglass Home: ' + error.message + ', detaching the mount');
      run('/bin/umount', ['-l', TARGET]);
    }
    if (mounted()) throw new Error('The Lumaglass Home mount is still active');
  }
  // Also finish a restore interrupted between unmounting and the rescan.
  // Read the uncovered files, since a firmware update may have changed LG Home.
  reread(json(TARGET + '/appinfo.json'));
  clearHomeMarker();
  stopHome(config.appDir);
  if (wasHome) launchHome();
}

// The LG splash is a static file. Stage the splash of the owner's current background, so a cold
// boot into Lumaglass Home opens on their wallpaper. Falls back to the package default on any error.
function stageSplash(appDir) {
  var background = 'dunes';
  var candidates = [
    '/media/developer/' + APP + '/preferences.json',
    '/media/developer/temp/' + APP + '/preferences.json',
    '/media/internal/.' + APP + '/preferences.json'
  ];
  // Match storage.rs: production native jails prefer internal storage. Reads
  // skip absent files, preserving an existing destination before migration.
  if (appDir.indexOf('/media/cryptofs/') === 0) candidates.unshift(candidates.pop());
  candidates.push(appDir + '/preferences.json');
  for (var i = 0; i < candidates.length; i++) {
    try {
      var saved = json(candidates[i]).background;
      if (typeof saved === 'string' && /^[a-z0-9-]{1,32}$/.test(saved)) {
        background = saved;
        break;
      }
    } catch (error) {
      if (error.code !== 'ENOENT') console.log('home: preferences unreadable: ' + error.message);
    }
  }
  var source = appDir + '/splash/' + background + '.png';
  if (!fs.existsSync(source)) source = appDir + '/splash.png';
  try {
    fs.writeFileSync(STAGE + '/splash.png.new', fs.readFileSync(source), { mode: 420 });
    fs.renameSync(STAGE + '/splash.png.new', STAGE + '/splash.png');
  } catch (error) {
    console.log('home: splash not staged: ' + error.message);
  }
}

function apply(config, open) {
  if (config.method === 'executable') return applyExecutable(config, open);
  assertHomeOwnership('manifest');
  assertNoForeignBoot();
  if (foreignMount()) throw new Error('Another integration is mounted over LG Home');
  if (!fs.existsSync(config.appDir + '/lumaglass'))
    throw new Error('Lumaglass is no longer installed');
  stageSplash(config.appDir);
  var front = foreground();
  try {
    if (!mounted()) run('/bin/mount', ['--bind', STAGE, TARGET]);
    if (!mounted()) throw new Error('The Home bind mount did not take');
    reread(json(STAGE + '/appinfo.json'));
    stopHome(config.appDir);
    // Only replace the screen if Home is still in front. During boot the
    // owner may have already selected an input or another app.
    if (open || (front === HOME && foreground() === HOME)) launchHome();
  } catch (error) {
    try {
      restore(config);
    } catch (recovery) {
      throw new Error(error.message + '\nRestore also failed: ' + recovery.message);
    }
    throw error;
  }
}

function installScripts() {
  ['home-mounts.cjs', 'home.cjs', 'home.sh'].forEach(function (name) {
    var target = STATE + '/' + name;
    var data = fs.readFileSync(path.join(__dirname, name));
    fs.writeFileSync(target + '.new', data, { mode: name === 'home.sh' ? 493 : 420 });
    fs.renameSync(target + '.new', target);
  });
}

function installHook() {
  mkdir('/var/lib/webosbrew');
  mkdir('/var/lib/webosbrew/init.d');
  if (!hookOwned()) fs.symlinkSync(STATE + '/home.sh', HOOK);
}

function install(appDir) {
  appDir = fs.realpathSync(appDir);
  var app = json(appDir + '/appinfo.json');
  if (app.id !== APP || app.type !== 'native' || app.main !== 'lumaglass') {
    throw new Error('Enable this from an installed native Lumaglass package');
  }
  assertHomeOwnership();
  assertNoForeignBoot();
  var release = read('/etc/starfish-release');
  // The executable method never mounts over LG's manifest, so it reads the original directly.
  var method = homeMethod(release, mounted() ? null : json(TARGET + '/appinfo.json'));
  assertHomeOwnership(method);
  if (foreignMount()) throw new Error('Another integration is mounted over LG Home');
  var exeRoot = exeMount();
  if (exeRoot && !exeOwned(exeRoot)) throw new Error('Another integration is mounted over LG Home');
  if (exists(HOOK) && !hookOwned()) throw new Error('64-lumaglass-home is already in use');
  if (exists('/var/lib/webosbrew/init.d/65-lumaglass')) {
    throw new Error('Disable the standalone cold-boot hook with autostart.sh disable first');
  }
  var previous = fs.existsSync(CONFIG) ? json(CONFIG) : null;
  var stock = mounted() ? previous && previous.stock : json(TARGET + '/appinfo.json');
  if (!stock || stock.id !== HOME || stock.main === 'lumaglass') {
    throw new Error('Could not identify the original LG Home manifest');
  }
  // Finish any normal tile instance before the Home identity starts using the
  // same settings. A later tile launch redirects in the native executable.
  closeNormal();
  if (method === 'executable') {
    installScripts();
    var exeConfig = { enabled: false, method: method, appDir: appDir, stock: stock };
    writeJson(CONFIG, exeConfig);
    installHook();
    apply(exeConfig, true);
    exeConfig.enabled = true;
    writeJson(CONFIG, exeConfig);
    console.log('Lumaglass is now Home. Home Auto Launch and Quick Start+ follow the TV settings.');
    return;
  }
  mkdir(STAGE);
  // Symlinks keep the executable at its installed path for the dynamic loader
  // and LS2. They also follow ordinary in-place package updates.
  ['lumaglass', 'assets', 'lib', 'splash', app.icon, app.largeIcon].forEach(function (name) {
    var link = STAGE + '/' + name;
    if (exists(link + '.new')) fs.unlinkSync(link + '.new');
    fs.symlinkSync(appDir + '/' + name, link + '.new');
    fs.renameSync(link + '.new', link);
  });
  app.id = HOME;
  app.title = 'Home';
  app.visible = false;
  app.appCategory = ['home'];
  writeJson(STAGE + '/appinfo.json', app);
  fs.writeFileSync(STAGE + '/.lumaglass-home', APP + '\n', { mode: 420 });
  installScripts();
  var config = { enabled: false, method: method, appDir: appDir, stock: stock };
  writeJson(CONFIG, config);
  installHook();
  apply(config, true);
  config.enabled = true;
  writeJson(CONFIG, config);
  console.log('Lumaglass is now Home. Home Auto Launch and Quick Start+ follow the TV settings.');
}

function recover() {
  // Observe every layer before touching hooks, state, mounts or processes.
  var report = assertHomeOwnership();
  var saved = null;
  try {
    if (fs.existsSync(CONFIG)) saved = json(CONFIG);
  } catch (error) {
    console.error('Lumaglass Home: saved state is unreadable: ' + error.message);
  }
  var method = report.executable.length ? 'executable' : report.manifest.length ? 'manifest' : null;
  var validSaved =
    saved &&
    (saved.method === 'executable' || saved.method === 'manifest' || !saved.method) &&
    typeof saved.appDir === 'string' &&
    path.isAbsolute(saved.appDir);
  if (!method && validSaved) method = saved.method || 'manifest';
  var appDir = validSaved ? saved.appDir : null;
  if (!appDir && method === 'manifest') {
    try {
      var link = fs.readlinkSync(STAGE + '/lumaglass');
      if (
        path.isAbsolute(link) &&
        path.basename(path.dirname(link)) === APP &&
        path.basename(link) === 'lumaglass'
      )
        appDir = path.dirname(link);
    } catch (error) {
      if (error.code !== 'ENOENT' && error.code !== 'EINVAL') throw error;
    }
  }
  // Disable only our boot hook first, so an interrupted recovery cannot reapply it.
  if (hookOwned()) fs.unlinkSync(HOOK);
  if (!method) {
    clearHomeMarker();
    console.log('No Lumaglass Home mount remains. Owned boot hook disabled; no apps restarted.');
    return;
  }
  var config = { enabled: false, method: method, appDir: appDir };
  try {
    writeJson(CONFIG, config);
  } catch (error) {
    // A broken state file must not prevent restoration of verified own mounts.
    console.error('Lumaglass Home: could not save disabled recovery state: ' + error.message);
  }
  restore(config);
  console.log('LG Home restored. Lumaglass Home boot integration disabled.');
}

try {
  var command = process.argv[2];
  if (command === 'enable') {
    install(process.argv[3]);
  } else if (command === 'status') {
    var saved = null;
    var stateError = null;
    var mountReport = null;
    var mountError = null;
    var bootReport = null;
    var bootError = null;
    try {
      if (fs.existsSync(CONFIG)) saved = json(CONFIG);
    } catch (error) {
      stateError = error.message;
    }
    try {
      mountReport = homeMounts();
    } catch (error) {
      mountError = error.message;
    }
    try {
      bootReport = bootIntegrations();
    } catch (error) {
      bootError = error.message;
    }
    console.log(
      JSON.stringify(
        {
          enabled: !!(saved && saved.enabled),
          method: saved ? saved.method || 'manifest' : null,
          mounted: saved && saved.method === 'executable' ? exeMounted(saved.appDir) : mounted(),
          bootHook: hookOwned(),
          appDir: saved ? saved.appDir : null,
          ownership: mountReport ? mountReport.home : 'unknown',
          conflict: mountReport ? mountReport.conflict : null,
          stateError: stateError,
          mountError: mountError,
          otherStartup: bootReport,
          startupError: bootError
        },
        null,
        2
      )
    );
  } else if (command === 'recover') {
    recover();
  } else if (command === 'apply' || command === 'disable' || command === 'restore') {
    if (!fs.existsSync(CONFIG)) throw new Error('Lumaglass Home has not been enabled');
    var config = json(CONFIG);
    if (command === 'apply') {
      if (config.enabled) apply(config, false);
      else console.log('Lumaglass Home integration is disabled; no changes applied.');
    } else {
      config.enabled = false;
      writeJson(CONFIG, config);
      restore(config);
      if (hookOwned()) fs.unlinkSync(HOOK);
      console.log('LG Home restored. Lumaglass Home boot integration disabled.');
    }
  } else {
    throw new Error('Unknown Home integration command');
  }
} catch (error) {
  console.error('Lumaglass Home: ' + error.message);
  process.exitCode = 1;
}
