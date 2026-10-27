// Embedded in the native executable and run on its platform worker through
// Homebrew's elevated exec service. Keep ES5 syntax for webOS's Node 8.
var fs = require('fs');
var path = require('path');
var selected = JSON.parse(process.argv[1]);
var APP = 'org.lumaglass.app';
var STATE = '/var/lib/lumaglass-home';
var roots = [
  '/media/developer/apps/usr/palm/applications',
  '/media/cryptofs/apps/usr/palm/applications',
  '/usr/palm/applications'
];

if (
  typeof selected.directory !== 'string' ||
  !roots.some(function (root) {
    return selected.directory === root + '/' + APP;
  })
) {
  throw new Error('Invalid Lumaglass installation directory');
}
if (typeof selected.background !== 'string' || !/^[a-z0-9-]{1,32}$/.test(selected.background)) {
  throw new Error('Invalid splash background');
}
var appDir = selected.directory;
var app = JSON.parse(fs.readFileSync(appDir + '/appinfo.json', 'utf8'));
if (app.id !== APP || app.type !== 'native' || app.main !== 'lumaglass') {
  throw new Error('Splash updates require an installed native Lumaglass package');
}
// Read the selection already loaded by the native app, rather than trying to
// rediscover its preferences from outside the jail. Never copy the default over
// a working splash if an update is missing the selected image.
var source = path.join(appDir, 'splash', selected.background + '.png');
if (selected.personal !== undefined && selected.personal !== null) {
  if (typeof selected.personal !== 'string' || !/^personal-[a-f0-9]{16}$/.test(selected.background))
    throw Error('Invalid personal splash');
  var allowed = [
    '/media/developer/' + APP,
    '/media/developer/temp/' + APP,
    '/media/internal/.' + APP
  ];
  if (
    !allowed.some(function (base) {
      return selected.personal === base + '/preferences.wallpapers/' + selected.background + '.png';
    })
  )
    throw Error('Invalid personal cache path');
  if (
    fs.lstatSync(path.dirname(selected.personal)).isSymbolicLink() ||
    fs.lstatSync(selected.personal).isSymbolicLink()
  )
    throw Error('Symlink splash refused');
  if (fs.statSync(selected.personal).size > 16 * 1024 * 1024)
    throw Error('Personal splash too large');
  source = selected.personal;
}
var image = fs.readFileSync(source);
if (
  selected.personal &&
  (image.length < 24 ||
    image.slice(0, 8).toString('hex') !== '89504e470d0a1a0a' ||
    image.readUInt32BE(16) !== 1920 ||
    image.readUInt32BE(20) !== 1080)
)
  throw Error('Invalid personal PNG');

function replace(file) {
  try {
    if (fs.readFileSync(file).equals(image)) return;
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  fs.writeFileSync(file + '.new', image, { mode: 420 });
  fs.renameSync(file + '.new', file);
}

// SAM's normal tile uses the installed manifest's splash.png. Updating only
// the staged Home copy leaves every normal launch on the package default.
replace(appDir + '/splash.png');
var config = null;
try {
  config = JSON.parse(fs.readFileSync(STATE + '/state.json', 'utf8'));
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
}
// Only the manifest method stages a Home copy. The executable method (webOS 5/6)
// keeps LG's own Home manifest, so there is no staged splash to update.
if (config && config.enabled && config.method !== 'executable' && config.appDir === appDir) {
  replace(STATE + '/app/splash.png');
}
console.log(JSON.stringify({ returnValue: true, background: selected.background }));
