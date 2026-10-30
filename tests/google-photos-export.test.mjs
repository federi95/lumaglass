// CI only: fake Picker responses and generated fixture pixels, no real account.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import sharp from 'sharp';
import { exportSelection, mediaUrl } from '../tools/photos/google-photos-export.mjs';
const json = (data) =>
  new Response(JSON.stringify(data), { headers: { 'Content-Type': 'application/json' } });
const session = {
  id: 'fixture-session',
  pickerUri: 'https://photos.google.com/picker/fixture',
  pollingConfig: { pollInterval: '1s', timeoutIn: '60s' },
};
const selected = (baseUrl = 'https://lh3.googleusercontent.com/fixture') => ({
  mediaItems: [
    {
      id: 'private-photo-id',
      type: 'PHOTO',
      mediaFile: { baseUrl, mimeType: 'image/png', filename: 'never-save-this-name.png' },
    },
  ],
});
test('Picker export observes polling, bounds photo bytes and saves a private user export without tokens or identifiers', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'lumaglass-picker-'));
  const output = path.join(dir, 'export');
  const calls = [];
  const waits = [];
  let polls = 0;
  const fixture = await sharp({
    create: { width: 8, height: 8, channels: 3, background: '#347bbd' },
  })
    .png()
    .toBuffer();
  try {
    const result = await exportSelection({
      accessToken: 'fixture-access',
      output,
      onPicker: async (url) => assert.equal(url, session.pickerUri),
      sleep: async (ms) => waits.push(ms),
      now: () => 0,
      fetchMedia: async (url, options) => {
        calls.push([url, options.method]);
        assert.equal(options.redirect, 'error');
        assert.equal(options.headers.Authorization, 'Bearer fixture-access');
        if (options.method === 'DELETE') return new Response(null, { status: 204 });
        if (options.method === 'POST') {
          assert.deepEqual(JSON.parse(options.body), { pickingConfig: { maxItemCount: '12' } });
          return json(session);
        }
        if (url.includes('/sessions/'))
          return json(
            ++polls === 1
              ? { mediaItemsSet: false, pollingConfig: { pollInterval: '4s' } }
              : { mediaItemsSet: true },
          );
        if (url.includes('/mediaItems?')) return json(selected());
        assert.equal(url, 'https://lh3.googleusercontent.com/fixture=w1920-h1080');
        return new Response(fixture, { headers: { 'Content-Type': 'image/png' } });
      },
    });
    assert.equal(result.count, 1);
    assert.equal(result.cleanupConfirmed, true);
    assert.deepEqual(waits, [2000, 4000]);
    const manifest = await fs.readFile(path.join(output, 'google-photos-export.json'), 'utf8');
    for (const value of [
      'fixture-access',
      'private-photo-id',
      'never-save-this-name',
      'googleusercontent.com',
    ])
      assert.ok(!manifest.includes(value));
    assert.deepEqual(JSON.parse(manifest).files, ['photo-01.jpg']);
    assert.equal((await sharp(path.join(output, 'photo-01.jpg')).metadata()).format, 'jpeg');
    assert.equal((await fs.stat(output)).mode & 0o777, 0o700);
    assert.equal((await fs.stat(path.join(output, 'photo-01.jpg'))).mode & 0o777, 0o600);
    assert.deepEqual(calls.at(-1), [
      'https://photospicker.googleapis.com/v1/sessions/fixture-session',
      'DELETE',
    ]);
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
});
test('Photo URLs reject credentials, HTTP, foreign hosts, ports and query parameters', () => {
  for (const url of [
    'http://lh3.googleusercontent.com/a',
    'https://user:pass@lh3.googleusercontent.com/a',
    'https://googleusercontent.com.evil.example/a',
    'https://lh3.googleusercontent.com:444/a',
    'https://lh3.googleusercontent.com/a?token=secret',
  ])
    assert.throws(() => mediaUrl(url));
});
test('A rejected media address cleans only staging and its session; earlier exports survive', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'lumaglass-picker-'));
  const output = path.join(dir, 'new');
  const old = path.join(dir, 'old');
  await fs.mkdir(old);
  await fs.writeFile(path.join(old, 'photo.jpg'), 'retained');
  const calls = [];
  try {
    await assert.rejects(
      exportSelection({
        accessToken: 'fixture-access',
        output,
        onPicker: async () => {},
        sleep: async () => {},
        now: () => 0,
        fetchMedia: async (url, options) => {
          calls.push([url, options.method]);
          if (options.method === 'DELETE') return new Response(null, { status: 204 });
          if (options.method === 'POST') return json(session);
          if (url.includes('/sessions/')) return json({ mediaItemsSet: true });
          return json(selected('https://evil.example/steal'));
        },
      }),
    );
    assert.ok(!calls.some(([url]) => url.includes('evil.example')));
    assert.equal(await fs.readFile(path.join(old, 'photo.jpg'), 'utf8'), 'retained');
    await assert.rejects(fs.access(output));
    assert.deepEqual((await fs.readdir(dir)).sort(), ['old']);
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
});
test('Existing export is never overwritten or sent to the service', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'lumaglass-picker-'));
  let contacted = false;
  try {
    await assert.rejects(
      exportSelection({
        accessToken: 'fixture-access',
        output: dir,
        onPicker: async () => {},
        fetchMedia: async () => {
          contacted = true;
          throw new Error('no request expected');
        },
      }),
    );
    assert.equal(contacted, false);
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
});
