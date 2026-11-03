// Where the site under test is: the built dist/ directory, the site source, and the running
// server. Every path can be overridden with an environment variable, so the workflow can check
// a site built from one commit with the checks of another.

import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

/** site/ (the Astro project). SITE_ROOT overrides it. */
export const SITE_ROOT = resolve(process.env.SITE_ROOT ?? join(here, '..', '..', '..'));

/** The built site. SITE_DIST overrides it; the default is SITE_ROOT/dist. */
export const SITE_DIST = resolve(process.env.SITE_DIST ?? join(SITE_ROOT, 'dist'));

/** The repository root (for docs/design-research/horizon). REPO_ROOT overrides it. */
export const REPO_ROOT = resolve(process.env.REPO_ROOT ?? join(SITE_ROOT, '..'));

/** The served site, for example `wrangler pages dev ./dist`. BASE_URL overrides it. */
export const BASE_URL = (process.env.BASE_URL ?? 'http://127.0.0.1:8788').replace(/\/+$/, '');

export const HORIZON_DIR = join(REPO_ROOT, 'docs', 'design-research', 'horizon');

export function requireDist() {
  if (!existsSync(join(SITE_DIST, '_routes.json')) && !existsSync(join(SITE_DIST, 'index.html'))) {
    throw new Error(`No built site at ${SITE_DIST}. Run "npm run build" in site/ first, or set SITE_DIST.`);
  }
}

/** Fetches a page of the running site; throws a readable error when the server is not up. */
export async function fetchPage(path) {
  const url = `${BASE_URL}${path}`;
  try {
    const res = await fetch(url, { redirect: 'follow' });
    return { url, status: res.status, html: await res.text(), headers: res.headers };
  } catch (error) {
    throw new Error(`Could not fetch ${url} (${error.message}). Start the site first: node serve.mjs, or set BASE_URL.`);
  }
}

/** Waits until the server answers, up to `timeoutMs`. */
export async function waitForServer(timeoutMs = 90_000) {
  const deadline = Date.now() + timeoutMs;
  let last = null;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${BASE_URL}/`);
      if (res.status < 500) return true;
      last = `HTTP ${res.status}`;
    } catch (error) {
      last = error.message;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error(`The site at ${BASE_URL} did not answer within ${timeoutMs / 1000} s (${last}).`);
}
