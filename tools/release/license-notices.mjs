// Preserve the exact notices shipped with the locked Rust sources, including
// nested notices (e.g. ring's BoringSSL/fiat and unicode-ident's Unicode data).
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { root } from '../lib/project.mjs';

async function noticeFiles(directory, relative = '') {
  const result = [];
  for (const entry of await fs.readdir(path.join(directory, relative), { withFileTypes: true })) {
    const name = path.join(relative, entry.name);
    if (entry.isDirectory() && !['.git', 'target'].includes(entry.name)) {
      result.push(...(await noticeFiles(directory, name)));
    } else if (
      entry.isFile() &&
      /^(licen[sc]e|notice|copying|copyright)(?:[._-]|$)/i.test(entry.name)
    ) {
      result.push(name);
    }
  }
  return result.sort();
}

export async function generateRustNotices() {
  const pinnedDirectory = path.join(root, 'assets/licenses/rust');
  const pinned = JSON.parse(await fs.readFile(path.join(pinnedDirectory, 'sources.json'), 'utf8'));
  const metadata = spawnSync(
    'cargo',
    [
      'metadata',
      '--locked',
      '--format-version',
      '1',
      '--filter-platform',
      'armv7-unknown-linux-gnueabi',
      '--features',
      'webos',
    ],
    { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, timeout: 120000 },
  );
  if (metadata.error) throw metadata.error;
  if (metadata.status !== 0) throw new Error(`Cannot collect Rust licenses: ${metadata.stderr}`);
  const workspace = JSON.parse(metadata.stdout);
  const included = new Set(workspace.resolve.nodes.map((node) => node.id));
  const packages = workspace.packages
    .filter((pkg) => pkg.source && included.has(pkg.id))
    .sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  const sections = [
    '# Rust dependency notices\n',
    'Generated from Cargo.lock and the exact crate sources for the ARM webOS build.\n' +
      'Includes notices for build dependencies resolved alongside the TV runtime.\n' +
      'Declared license expressions do not replace the original texts below.\n',
  ];
  const inventory = [];
  for (const pkg of packages) {
    const directory = path.dirname(pkg.manifest_path);
    const files = await noticeFiles(directory);
    if (pkg.license_file) {
      const file = path.relative(directory, path.resolve(directory, pkg.license_file));
      if (!files.includes(file)) files.push(file);
    }
    const notices = files.map((file) => ({ name: file, path: path.resolve(directory, file) }));
    if (!notices.length) {
      const fallback = pinned[`${pkg.name}@${pkg.version}`];
      if (!fallback?.length) {
        throw new Error(
          `Missing source license texts for ${pkg.name}@${pkg.version}; refresh pinned notices.`,
        );
      }
      for (const entry of fallback) {
        if (!/^[a-zA-Z0-9_./-]+$/.test(entry.file) || entry.file.split('/').includes('..')) {
          throw new Error('Invalid pinned license path.');
        }
        const file = path.join(pinnedDirectory, entry.file);
        const bytes = await fs.readFile(file);
        if (createHash('sha256').update(bytes).digest('hex') !== entry.sha256) {
          throw new Error(`Pinned license checksum mismatch: ${entry.file}`);
        }
        notices.push({ name: entry.file, path: file, source: entry.source });
      }
      // gl-rs publishes its copyright in source headers, not in the tarball's license file.
      const source = pkg.targets.find((target) => target.kind.includes('lib'))?.src_path;
      if (source) {
        const lines = (await fs.readFile(source, 'utf8')).split('\n');
        const header = [];
        for (const line of lines) {
          if (!line.startsWith('//') && line.trim()) break;
          header.push(line);
        }
        if (/copyright/i.test(header.join('\n'))) {
          notices.push({ name: 'Original source copyright header', text: header.join('\n') });
        }
      }
    }
    if (!notices.length || (!pkg.license && !pkg.license_file)) {
      throw new Error(`Missing declared license or source notice for ${pkg.name}@${pkg.version}`);
    }
    inventory.push({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license,
      repository: pkg.repository,
      files: notices.map(({ name, source }) => ({ name, source })),
    });
    sections.push(
      `\n## ${pkg.name} ${pkg.version}\n\nDeclared license: ${pkg.license ?? 'see license file'}\nSource: ${pkg.repository ?? pkg.source}\n`,
    );
    for (const file of notices) {
      const text = file.text ?? (await fs.readFile(file.path, 'utf8'));
      if (!text.trim()) throw new Error(`Empty license notice: ${pkg.name}/${file.name}`);
      sections.push(
        `\n### ${file.name}\n${file.source ? `\nSource: ${file.source}\n` : ''}\n${text}\n`,
      );
    }
  }
  const destination = path.join(root, '.build/licenses');
  await fs.mkdir(destination, { recursive: true });
  await fs.writeFile(path.join(destination, 'rust-dependencies.txt'), sections.join('\n'));
  await fs.writeFile(
    path.join(destination, 'rust-dependencies.json'),
    `${JSON.stringify(inventory, null, 2)}\n`,
  );
  console.log(`Collected original Rust notices for ${packages.length} locked crates.`);
  return destination;
}
