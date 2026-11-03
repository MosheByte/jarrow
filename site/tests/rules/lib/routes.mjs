// Which pages to capture and check, and which Horizon screenshot each one is compared with.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';

/** Captured and checked on every run, whatever changed. */
export const ALWAYS = ['/', '/board/', '/docs/'];

/**
 * Jarrow route -> the Horizon page it is compared with (docs/design-research/horizon/<slug>-*.png).
 * The pairs follow the report's inventory: the board is Horizon's live players page (the ember
 * field hero), authors and the roadmap are the hub (the aurora band), the 404 page is Horizon's
 * 404 (captured at /players). Docs articles use the Horizon docs page of the same name where one
 * exists, else /docs/install, a typical docs article.
 */
const EXACT = {
  '/': 'home',
  '/board/': 'stats',
  '/convert/': 'benchmarks',
  '/directions/': 'mods',
  '/authors/': 'hub',
  '/roadmap/': 'hub',
  '/contributors/': 'hub',
  '/releases/': 'releases',
  '/status/': 'stats',
  '/privacy/': 'privacy',
  '/about/': 'artists',
  '/docs/': 'docs',
  '/404': 'players',
};

/** The Horizon slug a Jarrow route is compared with. `available` lists the Horizon slugs on disk. */
export function horizonMatch(route, available = null) {
  const has = (slug) => available === null || available.has(slug);
  if (EXACT[route] && has(EXACT[route])) return EXACT[route];
  if (route.startsWith('/board/')) return has('mods-jei') ? 'mods-jei' : 'stats';
  if (route.startsWith('/docs/')) {
    const slug = `docs-${route.slice('/docs/'.length).replace(/\/+$/, '').replaceAll('/', '-')}`;
    if (has(slug)) return slug;
    return 'docs-install';
  }
  // Short pages that say what is not built yet read like Horizon's short prose pages.
  return 'artists';
}

/** File-name slug for a route, in Horizon's naming: "/" is "home", "/docs/faq/" is "docs-faq". */
export function slugFor(route) {
  if (route === '/') return 'home';
  return route.replace(/^\/+|\/+$/g, '').replaceAll('/', '-') || 'home';
}

/**
 * Horizon slugs that have both a 1440 and a 390 first-screen capture in `dir`. When the folder
 * is not checked out, HORIZON_LIST may name a file listing its file names (git ls-tree output).
 */
export function horizonSlugs(dir) {
  let list;
  if (existsSync(dir)) list = readdirSync(dir);
  else if (process.env.HORIZON_LIST && existsSync(process.env.HORIZON_LIST)) list = readFileSync(process.env.HORIZON_LIST, 'utf8').split(/\r?\n/).map((l) => l.split('/').pop());
  else return new Set();
  const names = new Set(list);
  const slugs = new Set();
  for (const name of names) {
    const m = name.match(/^(.+)-1440-first\.png$/);
    if (m && names.has(`${m[1]}-390-first.png`)) slugs.add(m[1]);
  }
  return slugs;
}

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

/**
 * Every page route of a built site: the prerendered HTML files in dist/ plus the server-rendered
 * page routes listed in dist/_routes.json (endpoints and wildcards left out).
 */
