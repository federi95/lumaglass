// Pinned host SDK archives. Verify before extraction or executing relocate-sdk.sh.
import fs from 'node:fs/promises';
import { createReadStream, constants } from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { root } from '../lib/project.mjs';

const parent = path.join(root, '.build/toolchain');
const dirname = 'arm-webos-linux-gnueabi_sdk-buildroot';
const sdk = path.join(parent, dirname);
const sources = JSON.parse(
  await fs.readFile(new URL('./sdk-sources.json', import.meta.url), 'utf8'),
);
const source = sources[`${process.platform}-${process.arch}`];
if (!source) throw new Error(`No pinned webOS SDK for ${process.platform}/${process.arch}.`);
if (/\s/.test(sdk)) throw new Error('Extract the webOS SDK in a path without spaces.');
await fs.mkdir(parent, { recursive: true });
const marker = path.join(sdk, '.lumaglass-sdk.json');
const compiler = path.join(sdk, 'bin/arm-webos-linux-gnueabi-gcc');
let installed;
try {
  installed = JSON.parse(await fs.readFile(marker, 'utf8'));
} catch (error) {
  if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
}
let usable = false;
try {
  await fs.access(compiler, constants.X_OK);
  await fs.access(path.join(sdk, 'arm-webos-linux-gnueabi/sysroot/usr/include/stdint.h'));
  usable = true;
} catch {}

function run(command, args, timeout) {
  const result = spawnSync(command, args, { stdio: 'inherit', timeout });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`SDK ${command} failed (${result.status}).`);
}
async function digest(file) {
  const hash = createHash('sha256');
  for await (const bytes of createReadStream(file)) hash.update(bytes);
  return hash.digest('hex');
}
if (
  !usable ||
  installed?.sha256 !== source.sha256 ||
  installed?.url !== source.url ||
  installed?.directory !== sdk
) {
  const archive = path.join(parent, path.basename(source.url));
  try {
    await fs.access(archive);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
    const temporary = `${archive}.${process.pid}.partial`;
    try {
      run(
        'curl',
        [
          '--fail',
          '--location',
          '--retry',
          '3',
          '--silent',
          '--show-error',
          '--output',
          temporary,
          source.url,
        ],
        300000,
      );
      if ((await digest(temporary)) !== source.sha256)
        throw new Error('webOS SDK archive checksum mismatch.');
      await fs.rename(temporary, archive);
    } finally {
      await fs.rm(temporary, { force: true });
    }
  }
  if ((await digest(archive)) !== source.sha256) {
    throw new Error(
      `webOS SDK cache checksum mismatch: ${archive}. Remove this archive and run pnpm sdk again.`,
    );
  }
  const staging = await fs.mkdtemp(path.join(parent, '.sdk-stage-'));
  const previous = path.join(staging, 'previous');
  let hadPrevious = false;
  let replaced = false;
  let cleanup = true;
  try {
    run('tar', [source.url.endsWith('.bz2') ? '-xjf' : '-xzf', archive, '-C', staging], 120000);
    const candidate = path.join(staging, dirname);
    await fs.access(path.join(candidate, 'bin/arm-webos-linux-gnueabi-gcc'), constants.X_OK);
    await fs.access(path.join(candidate, 'relocate-sdk.sh'));
    try {
      await fs.rename(sdk, previous);
      hadPrevious = true;
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    await fs.rename(candidate, sdk);
    replaced = true;
    // Relocate only at the final path; a later rename would invalidate embedded paths.
    run('sh', [path.join(sdk, 'relocate-sdk.sh')], 120000);
    await fs.access(compiler, constants.X_OK);
    await fs.access(path.join(sdk, 'arm-webos-linux-gnueabi/sysroot/usr/include/stdint.h'));
    await fs.writeFile(marker, `${JSON.stringify({ ...source, directory: sdk }, null, 2)}\n`);
  } catch (error) {
    // If rollback itself fails, retain the previous SDK in staging for recovery.
    if (hadPrevious) cleanup = false;
    if (replaced) await fs.rm(sdk, { recursive: true, force: true });
    if (hadPrevious) await fs.rename(previous, sdk);
    cleanup = true;
    throw error;
  } finally {
    if (cleanup) await fs.rm(staging, { recursive: true, force: true });
  }
}
console.log(`Verified webOS SDK ready. Set WEBOS_NDK=${sdk}`);
