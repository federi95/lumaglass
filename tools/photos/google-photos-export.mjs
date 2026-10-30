// User-initiated photo export. Google credentials remain in memory on this computer.
import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import sharp from 'sharp';
import { authorize } from './google-auth.mjs';
const api = 'https://photospicker.googleapis.com/v1';
const validToken = (s, max) =>
  typeof s === 'string' && s.length > 0 && s.length <= max && /^[\x21-\x7e]+$/.test(s);
export async function freshOutput(output) {
  const parent = path.dirname(output);
  await fs.mkdir(parent, { recursive: true, mode: 0o700 });
  const metadata = await fs.lstat(parent);
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    (process.getuid && (metadata.uid !== process.getuid() || (metadata.mode & 0o022) !== 0))
  )
    throw new Error('Use an owned output directory.');
  if (
    await fs.lstat(output).then(
      () => true,
      (e) => {
        if (e.code === 'ENOENT') return false;
        throw e;
      },
    )
  )
    throw new Error('Choose a new export directory; existing files are preserved.');
}
function duration(value, maximum) {
  if (typeof value !== 'string' || !/^\d+(?:\.\d+)?s$/.test(value))
    throw new Error('Invalid selection timing.');
  const seconds = Number(value.slice(0, -1));
  if (!Number.isFinite(seconds) || seconds > maximum) throw new Error('Invalid selection timing.');
  return seconds * 1000;
}
export function mediaUrl(base) {
  const url = new URL(base);
  if (
    url.protocol !== 'https:' ||
    !url.hostname.endsWith('.googleusercontent.com') ||
    url.username ||
    url.password ||
    url.port ||
    url.search ||
    url.hash ||
    base.length > 4096
  )
    throw new Error('Unsupported photo address.');
  return `${base}=w1920-h1080`;
}
export async function exportSelection({
  accessToken,
  output,
  onPicker,
  fetchMedia = globalThis.fetch,
  sleep = delay,
  signal = AbortSignal.timeout(600000),
  now = Date.now,
}) {
  if (!validToken(accessToken, 4096)) throw new Error('Authorization is required.');
  await freshOutput(output);
  const stage = `${output}.${randomUUID()}.tmp`;
  await fs.mkdir(stage, { mode: 0o700 });
  let sessionId;
  let committed = false;
  async function request(url, method = 'GET', body, limit = 2 * 1024 * 1024, cleanup = false) {
    const response = await fetchMedia(url, {
      method,
      headers: {
        Authorization: `Bearer ${accessToken}`,
        ...(body ? { 'Content-Type': 'application/json' } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
      redirect: 'error',
      signal: cleanup
        ? AbortSignal.timeout(5000)
        : AbortSignal.any([signal, AbortSignal.timeout(15000)]),
    });
    if (!response.ok) throw new Error('Photo selection or download was refused.');
    if (Number(response.headers.get('content-length')) > limit) {
      await response.body?.cancel();
      throw new Error('Photo response exceeds the import limit.');
    }
    const chunks = [];
    let size = 0;
    for await (const chunk of response.body ?? []) {
      size += chunk.length;
      if (size > limit) throw new Error('Photo response exceeds the import limit.');
      chunks.push(chunk);
    }
    return Buffer.concat(chunks);
  }
  async function json(url, method = 'GET', body) {
    const bytes = await request(url, method, body);
    try {
      return JSON.parse(bytes.toString('utf8'));
    } catch {
      throw new Error('Invalid photo service response.');
    }
  }
  let cleanupConfirmed = false;
  try {
    const session = await json(`${api}/sessions`, 'POST', {
      pickingConfig: { maxItemCount: '12' },
    });
    if (!validToken(session.id, 512)) throw new Error('Invalid photo selection session.');
    sessionId = session.id;
    const uri = new URL(session.pickerUri);
    if (
      uri.origin !== 'https://photos.google.com' ||
      !uri.pathname.includes('/picker/') ||
      uri.username ||
      uri.password ||
      uri.href.length > 2048
    )
      throw new Error('Invalid photo selection address.');
    const sessionUrl = `${api}/sessions/${encodeURIComponent(sessionId)}`;
    let interval = Math.max(2000, duration(session.pollingConfig?.pollInterval, 300));
    const deadline = now() + Math.min(duration(session.pollingConfig?.timeoutIn, 3600), 600000);
    await onPicker(uri.href);
    while (true) {
      signal.throwIfAborted();
      if (now() + interval >= deadline) throw new Error('Photo selection expired.');
      await sleep(interval, undefined, { signal });
      const status = await json(sessionUrl);
      if (status.mediaItemsSet === true) break;
      interval = Math.max(2000, duration(status.pollingConfig?.pollInterval, 300));
    }
    const files = [];
    const seen = new Set();
    const pages = new Set();
    let page = '';
    let total = 0;
    for (let pass = 0; pass < 3 && files.length < 12; pass++) {
      const url = new URL(`${api}/mediaItems`);
      url.search = new URLSearchParams({ sessionId, pageSize: '100', pageToken: page });
      const result = await json(url.href);
      if (!Array.isArray(result.mediaItems) || result.mediaItems.length > 100)
        throw new Error('Photo listing exceeds the import limit.');
      for (const item of result.mediaItems) {
        if (
          item.type !== 'PHOTO' ||
          typeof item.mediaFile?.mimeType !== 'string' ||
          !item.mediaFile.mimeType.startsWith('image/')
        )
          continue;
        if (!validToken(item.id, 512)) throw new Error('Invalid photo listing.');
        if (seen.has(item.id)) continue;
        seen.add(item.id);
        const bytes = await request(
          mediaUrl(item.mediaFile.baseUrl),
          'GET',
          undefined,
          8 * 1024 * 1024,
        );
        total += bytes.length;
        if (total > 32 * 1024 * 1024) throw new Error('Photo download budget exceeded.');
        const supported =
          bytes.subarray(0, 3).equals(Buffer.from([0xff, 0xd8, 0xff])) ||
          bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])) ||
          (bytes.subarray(0, 4).toString() === 'RIFF' &&
            bytes.subarray(8, 12).toString() === 'WEBP');
        if (!supported) throw new Error('Unsupported photo response.');
        const data = await sharp(bytes, { limitInputPixels: 24_000_000 })
          .rotate()
          .resize({ width: 1920, height: 1080, fit: 'inside', withoutEnlargement: true })
          .jpeg({ quality: 90 })
          .toBuffer();
        const name = `photo-${String(files.length + 1).padStart(2, '0')}.jpg`;
        await fs.writeFile(path.join(stage, name), data, { mode: 0o600, flag: 'wx' });
        files.push(name);
        if (files.length === 12) break;
      }
      if (!result.nextPageToken) break;
      if (!validToken(result.nextPageToken, 2048) || pages.has(result.nextPageToken))
        throw new Error('Invalid photo pagination.');
      pages.add(result.nextPageToken);
      page = result.nextPageToken;
    }
    if (files.length === 0) throw new Error('No supported photos were selected.');
    await fs.writeFile(
      path.join(stage, 'google-photos-export.json'),
      `${JSON.stringify({ schema: 1, source: 'google-photos-picker', files }, null, 2)}\n`,
      { mode: 0o600, flag: 'wx' },
    );
    signal.throwIfAborted();
    await freshOutput(output);
    await fs.rename(stage, output);
    committed = true;
    return {
      count: files.length,
      output,
      get cleanupConfirmed() {
        return cleanupConfirmed;
      },
    };
  } finally {
    if (sessionId) {
      try {
        await request(
          `${api}/sessions/${encodeURIComponent(sessionId)}`,
          'DELETE',
          undefined,
          32768,
          true,
        );
        cleanupConfirmed = true;
      } catch {
        /* The selection session also has a bounded server lifetime. */
      }
    }
    if (!committed) await fs.rm(stage, { recursive: true, force: true });
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const args = process.argv.slice(2);
  const value = (key) => (args.includes(key) ? args[args.indexOf(key) + 1] : undefined);
  const controller = new AbortController();
  process.once('SIGINT', () => controller.abort());
  const show = (url) => {
    console.log(`Open in your browser:\n${url}`);
    if (args.includes('--open'))
      execFile(process.platform === 'darwin' ? 'open' : 'xdg-open', [url], () => {});
  };
  try {
    if (!value('--out') || !value('--client'))
      throw new Error('Choose a client and a new export directory.');
    const output = path.resolve(value('--out'));
    await freshOutput(output);
    console.log(
      'Export up to 12 photos you choose into a local folder. Only the chosen photos are downloaded; Google tokens are never saved.',
    );
    const authorization = await authorize({
      provider: 'google-photos',
      persist: false,
      signal: controller.signal,
      clientPath: path.resolve(value('--client')),
      onReady: async ({ url }) => show(url),
    });
    const result = await exportSelection({
      accessToken: authorization.accessToken,
      output,
      onPicker: async (url) => show(url),
      signal: controller.signal,
    });
    console.log(
      `Exported ${result.count} photos. Copy this private folder to the TV and configure its local Google Photos export source.`,
    );
    if (!result.cleanupConfirmed)
      console.log('Selection-session cleanup could not be confirmed; it will expire on Google.');
  } catch {
    console.error(
      'Export failed. Existing exports are preserved. Check Google API access, the private OAuth client file and the chosen output directory.',
    );
    process.exitCode = 1;
  }
}