export function builtRoutes(dist) {
  const routes = new Set();
  for (const file of walk(dist)) {
    const rel = relative(dist, file).split(sep).join('/');
    if (!rel.endsWith('.html')) continue;
    if (/^(_worker\.js|_astro|pagefind)\//.test(rel)) continue;
    if (rel === '404.html') {
      routes.add('/404');
      continue;
    }
    if (rel === 'index.html') routes.add('/');
    else if (rel.endsWith('/index.html')) routes.add(`/${rel.slice(0, -'index.html'.length)}`);
    else routes.add(`/${rel.slice(0, -'.html'.length)}/`);
  }
  const routesFile = join(dist, '_routes.json');
  if (existsSync(routesFile)) {
    const { include = [] } = JSON.parse(readFileSync(routesFile, 'utf8'));
    for (const pattern of include) {
      if (pattern.includes('*') || pattern.startsWith('/api/') || /\.[a-z0-9]+$/i.test(pattern)) continue;
      routes.add(pattern === '/' || pattern.endsWith('/') ? pattern : `${pattern}/`);
    }
  }
  return [...routes].sort(routeOrder);
}

/** "/" first, then the always-captured pages, then the rest alphabetically. */
export function routeOrder(a, b) {
  const rank = (r) => (ALWAYS.includes(r) ? ALWAYS.indexOf(r) : ALWAYS.length);
  return rank(a) - rank(b) || a.localeCompare(b);
}

/** The URL path to request for a route: the 404 page is whatever an unknown path returns. */
export function requestPath(route) {
  return route === '/404' ? '/this-page-does-not-exist/' : route;
}

/**
 * Pages a change touches. `files` are repository paths. Page and docs files map to their route;
 * a component, layout, style or script maps to the pages under src/pages that import it
 * directly (read from `siteRoot`); anything else maps to the always-captured pages only.
 * Returns { routes, notes } where notes explain files that map to no single page.
 */
export function routesForChangedFiles(files, { siteRoot = null, known = null } = {}) {
  const routes = new Set(ALWAYS);
  const notes = [];
  const pageSources = siteRoot ? pageSourceIndex(siteRoot) : [];
  for (const file of files) {
    const f = file.replaceAll('\\', '/');
    let m = f.match(/^site\/src\/pages\/(.+)\.(astro|md|mdx|html)$/);
    if (m) {
      const route = pageRoute(m[1]);
      if (route) routes.add(route);
      else notes.push(`${f}: a dynamic route; capture it by hand with a real parameter`);
      continue;
    }
    m = f.match(/^site\/src\/content\/docs\/(.+)\.(md|mdx)$/);
    if (m) {
      let path = m[1];
      if (path === 'index' || path.endsWith('/index')) path = path.replace(/\/?index$/, '');
      routes.add(path ? `/${path}/` : '/');
      continue;
    }
    m = f.match(/^site\/src\/(components|layouts|styles|lib|scripts|scenes)\/(.+)\.[a-z]+$/);
    if (m && pageSources.length) {
      const base = m[2].split('/').pop();
      const users = pageSources.filter(({ source }) => importsName(source, base));
      for (const { route } of users) routes.add(route);
    }
  }
  let list = [...routes];
  if (known) {
    const knownSet = new Set(known);
    for (const r of list) if (!knownSet.has(r)) notes.push(`${r}: changed, but the build has no such page`);
    list = list.filter((r) => knownSet.has(r));
  }
  return { routes: list.sort(routeOrder), notes };
}

function importsName(source, base) {
  const escaped = base.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`(?:import|from)\\s[^;\\n]*['"][^'"]*/${escaped}(?:\\.[a-z]+)?['"]`).test(source);
}

/** Route for a file under src/pages (without extension), or null for a dynamic route. */
export function pageRoute(path) {
  if (path.includes('[')) return null;
  if (path === 'index') return '/';
  if (path === '404') return '/404';
  if (path.startsWith('api/')) return null;
  const clean = path.replace(/\/index$/, '');
  return `/${clean}/`;
}

function pageSourceIndex(siteRoot) {
  const pagesDir = join(siteRoot, 'src', 'pages');
  if (!existsSync(pagesDir)) return [];
  return walk(pagesDir)
    .filter((file) => /\.(astro|md|mdx)$/.test(file))
    .map((file) => {
      const rel = relative(pagesDir, file).split(sep).join('/').replace(/\.(astro|md|mdx)$/, '');
      return { route: pageRoute(rel), source: readFileSync(file, 'utf8') };
    })
    .filter(({ route }) => route);
}
