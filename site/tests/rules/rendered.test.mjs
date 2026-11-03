// Rendered checks: every built page at 1440 x 900 and 390 x 844 in Chromium, measured through
// computed styles (lib/probe.mjs) and interaction (lib/browser.mjs), judged per rule
// (lib/judge.mjs). Needs the site served at BASE_URL (node serve.mjs --background).
//
// RULE_ROUTES=/,/board/ limits the pages; RULES_OUT sets where rendered.json is written.

import assert from 'node:assert/strict';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, describe, it } from 'node:test';

import { launch, measure, measureLightPreference, measureNoJs, measureReducedMotion } from './lib/browser.mjs';
import { keyframes, collectShippedCss } from './lib/css.mjs';
import { JUDGES } from './lib/judge.mjs';
import { RULES } from './lib/rules.mjs';
import { builtRoutes, requestPath } from './lib/routes.mjs';
import { BASE_URL, SITE_DIST, SITE_ROOT, fetchPage, requireDist, waitForServer } from './lib/site.mjs';
import { SPEC } from './lib/spec.mjs';

requireDist();
await waitForServer(30_000);
const OUT = process.env.RULES_OUT ?? 'out';
const routes = process.env.RULE_ROUTES ? process.env.RULE_ROUTES.split(',').map((r) => r.trim()).filter(Boolean) : builtRoutes(SITE_DIST);
const widths = [1440, 390];

// The entrance keyframe's name, read from the shipped CSS (rule 20).
const pages = [];
for (const route of routes) pages.push({ route, ...(await fetchPage(requestPath(route))) });
const sheets = await collectShippedCss({ dist: SITE_DIST, pages, fetchText: async (href) => (await fetch(`${BASE_URL}${href}`)).text() });
const entranceNames = [...new Set(keyframes(sheets, SPEC.entrance.translatePx).filter((k) => k.entrance).map((k) => k.name))];

// The brand sources on record (rule 22).
const brandFile = join(SITE_ROOT, 'BRAND-SOURCES.md');
const brandSources = existsSync(brandFile) ? [...readFileSync(brandFile, 'utf8').matchAll(/https:\/\/[^\s)>|`]+/g)].map((m) => m[0].replace(/[.,;]+$/, '')) : null;

const browser = await launch();
after(() => browser.close());

const jobs = routes.flatMap((route) => widths.map((width) => ({ route, width })));
const samples = [];
const errors = [];
async function worker() {
  while (jobs.length) {
    const { route, width } = jobs.shift();
    const url = `${BASE_URL}${requestPath(route)}`;
    try {
      const probe = await measure(browser, url, width);
      const animated = probe.canvases.length > 0 || probe.animations.length > 0;
      const reduced = animated ? await measureReducedMotion(browser, url, width) : null;
      const nojs = await measureNoJs(browser, url, width);
      const light = await measureLightPreference(browser, url, width);
      samples.push({ route, width, status: probe.status, probe, reduced, nojs, light });
    } catch (error) {
      errors.push(`${route} at ${width}: ${error.message.split('\n')[0]}`);
    }
  }
}
await Promise.all(Array.from({ length: Number(process.env.RULES_CONCURRENCY ?? 4) }, worker));
samples.sort((a, b) => routes.indexOf(a.route) - routes.indexOf(b.route) || b.width - a.width);

const report = {};
const show = (list, max = 30) => `\n  - ${list.slice(0, max).join('\n  - ')}${list.length > max ? `\n  - ... and ${list.length - max} more` : ''}`;

describe('rendered pages', () => {
  it('every page loaded at both widths', () => {
    assert.equal(errors.length, 0, `Pages that could not be measured:${show(errors)}`);
  });

  for (const [n, judge] of Object.entries(JUDGES)) {
    const id = `rule-${String(n).padStart(2, '0')}`;
    it(`${id}: ${RULES[n].title}`, (t) => {
      const { violations, checked, note } = judge(samples, { entranceNames, brandSources });
      report[id] = { rule: Number(n), checked, violations, ...(note ? { note } : {}) };
      if (checked === 0 && violations.length === 0) {
        report[id].skipped = 'nothing on the built pages this rule applies to';
        t.skip('nothing on the built pages this rule applies to');
        return;
      }
      assert.equal(violations.length, 0, `${violations.length} violations:${show(violations)}`);
    });
  }
});

process.on('exit', () => {
  mkdirSync(OUT, { recursive: true });
  const slim = samples.map(({ route, width, status, probe, reduced, nojs }) => ({
    route,
    width,
    status,
    header: probe.header,
    headerScrolled: probe.headerScrolled,
    h1: probe.headings.filter((h) => h.tag === 'h1').map(({ text, fontSize, lineHeight, letterSpacing, lines, fontFamily }) => ({ text, fontSize, lineHeight, letterSpacing, lines, fontFamily })),
    canvases: probe.canvases.length,
    sceneFrames: probe.sceneFrames,
    offscreenFrames: probe.offscreenFrames ?? null,
    reduced,
    nojs,
  }));
  writeFileSync(join(OUT, 'rendered.json'), `${JSON.stringify({ base: BASE_URL, routes, entranceNames, errors, report, samples: slim }, null, 2)}\n`);
});
