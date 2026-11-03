// Playwright helpers: one browser, contexts for each viewport, and the measurements that need
// interaction (scrolling, hovering, counting canvas frames, JavaScript off).

import { chromium } from 'playwright';

import { parseColor, sameRgb } from './color.mjs';
import { probePage } from './probe.mjs';
import { MOBILE_UA, SPEC, VIEWPORTS } from './spec.mjs';

/**
 * Launches Chromium. PW_CHANNEL=chrome uses the installed Google Chrome (the CI runners have it,
 * so no browser download); PW_EXECUTABLE points at any Chromium build.
 */
export async function launch() {
  const options = { headless: true, args: ['--hide-scrollbars', '--font-render-hinting=none'] };
  if (process.env.PW_EXECUTABLE) options.executablePath = process.env.PW_EXECUTABLE;
  else if (process.env.PW_CHANNEL) options.channel = process.env.PW_CHANNEL;
  return chromium.launch(options);
}

/** Context options for a viewport width (1440 or 390). 390 is a touch phone, like Horizon's captures. */
export function contextOptions(width, { reducedMotion = 'no-preference', javaScriptEnabled = true, colorScheme = 'dark' } = {}) {
  const vp = VIEWPORTS[width];
  const mobile = width < 768;
  return {
    viewport: { width: vp.width, height: vp.height },
    deviceScaleFactor: 1,
    isMobile: mobile,
    hasTouch: mobile,
    ...(mobile ? { userAgent: MOBILE_UA } : {}),
    reducedMotion,
    javaScriptEnabled,
    // Measured and captured with the system set to dark, the site's only look; rule 1 loads
    // each page once more with the system set to light (measureLightPreference).
    colorScheme,
  };
}

