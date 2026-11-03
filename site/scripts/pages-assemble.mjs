// Turns Astro's build output into a Cloudflare Pages advanced-mode directory.
//
// @astrojs/cloudflare (v13 and later) builds for Workers: static files in dist/client/ and the
// server worker in dist/server/. The site deploys to a Pages project (D-004), which expects
// static files at the root of pages_build_output_dir and the worker in _worker.js/. This
// script, run after `astro build`, produces:
//
//   dist/                 static files (prerendered pages, /_astro assets, pagefind, _headers)
//   dist/_worker.js/      the Astro server worker; index.js re-exports entry.mjs
//   dist/_routes.json     only the server-rendered routes invoke the worker
//
// Server-rendered routes are the files under src/pages that declare
// `export const prerender = false`. Everything else is served as a static asset by Pages,
// and unknown paths get dist/404.html with status 404.

import { existsSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = fileURLToPath(new URL('..', import.meta.url));
const dist = join(siteRoot, 'dist');
const client = join(dist, 'client');
const server = join(dist, 'server');
const worker = join(dist, '_worker.js');
const pagesDir = join(siteRoot, 'src', 'pages');

function fail(message) {
  console.error(`pages-assemble: ${message}`);
  process.exit(1);
}

/** All files under a directory, recursively. */
function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

const SERVER_RENDERED = /^\s*export\s+const\s+prerender\s*=\s*false\s*;?\s*$/m;
const PAGE_EXTENSIONS = /\.(astro|md|mdx|html)$/;
const ENDPOINT_EXTENSIONS = /\.(ts|js|mjs)$/;

/**
 * Pages route patterns for one file under src/pages. Pages become "/x" and "/x/" (Astro's
 * trailingSlash is "ignore"); endpoints keep their exact path; dynamic segments become "*".
 */
export function routesFor(file) {
  const rel = relative(pagesDir, file).split(sep).join('/');
  const isPage = PAGE_EXTENSIONS.test(rel);
  if (!isPage && !ENDPOINT_EXTENSIONS.test(rel)) return [];
  const segments = rel.replace(isPage ? PAGE_EXTENSIONS : ENDPOINT_EXTENSIONS, '').split('/');
  if (segments.at(-1) === 'index') segments.pop();
  const parts = [];
  for (const segment of segments) {
    if (segment.includes('[')) {
      parts.push('*');
      break;
    }
    parts.push(segment);
  }
  const path = `/${parts.join('/')}`;
  if (path === '/' || path.endsWith('*') || !isPage) return [path];
  return [path, `${path}/`];
}

function serverRoutes() {
  const routes = new Set();
  for (const file of walk(pagesDir)) {
    if (file.includes(`${sep}_`) || /\.test\.[a-z]+$/.test(file)) continue;
    const source = readFileSync(file, 'utf8');
    // Only the frontmatter (or the module itself, for endpoints) can declare prerender.
    const head = file.endsWith('.astro') ? (source.match(/^---\r?\n([\s\S]*?)\r?\n---/)?.[1] ?? '') : source;
    if (SERVER_RENDERED.test(head)) for (const route of routesFor(file)) routes.add(route);
  }
  return [...routes].sort();
}

function main() {
  if (!existsSync(client) || !existsSync(server)) {
    fail('expected dist/client and dist/server from `astro build`; run `npm run build`, not this script alone.');
  }
  if (existsSync(worker)) fail('dist/_worker.js already exists; the build output is not fresh.');

  // 1. The server worker becomes _worker.js/. Its Workers deploy config does not apply to Pages.
  renameSync(server, worker);
  rmSync(join(worker, 'wrangler.json'), { force: true });
  rmSync(join(worker, '.prerender'), { recursive: true, force: true });
  for (const file of walk(worker)) if (file.endsWith('.map')) rmSync(file);
  writeFileSync(join(worker, 'index.js'), "export { default } from './entry.mjs';\n");

  // 2. Static files move to the root of dist/. .assetsignore is a Workers-only file.
  for (const name of readdirSync(client)) {
    if (name === '_worker.js' || name === '_routes.json') fail(`dist/client/${name} would collide with the Pages worker files.`);
    renameSync(join(client, name), join(dist, name));
  }
  rmSync(client, { recursive: true });
  rmSync(join(dist, '.assetsignore'), { force: true });

  // 3. Only server-rendered routes invoke the worker.
  const include = serverRoutes();
  if (include.length === 0) fail('no server-rendered routes found under src/pages.');
  if (include.length > 100) fail(`_routes.json allows 100 rules; found ${include.length}.`);
  const routes = { version: 1, include, exclude: [] };
  writeFileSync(join(dist, '_routes.json'), `${JSON.stringify(routes, null, 2)}\n`);

  // 4. The Cloudflare Vite plugin leaves a deploy redirect to dist/server/wrangler.json for
  // `wrangler deploy` (Workers). Pages uses wrangler.toml, and the redirect would point at a
  // directory that no longer exists, so `wrangler pages dev/deploy` would refuse to start.
  const redirect = join(siteRoot, '.wrangler', 'deploy', 'config.json');
  if (existsSync(redirect)) {
    const target = JSON.parse(readFileSync(redirect, 'utf8')).configPath ?? '';
    if (String(target).split(/[\\/]/).includes('dist')) rmSync(redirect);
  }

  if (!existsSync(join(dist, '404.html'))) fail('dist/404.html is missing; Pages needs it for unknown paths.');
  console.log(`pages-assemble: dist/ is a Pages directory; worker routes: ${include.join(' ')}`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) main();
