// Captures the pages a change touches at 1440 x 900 and 390 x 844, named like the Horizon
// captures in docs/design-research/horizon/ (<slug>-<width>-first.png, <slug>-<width>-full.png),
// and writes manifest.json pairing each page with its Horizon screenshot.
//
//   node screenshots.mjs --out shots [--routes /,/board/] [--changed changed-files.txt]
//
// Without --routes, the pages are the always-captured ones (/, /board/, /docs/) plus every page
// the files listed in --changed map to (lib/routes.mjs). The first screen is captured with
// motion on after the entrance animations finish; the full page with prefers-reduced-motion:
// reduce, as the Horizon full-page captures were, so that scroll reveals do not hide content.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';

import { launch, load, newPage } from './lib/browser.mjs';
import { ALWAYS, builtRoutes, horizonMatch, horizonSlugs, requestPath, routesForChangedFiles, slugFor } from './lib/routes.mjs';
import { BASE_URL, HORIZON_DIR, SITE_DIST, SITE_ROOT, requireDist, waitForServer } from './lib/site.mjs';

const { values } = parseArgs({
  options: {
    out: { type: 'string', default: 'shots' },
    routes: { type: 'string' },
    changed: { type: 'string' },
    'settle-ms': { type: 'string', default: '3500' },
  },
});

requireDist();
await waitForServer(60_000);
const known = builtRoutes(SITE_DIST);
let routes;
let notes = [];
if (values.routes) {
  routes = values.routes.split(',').map((r) => r.trim()).filter(Boolean);
} else {
  const files = values.changed && existsSync(values.changed) ? readFileSync(values.changed, 'utf8').split(/\r?\n/).filter(Boolean) : [];
  ({ routes, notes } = routesForChangedFiles(files, { siteRoot: SITE_ROOT, known }));
  for (const r of ALWAYS) if (!routes.includes(r) && known.includes(r)) routes.push(r);
}

mkdirSync(values.out, { recursive: true });
const horizon = horizonSlugs(HORIZON_DIR);
const browser = await launch();
const manifest = { base: BASE_URL, created: new Date().toISOString(), notes, pages: [] };
const settle = Number(values['settle-ms']);

try {
  for (const route of routes) {
    const slug = slugFor(route);
    const entry = { route, slug, horizon: horizonMatch(route, horizon.size ? horizon : null), status: null, files: {} };
    for (const width of [1440, 390]) {
      const url = `${BASE_URL}${requestPath(route)}`;
      {
        const { context, page } = await newPage(browser, width);
        entry.status = await load(page, url, { settleMs: settle });
        const file = `${slug}-${width}-first.png`;
        await page.screenshot({ path: join(values.out, file) });
        entry.files[`${width}-first`] = file;
        await context.close();
      }
      {
        const { context, page } = await newPage(browser, width, { reducedMotion: 'reduce' });
        await load(page, url, { settleMs: 1500 });
        const file = `${slug}-${width}-full.png`;
        await page.screenshot({ path: join(values.out, file), fullPage: true });
        entry.files[`${width}-full`] = file;
        await context.close();
      }
    }
    manifest.pages.push(entry);
    console.log(`screenshots: ${route} -> ${slug}-{1440,390}-{first,full}.png (Horizon: ${entry.horizon}, HTTP ${entry.status})`);
  }
} finally {
  await browser.close();
}
for (const note of notes) console.log(`screenshots: note: ${note}`);
writeFileSync(join(values.out, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
