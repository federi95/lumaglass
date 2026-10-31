// Lumaglass Home ownership observations. No mutations. Node 8-compatible.
var TARGET = '/usr/palm/applications/com.webos.app.home';
var EXE = '/usr/bin/com.webos.app.home';
var STAGE = '/var/lib/lumaglass-home/app';
var LIMIT = 256 * 1024;

function executableOwned(source) {
  return /\/org\.lumaglass\.app\/lumaglass(\/\/deleted)?$/.test(source || '');
}

function unescapePath(value) {
  return value.replace(/\\(040|011|012|134)/g, function (_, octal) {
    return String.fromCharCode(parseInt(octal, 8));
  });
}

function inspect(text, stagePath) {
  if (typeof text !== 'string' || !text.trim() || text.length > LIMIT)
    throw new Error('Home mount information is empty or exceeds its limit');
  var manifest = [];
  var executable = [];
  var foreign = false;
  var records = [];
  text.split('\n').forEach(function (line) {
    if (!line) return;
    var fields = line.split(' ');
    var separator = fields.indexOf('-');
    if (separator < 6 || fields.length < separator + 4 || !/^\d+$/.test(fields[0]))
      throw new Error('Invalid Home mount information');
    records.push({
      id: fields[0],
      parent: fields[1],
      target: unescapePath(fields[4]),
      source: unescapePath(fields[3]),
      device: fields[2]
    });
  });
  // mountinfo roots are relative to their filesystem, not necessarily to '/'.
  // Resolve the staged app through its deepest containing mount (e.g. /var).
  stagePath = stagePath || STAGE;
  var containers = [];
  var deepest = 0;
  var byId = Object.create(null);
  records.forEach(function (record) {
    byId[record.id] = record;
    var point = record.target;
    if (point === '/' || stagePath === point || stagePath.indexOf(point + '/') === 0) {
      if (point.length > deepest) {
        deepest = point.length;
        containers = [];
      }
      if (point.length === deepest) containers.push(record);
    }
  });
  // webOS can overmount /var itself: the active ext4 mount is a child of
  // the covered overlay at the same path. Mountinfo line order is not proof
  // of visibility. Require one descendant of every same-path candidate.
  var visible = containers.filter(function (candidate) {
    return containers.every(function (other) {
      var current = candidate;
      for (var steps = 0; current && steps < records.length; steps++) {
        if (current === other) return true;
        current = byId[current.parent];
      }
      return false;
    });
  });
  if (containers.length && visible.length !== 1)
    throw new Error('Ambiguous containing mounts for the staged Home app');
  var container = visible[0] || null;
  var stageRoot = container
    ? container.source.replace(/\/$/, '') +
      stagePath.slice(container.target === '/' ? 0 : container.target.length)
    : null;
  records.forEach(function (record) {
    var target = record.target;
    var source = record.source;
    var list;
    var owned = false;
    if (target === TARGET || target.indexOf(TARGET + '/') === 0) {
      list = manifest;
      // Lumaglass creates one whole-app mount, never child mounts.
      owned =
        target === TARGET &&
        container &&
        source === stageRoot &&
        record.device === container.device;
    } else if (target === EXE || target.indexOf(EXE + '/') === 0) {
      list = executable;
      owned = target === EXE && executableOwned(source);
    } else return;
    list.push({ source: source, target: target, owned: !!owned });
    foreign = foreign || !owned;
  });
  var conflict =
    foreign ||
    manifest.length > 1 ||
    executable.length > 1 ||
    (manifest.length > 0 && executable.length > 0);
  return {
    home: foreign
      ? 'other'
      : conflict
        ? 'unknown'
        : manifest.length + executable.length
          ? 'lumaglass'
          : 'stock',
    conflict: conflict,
    manifest: manifest,
    executable: executable
  };
}

function snapshot(fs) {
  var fd = fs.openSync('/proc/self/mountinfo', 'r');
  try {
    var bytes = Buffer.alloc(LIMIT + 1);
    var count = 0;
    while (count < bytes.length) {
      var next = fs.readSync(fd, bytes, count, bytes.length - count, null);
      if (!next) break;
      count += next;
    }
    if (count > LIMIT) throw new Error('Home mount information exceeds its limit');
    var stagePath;
    try {
      stagePath = fs.realpathSync(STAGE);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      // The stage may not exist yet. Still account for a symlinked /var.
      stagePath = fs.realpathSync('/var/lib') + '/lumaglass-home/app';
    }
    return inspect(bytes.slice(0, count).toString('utf8'), stagePath);
  } finally {
    fs.closeSync(fd);
  }
}

// Only known Home integration state, not the shared Glasshouse service hook.
// Presence of 50-tvweb.sh alone says nothing about its Home takeover choice.
function boot(fs) {
  var luma = 'absent';
  try {
    fs.lstatSync('/var/lib/webosbrew/init.d/64-luma-home');
    luma = 'present';
  } catch (error) {
    if (error.code !== 'ENOENT') luma = 'unknown';
  }
  var glasshouse = 'absent';
  var fd;
  try {
    fd = fs.openSync('/var/lib/tvweb/home.json', 'r');
    var bytes = Buffer.alloc(64 * 1024 + 1);
    var size = 0;
    while (size < bytes.length) {
      var count = fs.readSync(fd, bytes, size, bytes.length - size, null);
      if (!count) break;
      size += count;
    }
    if (size === bytes.length) glasshouse = 'unknown';
    else {
      var state = JSON.parse(bytes.slice(0, size).toString('utf8'));
      glasshouse =
        state && state.enabled === true
          ? 'enabled'
          : state && state.enabled === false
            ? 'disabled'
            : 'unknown';
    }
  } catch (error) {
    if (error.code !== 'ENOENT') glasshouse = 'unknown';
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
  return {
    luma: luma,
    glasshouse: glasshouse,
    foreign: luma === 'present' || glasshouse === 'enabled',
    uncertain: luma === 'unknown' || glasshouse === 'unknown'
  };
}

module.exports = {
  inspect: inspect,
  snapshot: snapshot,
  executableOwned: executableOwned,
  boot: boot
};
