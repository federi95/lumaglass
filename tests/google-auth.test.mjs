// CI-only OAuth fixtures. Google endpoints are asserted but never contacted.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { authorize, authorizationUrl, scopes } from '../tools/photos/google-auth.mjs';
test('Google authorization URL uses exact read-only scope and S256 PKCE', () => {
  const url = new URL(
    authorizationUrl({
      clientId: 'fixture.apps.googleusercontent.com',
      provider: 'drive',
      redirectUri: 'http://127.0.0.1:1234/oauth2callback',
      state: 'fixture-state',
      verifier: 'fixture-verifier',
    }),
  );
  assert.equal(url.origin, 'https://accounts.google.com');
  assert.equal(url.searchParams.get('scope'), scopes.drive);
  assert.equal(url.searchParams.get('code_challenge_method'), 'S256');
  assert.equal(
    url.searchParams.get('code_challenge'),
    createHash('sha256').update('fixture-verifier').digest('base64url'),
  );
  assert.equal(url.searchParams.has('client_secret'), false);
});
for (const provider of ['drive', 'google-photos'])
  test(`${provider} loopback validates state and saves only private refresh credentials`, async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'lumaglass-oauth-'));
    const client = path.join(dir, 'client.json');
    const output = path.join(
      dir,
      provider === 'drive' ? 'preferences.google-drive.json' : 'preferences.google-photos.json',
    );
    await fs.writeFile(
      client,
      JSON.stringify({
        installed: {
          client_id: 'fixture.apps.googleusercontent.com',
          client_secret: 'fixture-secret',
        },
      }),
      { mode: 0o600 },
    );
    let challenge;
    try {
      const result = await authorize({
        provider,
        clientPath: client,
        output,
        onReady: async ({ url, redirectUri }) => {
          const auth = new URL(url);
          challenge = auth.searchParams.get('code_challenge');
          const invalid = new URL(redirectUri);
          invalid.search = new URLSearchParams({ code: 'fixture-code', state: 'wrong-state' });
          assert.equal((await fetch(invalid)).status, 400);
          const callback = new URL(redirectUri);
          callback.search = new URLSearchParams({
            code: 'fixture-code',
            state: auth.searchParams.get('state'),
          });
          assert.equal((await fetch(callback)).status, 200);
        },
        fetchToken: async (url, options) => {
          assert.equal(url, 'https://oauth2.googleapis.com/token');
          assert.equal(options.redirect, 'error');
          assert.equal(options.body.get('grant_type'), 'authorization_code');
          assert.equal(
            createHash('sha256').update(options.body.get('code_verifier')).digest('base64url'),
            challenge,
          );
          return new Response(
            JSON.stringify({
              refresh_token: 'fixture-refresh',
              access_token: 'never-persist',
              token_type: 'Bearer',
              scope: scopes[provider],
            }),
          );
        },
      });
      if (provider === 'drive') {
        const saved = await fs.readFile(output, 'utf8');
        assert.ok(!saved.includes('never-persist'));
        assert.equal(JSON.parse(saved).scope, scopes[provider]);
        assert.equal((await fs.stat(output)).mode & 0o777, 0o600);
      } else {
        assert.equal(result.accessToken, 'never-persist');
        assert.equal(result.scope, scopes[provider]);
        assert.equal(Object.hasOwn(result, 'refresh_token'), false);
        await assert.rejects(fs.access(output));
      }
    } finally {
      await fs.rm(dir, { recursive: true, force: true });
    }
  });
