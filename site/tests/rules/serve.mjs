// Serves the built site with `wrangler pages dev`, the same runtime Cloudflare Pages uses, with
// local simulations of the KV, D1 and R2 bindings declared in site/wrangler.toml (so /board/
// renders its empty state: local KV holds no board.json).
//
//   node serve.mjs                 serve in the foreground on BASE_URL's port (default 8788)
//   node serve.mjs --background    start it detached, wait until it answers, then exit;
//                                  the log goes to --log (default serve.log), the pid to serve.pid
//
// Environment: SITE_ROOT (the Astro project, default ../..), SITE_DIST (default SITE_ROOT/dist),
// BASE_URL (default http://127.0.0.1:8788).

import { spawn } from 'node:child_process';
import { existsSync, openSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';

import { BASE_URL, SITE_DIST, SITE_ROOT, requireDist, waitForServer } from './lib/site.mjs';

const { values } = parseArgs({
  options: {
    background: { type: 'boolean', default: false },
    log: { type: 'string', default: 'serve.log' },
    pid: { type: 'string', default: 'serve.pid' },
  },
});

requireDist();
const url = new URL(BASE_URL);
const port = url.port || '8788';
const bin = join(SITE_ROOT, 'node_modules', '.bin', process.platform === 'win32' ? 'wrangler.cmd' : 'wrangler');
if (!existsSync(bin)) {
  console.error(`serve: ${bin} not found; run "npm ci" in ${SITE_ROOT} first.`);
  process.exit(1);
}
const args = ['pages', 'dev', SITE_DIST, '--port', port, '--ip', url.hostname, '--log-level', 'warn'];
const env = { ...process.env, WRANGLER_SEND_METRICS: 'false', NO_COLOR: '1' };

if (!values.background) {
  const child = spawn(bin, args, { cwd: SITE_ROOT, stdio: 'inherit', env, shell: process.platform === 'win32' });
  child.on('exit', (code) => process.exit(code ?? 0));
} else {
  const out = openSync(values.log, 'a');
  const child = spawn(bin, args, {
    cwd: SITE_ROOT,
    detached: true,
    stdio: ['ignore', out, out],
    env,
    shell: process.platform === 'win32',
  });
  child.unref();
  writeFileSync(values.pid, String(child.pid));
  try {
    await waitForServer(120_000);
    console.log(`serve: ${BASE_URL} is up (pid ${child.pid}, log ${values.log})`);
    process.exit(0);
  } catch (error) {
    console.error(`serve: ${error.message} See ${values.log}.`);
    process.exit(1);
  }
}
