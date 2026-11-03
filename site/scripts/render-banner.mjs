// Renders the README banner: the sunrise scene as one static frame at 1600x600 with the word
// "Jarrow" over it, through the scene harness. The scene is seeded and drawn on the
// reduced-motion path (one frame, no loop), so the output is the same on every run.
//
//   PLAYWRIGHT_MODULE=<path to playwright's index.mjs> node scripts/render-banner.mjs [out.png]
//
// Playwright is not a dependency of the site (see scripts/scene-harness/shoot.mjs); leave the
// variable unset when `playwright` resolves from here. Writes a PNG (default banner.png in the
// current directory); convert it to WebP separately (docs/media/banner.webp is kept at 60 KB or
// less, for example with Pillow: Image.open(png).save('banner.webp', quality=q, method=6)).

import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { startHarness } from './scene-harness/server.mjs';

const WIDTH = 1600;
const HEIGHT = 600;
const FONT_FILE = new URL(
  '../node_modules/@fontsource-variable/bricolage-grotesque/files/bricolage-grotesque-latin-wght-normal.woff2',
  import.meta.url,
);

const out = resolve(process.argv[2] ?? 'banner.png');

async function loadPlaywright() {
  const spec = process.env.PLAYWRIGHT_MODULE;
  const mod = spec ? await import(pathToFileURL(resolve(spec)).href) : await import('playwright');
  return mod.chromium ?? mod.default.chromium;
}

const fontBase64 = (await readFile(FONT_FILE)).toString('base64');
const chromium = await loadPlaywright();
const harness = await startHarness();
const browser = await chromium.launch();
try {
  const context = await browser.newContext({
    viewport: { width: WIDTH, height: HEIGHT },
    deviceScaleFactor: 1,
    reducedMotion: 'reduce',
  });
  const page = await context.newPage();
  await page.goto(`${harness.url}/scene?name=sunrise&variant=home`);
  await page.waitForSelector('.jw-scene[data-ready]');
  // The canvases fade in over 700 ms; wait that out so the frame is fully opaque.
  await page.waitForTimeout(1000);
  await page.addStyleTag({
    content: `
      @font-face {
        font-family: 'Bricolage Grotesque Banner';
        font-weight: 200 800;
        font-style: normal;
        src: url(data:font/woff2;base64,${fontBase64}) format('woff2');
      }
      .banner-word {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: 'Bricolage Grotesque Banner', sans-serif;
        font-weight: 600;
        font-size: 160px;
        line-height: 1;
        color: #f4f3ff;
        z-index: 2;
      }
    `,
  });
  await page.evaluate(() => {
    const word = document.createElement('div');
    word.className = 'banner-word';
    word.textContent = 'Jarrow';
    document.querySelector('.hero').append(word);
  });
  await page.evaluate(() => document.fonts.ready);
  const loaded = await page.evaluate(() => document.fonts.check("600 160px 'Bricolage Grotesque Banner'"));
  if (!loaded) throw new Error('Bricolage Grotesque did not load');
  await page.screenshot({ path: out, clip: { x: 0, y: 0, width: WIDTH, height: HEIGHT } });
  await context.close();
} finally {
  await browser.close();
  await harness.close();
}
console.log(out);
