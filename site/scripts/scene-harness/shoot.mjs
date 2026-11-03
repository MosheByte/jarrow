// Screenshots of every scene through the harness, at 1440x900 and 390x844.
//
//   PLAYWRIGHT_MODULE=<path to playwright's index.mjs> node scripts/scene-harness/shoot.mjs <out dir>
//
// Playwright is not a dependency of the site: install it anywhere (npm i playwright, then
// npx playwright install chromium) and point PLAYWRIGHT_MODULE at it, or leave the variable
// unset when `playwright` resolves from here. Options: --reduced also captures the static
// reduced-motion frame; --wait <ms> sets how long each scene runs before the capture (1500).

import { mkdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { SCENES, startHarness } from './server.mjs';

const VIEWPORTS = [
  { name: '1440', width: 1440, height: 900, isMobile: false },
  { name: '390', width: 390, height: 844, isMobile: true },
];

const args = process.argv.slice(2);
const outDir = resolve(args.find((a) => !a.startsWith('--') && !/^\d+$/.test(a)) ?? 'scene-shots');
const reduced = args.includes('--reduced');
const waitFlag = args.indexOf('--wait');
const wait = waitFlag >= 0 ? Number(args[waitFlag + 1]) : 1500;

async function loadPlaywright() {
  const spec = process.env.PLAYWRIGHT_MODULE;
  const mod = spec ? await import(pathToFileURL(resolve(spec)).href) : await import('playwright');
  return mod.chromium ?? mod.default.chromium;
}

const chromium = await loadPlaywright();
await mkdir(outDir, { recursive: true });
const harness = await startHarness();
const browser = await chromium.launch();
const written = [];
try {
  for (const [name, scene] of Object.entries(SCENES)) {
    for (const variant of scene.variants.length ? scene.variants : [null]) {
      for (const viewport of VIEWPORTS) {
        for (const motion of reduced ? ['no-preference', 'reduce'] : ['no-preference']) {
          const context = await browser.newContext({
            viewport: { width: viewport.width, height: viewport.height },
            deviceScaleFactor: 1,
            isMobile: viewport.isMobile,
            hasTouch: viewport.isMobile,
            reducedMotion: motion,
          });
          const page = await context.newPage();
          const query = variant ? `?name=${name}&variant=${variant}` : `?name=${name}`;
          await page.goto(`${harness.url}/scene${query}`);
          await page.waitForSelector('.jw-scene[data-ready]');
          await page.waitForTimeout(wait);
          const label = [name, variant && variant !== 'home' ? variant : null, viewport.name, motion === 'reduce' ? 'reduced' : null]
            .filter(Boolean)
            .join('-');
          const file = join(outDir, `${label}.png`);
          await page.screenshot({ path: file });
          written.push(file);
          await context.close();
        }
      }
    }
  }
} finally {
  await browser.close();
  await harness.close();
}
for (const file of written) console.log(file);
