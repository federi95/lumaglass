// Explicit desktop setup only: system-browser OAuth, PKCE and private refresh-token output.
import fs from 'node:fs/promises';
import path from 'node:path';
import http from 'node:http';
import { randomBytes, randomUUID, createHash, timingSafeEqual } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { execFile } from 'node:child_process';
export const scopes = {
  drive: 'https://www.googleapis.com/auth/drive.readonly',
  'google-photos': 'https://www.googleapis.com/auth/photospicker.mediaitems.readonly',
};
const tokenEndpoint = 'https://oauth2.googleapis.com/token';
const token = (s, max) =>
  typeof s === 'string' && s.length > 0 && s.length <= max && /^[\x21-\x7e]+$/.test(s);
export function authorizationUrl({ clientId, provider, redirectUri, state, verifier }) {
  if (
    !scopes[provider] ||
    !token(clientId, 256) ||
    !clientId.endsWith('.apps.googleusercontent.com')
  )
    throw new Error('Invalid Google client or service.');
  const url = new URL('https://accounts.google.com/o/oauth2/v2/auth');
  url.search = new URLSearchParams({
    client_id: clientId,
    redirect_uri: redirectUri,
    response_type: 'code',
    scope: scopes[provider],
    access_type: provider === 'drive' ? 'offline' : 'online',
    prompt: 'consent',
    state,
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  }).toString();
  return url.href;
}
async function privateMetadata(file, optional = false) {
  const meta = await fs.lstat(file).catch((e) => {
    if (optional && e.code === 'ENOENT') return null;
    throw e;
  });
  if (
    meta &&
    (!meta.isFile() ||
      meta.isSymbolicLink() ||
      meta.size > 65536 ||
      (process.getuid && (meta.uid !== process.getuid() || (meta.mode & 0o077) !== 0)))
  )
    throw new Error('Use an owned regular file with mode 0600.');
  return meta;
}
async function writePrivate(file, record, signal) {
  const parent = path.dirname(file);
  await fs.mkdir(parent, { recursive: true, mode: 0o700 });
  const dir = await fs.lstat(parent);
  if (
    !dir.isDirectory() ||
    dir.isSymbolicLink() ||
    (process.getuid && (dir.uid !== process.getuid() || (dir.mode & 0o022) !== 0))
  )
    throw new Error('Use an owned, non-writable-by-others output directory.');
  await privateMetadata(file, true);
  const data = Buffer.from(`${JSON.stringify(record, null, 2)}\n`);
  if (data.length > 4096) throw new Error('Connection file exceeds the native limit.');
  const temporary = `${file}.${randomUUID()}.tmp`;
  try {
    const handle = await fs.open(temporary, 'wx', 0o600);
    try {
      await handle.writeFile(data);
      await handle.sync();
    } finally {
      await handle.close();
    }
    await privateMetadata(file, true);
    signal.throwIfAborted();
    await fs.rename(temporary, file);
  } finally {
    await fs.rm(temporary, { force: true });
  }
}
export async function authorize({
  provider,
  persist = provider === 'drive',
  clientPath,
  output,
  folders = [],
  onReady,
  fetchToken = globalThis.fetch,
  timeoutMs = 240000,
  signal,
}) {
  signal?.throwIfAborted();
  if (
    !scopes[provider] ||
    folders.length > 3 ||
    new Set(folders).size !== folders.length ||
    folders.some((id) => !/^[\w-]{1,256}$/.test(id)) ||
    (provider !== 'drive' && folders.length)
  )
    throw new Error('Choose a service and up to three Drive folder IDs.');
  if (
    persist &&
    (provider !== 'drive' || typeof output !== 'string' || !output.endsWith('.google-drive.json'))
  )
    throw new Error('Output must end with .google-drive.json or .google-photos.json.');
  await privateMetadata(clientPath);
  const source = JSON.parse(await fs.readFile(clientPath, 'utf8'));
  const client = source.installed;
  if (!client || !token(client.client_id, 256) || !token(client.client_secret, 512))
    throw new Error('A Google Desktop application client file is required.');
  const state = randomBytes(32).toString('base64url');
  const verifier = randomBytes(32).toString('base64url');
  const lifetime = new AbortController();
  let finish,
    fail,
    handled = false;
  const result = new Promise((resolve, reject) => {
    finish = resolve;
    fail = reject;
  });
  // Attach a handler before exposing the endpoint so setup errors never cause unhandled rejection.
  result.catch(() => {});
  const server = http.createServer(async (req, res) => {
    const expectedHost = `127.0.0.1:${server.address().port}`;
    if (req.method !== 'GET' || req.headers.host !== expectedHost || handled) {
      res.writeHead(400).end();
      return;
    }
    const url = new URL(req.url, `http://${expectedHost}`);
    const supplied = url.searchParams.get('state') ?? '';
    if (
      url.pathname !== '/oauth2callback' ||
      Buffer.byteLength(supplied) !== Buffer.byteLength(state) ||
      !timingSafeEqual(Buffer.from(supplied), Buffer.from(state))
    ) {
      res.writeHead(400).end();
      return;
    }
    handled = true;
    if (url.searchParams.has('error') || !token(url.searchParams.get('code'), 4096)) {
      res.writeHead(400).end('Authorization was not completed.');
      fail(new Error('Authorization was not completed.'));
      return;
    }
    try {
      const response = await fetchToken(tokenEndpoint, {
        method: 'POST',
        redirect: 'error',
        signal: AbortSignal.any([lifetime.signal, AbortSignal.timeout(15000)]),
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams({
          client_id: client.client_id,
          client_secret: client.client_secret,
          code: url.searchParams.get('code'),
          code_verifier: verifier,
          grant_type: 'authorization_code',
          redirect_uri: `http://${expectedHost}/oauth2callback`,
        }),
      });
      if (!response.ok) throw new Error('Google refused authorization.');
      const parts = [];
      let length = 0;
      for await (const part of response.body) {
        length += part.length;
        if (length > 65536) throw new Error('Invalid authorization response.');
        parts.push(part);
      }
      const credentials = JSON.parse(Buffer.concat(parts).toString('utf8'));
      lifetime.signal.throwIfAborted();
      if (
        (persist
          ? !token(credentials.refresh_token, 2048)
          : !token(credentials.access_token, 4096)) ||
        credentials.token_type?.toLowerCase() !== 'bearer' ||
        (credentials.scope && !credentials.scope.split(/\s+/).includes(scopes[provider]))
      )
        throw new Error('The requested read-only grant was not returned.');
      if (persist)
        await writePrivate(
          output,
          {
            schema: 1,
            client_id: client.client_id,
            client_secret: client.client_secret,
            refresh_token: credentials.refresh_token,
            scope: scopes[provider],
            folders,
          },
          lifetime.signal,
        );
      res
        .writeHead(200, {
          'Content-Type': 'text/plain; charset=utf-8',
          'Cache-Control': 'no-store',
        })
        .end('Lumaglass authorization complete. You can close this window.');
      finish(persist ? output : { accessToken: credentials.access_token, scope: scopes[provider] });
    } catch {
      res.writeHead(400).end('Authorization could not be saved.');
      fail(new Error('Authorization could not be saved; no credentials were printed.'));
    }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const cancelled = () => {
    lifetime.abort();
    fail(new Error('Authorization cancelled.'));
  };
  signal?.addEventListener('abort', cancelled, { once: true });
  if (signal?.aborted) cancelled();
  const timeout = setTimeout(() => {
    lifetime.abort();
    fail(new Error('Authorization timed out.'));
  }, timeoutMs);
  try {
    const redirectUri = `http://127.0.0.1:${server.address().port}/oauth2callback`;
    const url = authorizationUrl({
      clientId: client.client_id,
      provider,
      redirectUri,
      state,
      verifier,
    });
    await onReady({ url, redirectUri });
    return await result;
  } finally {
    clearTimeout(timeout);
    lifetime.abort();
    signal?.removeEventListener('abort', cancelled);
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const args = process.argv.slice(2);
  const value = (key) => (args.includes(key) ? args[args.indexOf(key) + 1] : undefined);
  const provider = value('--provider');
  const folderIds = [];
  for (let i = 0; i < args.length; i++) if (args[i] === '--folder') folderIds.push(args[++i]);
  try {
    if (provider !== 'drive') throw new Error('Google Photos uses the explicit export helper.');
    const output = path.resolve(value('--out') ?? '');
    await authorize({
      provider,
      clientPath: path.resolve(value('--client') ?? ''),
      output,
      folders: folderIds,
      onReady: async ({ url }) => {
        console.log(`Open this Google authorization page:\n${url}`);
        if (args.includes('--open'))
          execFile(process.platform === 'darwin' ? 'open' : 'xdg-open', [url], () => {});
      },
    });
    console.log(
      'Private connection saved. Transfer it with mode 0600 beside the TV preferences file.',
    );
  } catch {
    console.error(
      'Setup failed. Check --provider, --client, --out, private file permissions and Google API configuration. No credentials were printed.',
    );
    process.exitCode = 1;
  }
}
