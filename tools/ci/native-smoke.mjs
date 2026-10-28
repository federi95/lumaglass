// Only CI opts into the exact binary artifact produced by its own prepare job.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { root } from '../lib/project.mjs';

export function buildNative() {
  if (process.env.LUMAGLASS_SMOKE_PREBUILT === '1') {
    const binary = path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/lumaglass');
    fs.accessSync(binary, fs.constants.X_OK);
    return;
  }
  const result = spawnSync('cargo', ['build', '--locked', '--package', 'lumaglass'], {
    cwd: root,
    stdio: 'inherit',
    timeout: 120000,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Native build failed (${result.status}).`);
}