/** Counts canvas draw calls per animation frame, per canvas. Runs before any page script. */
function instrumentCanvas() {
  const q = { frame: 0, next: 1, canvases: new Map() };
  Object.defineProperty(window, '__quality', { value: q });
  const tick = () => {
    q.frame++;
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  const record = (canvas) => {
    if (!canvas) return;
    if (!canvas.__qualityId) Object.defineProperty(canvas, '__qualityId', { value: q.next++ });
    let entry = q.canvases.get(canvas.__qualityId);
    if (!entry) {
      entry = { calls: 0, lastFrame: -1, frameTimes: [] };
      q.canvases.set(canvas.__qualityId, entry);
    }
    entry.calls++;
    if (entry.lastFrame !== q.frame) {
      entry.lastFrame = q.frame;
      entry.frameTimes.push(performance.now());
    }
  };
  const wrap = (proto, names) => {
    for (const name of names) {
      const original = proto[name];
      if (typeof original !== 'function') continue;
      proto[name] = function wrapped(...args) {
        record(this.canvas);
        return original.apply(this, args);
      };
    }
  };
  wrap(CanvasRenderingContext2D.prototype, ['putImageData', 'drawImage', 'fillRect', 'clearRect', 'strokeRect', 'fill', 'stroke', 'fillText', 'strokeText']);
  if (window.ImageBitmapRenderingContext) wrap(ImageBitmapRenderingContext.prototype, ['transferFromImageBitmap']);
  q.window = (from, to) => {
    const out = {};
    for (const [id, e] of q.canvases) out[id] = { calls: e.calls, frames: e.frameTimes.filter((t) => t >= from && t <= to).length };
    return out;
  };
  q.ids = () => [...document.querySelectorAll('canvas')].map((c) => c.__qualityId ?? null);
}

export async function newPage(browser, width, options = {}) {
  const context = await browser.newContext(contextOptions(width, options));
  if (options.javaScriptEnabled !== false) await context.addInitScript(instrumentCanvas);
  const page = await context.newPage();
  return { context, page };
}

/** Loads a route and waits for fonts, scenes and entrance animations to settle. */
export async function load(page, url, { settleMs = 900 } = {}) {
  const res = await page.goto(url, { waitUntil: 'load', timeout: 60_000 });
  try {
    await page.evaluate(() => document.fonts.ready.then(() => true));
  } catch {
    // JavaScript disabled: nothing to wait for.
  }
  await page.waitForTimeout(settleMs);
  return res?.status() ?? 0;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Frames drawn per canvas during `ms`, keyed by the canvas's position in the document. */
async function framesDuring(page, ms) {
  const from = await page.evaluate(() => performance.now());
  await sleep(ms);
  const to = await page.evaluate(() => performance.now());
  return page.evaluate(
    ([a, b]) => {
      const q = window.__quality;
      const counts = q.window(a, b);
      return q.ids().map((id) => (id === null ? { calls: 0, frames: 0 } : counts[id] ?? { calls: 0, frames: 0 }));
    },
    [from, to],
  );
}

/**
 * Everything one page needs at one width: the probe, the header after scrolling, the primary
 * buttons on hover, and the scene frame rate on and off screen.
 */
export async function measure(browser, url, width) {
  const { context, page } = await newPage(browser, width);
  try {
    const status = await load(page, url);
    const probe = await page.evaluate(probePage);
    probe.status = status;

    // Scene frame rate while on screen (rule 16), measured at the top of the page.
    probe.sceneFrames = probe.canvases.length ? await framesDuring(page, 2000) : [];

    // Header after scrolling (rule 11).
    const canScroll = probe.scrollHeight > probe.viewport.h + 600;
    if (probe.header && canScroll) {
      await page.evaluate(() => window.scrollTo(0, 600));
      await sleep(600);
      probe.headerScrolled = await page.evaluate(() => {
        const h = document.querySelector('body > header, body > div > header, header');
        const s = getComputedStyle(h);
        return { bg: s.backgroundColor, backdrop: s.backdropFilter, height: h.getBoundingClientRect().height };
      });
    } else probe.headerScrolled = null;

    // Scenes off screen stop drawing (rule 16): scroll well past the lowest canvas.
    if (probe.canvases.length) {
      const lowest = Math.max(...probe.canvases.map((c) => c.rect.bottom));
      if (probe.scrollHeight - lowest > probe.viewport.h + 100) {
        await page.evaluate((y) => window.scrollTo(0, y), lowest + 100);
        await sleep(800);
        probe.offscreenFrames = await framesDuring(page, 1500);
      } else probe.offscreenFrames = null;
    }
    await page.evaluate(() => window.scrollTo(0, 0));
    await sleep(300);

    // Primary buttons on hover (rule 8): hover each primary-coloured button and read its background.
    probe.hover = [];
    const primary = parseColor(SPEC.colours.primary);
    const primaries = probe.buttons.filter((b) => {
      const c = parseColor(b.bg);
      return c && c.a > 0.99 && sameRgb(c, primary, 2);
    });
    for (const b of primaries.slice(0, 8)) {
      const loc = page.locator(`[data-quality-id="${b.id}"]`);
      try {
        await loc.scrollIntoViewIfNeeded({ timeout: 2000 });
        await loc.hover({ timeout: 2000, force: true });
        await sleep(350);
        probe.hover.push({ id: b.id, bg: await loc.evaluate((e) => getComputedStyle(e).backgroundColor) });
      } catch {
        probe.hover.push({ id: b.id, bg: null });
      }
    }
    await page.mouse.move(1, 1);
    return probe;
  } finally {
    await context.close();
  }
}

/** Under prefers-reduced-motion: reduce, scenes draw one frame and nothing animates (rules 16, 20). */
export async function measureReducedMotion(browser, url, width) {
  const { context, page } = await newPage(browser, width, { reducedMotion: 'reduce' });
  try {
    await load(page, url, { settleMs: 1500 });
    const before = await page.evaluate(() => [...window.__quality.canvases.values()].reduce((n, e) => n + e.calls, 0));
    const frames = await framesDuring(page, 2000);
    const runningAnimations = await page.evaluate(() => document.getAnimations().filter((a) => a.playState === 'running').length);
    const canvases = await page.evaluate(() => document.querySelectorAll('canvas').length);
    return { drewBefore: before, frames, runningAnimations, canvases };
  } finally {
    await context.close();
  }
}

/** With the system set to light, the page is still the dark page (rule 1). */
export async function measureLightPreference(browser, url, width) {
  const { context, page } = await newPage(browser, width, { colorScheme: 'light' });
  try {
    await load(page, url, { settleMs: 300 });
    return await page.evaluate(() => {
      const html = getComputedStyle(document.documentElement);
      const body = getComputedStyle(document.body);
      return { htmlBg: html.backgroundColor, bodyBg: body.backgroundColor, colorScheme: html.colorScheme, dataTheme: document.documentElement.dataset.theme ?? null };
    });
  } finally {
    await context.close();
  }
}

/** With JavaScript off, the page still has its text, navigation and footer (rule 30). */
export async function measureNoJs(browser, url, width) {
  const { context, page } = await newPage(browser, width, { javaScriptEnabled: false });
  try {
    const status = await load(page, url, { settleMs: 200 });
    const read = await page.evaluate(() => ({
      mainTextLength: (document.querySelector('main')?.innerText ?? '').trim().length,
      h1Text: document.querySelector('h1')?.innerText.trim() ?? null,
      navLinks: document.querySelectorAll('header nav a[href], nav[aria-label] a[href]').length,
      footer: !!document.querySelector('footer'),
    }));
    return { status, ...read };
  } finally {
    await context.close();
  }
}
