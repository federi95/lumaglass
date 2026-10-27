// CI-only execution in this session. No TV services or privileged processes.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { setTimeout as delay } from 'node:timers/promises';

const root = path.resolve(import.meta.dirname, '..');
const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
const temporary = await fs.mkdtemp(path.join(root, '.build/display-watchdog-'));
const preferences = path.join(temporary, 'preferences.json');
const request = path.join(temporary, 'preferences.display-request.json');
const fallback = path.join(temporary, 'preferences.display-1080p');
const dump = path.join(temporary, 'recovered.json');
const logPath = path.join(temporary, 'recovery.log');
const log = await fs.open(logPath, 'a');
const token = `ci-timeout-${process.pid}`;
let parent;
let deadline;
try {
  await fs.writeFile(
    preferences,
    JSON.stringify({ display: { four_k: false }, show_weather: false }),
  );
  await fs.writeFile(request, JSON.stringify({ token, staged: false, trial: true }));
  // Only this disposable parent can be signalled by the native watchdog.
  // The test controller remains a separate process and observes recovery.
  parent = spawn(
    process.execPath,
    [
      '-e',
      `
    const {spawn}=require('node:child_process');
    const data=JSON.parse(process.argv[1]);
    const watchdog=spawn(data.binary,['--display-watchdog',data.preferences,data.token,'--',
      '--preferences',data.preferences,'--frames','2','--dump-state',data.dump],{stdio:'inherit'});
    watchdog.once('error',()=>process.exit(2));
    setInterval(()=>{},1000);
  `,
      JSON.stringify({ binary, preferences, token, dump }),
    ],
    {
      cwd: root,
      stdio: ['ignore', log.fd, log.fd],
      env: { ...process.env, LIBGL_ALWAYS_SOFTWARE: '1' },
    },
  );
  const started = performance.now();
  const ended = await Promise.race([
    once(parent, 'exit'),
    new Promise((_, reject) => {
      deadline = setTimeout(
        () => reject(Error('Watchdog timeout did not end the disposable parent')),
        30000,
      );
    }),
  ]);
  clearTimeout(deadline);
  assert.ok(
    performance.now() - started >= 14000,
    'The watchdog must allow the confirmation interval',
  );
  assert.ok(
    ['SIGTERM', 'SIGKILL'].includes(ended[1]),
    'Only the disposable parent should be terminated',
  );
  const limit = performance.now() + 15000;
  let recovered;
  while (performance.now() < limit) {
    try {
      recovered = JSON.parse(await fs.readFile(dump, 'utf8'));
      break;
    } catch (error) {
      if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
    }
    await delay(100);
  }
  assert.ok(recovered, 'The watchdog must restart the launcher after claiming an expired trial');
  assert.equal(recovered.preferences.display.four_k, false);
  assert.equal(recovered.webos, false);
  assert.equal((await fs.readFile(fallback, 'utf8')).trim(), '1080p');
  await assert.rejects(fs.access(request), { code: 'ENOENT' });
  console.log('Display watchdog: timed parent termination and safe desktop restart verified.');
} catch (error) {
  console.error((await fs.readFile(logPath, 'utf8')).slice(-6000));
  throw error;
} finally {
  clearTimeout(deadline);
  await fs.rm(request, { force: true });
  if (parent && parent.exitCode === null && parent.signalCode === null) parent.kill('SIGKILL');
  await log.close();
  await fs.rm(temporary, { recursive: true, force: true });
}
