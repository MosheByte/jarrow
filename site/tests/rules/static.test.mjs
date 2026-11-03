// Stylesheet checks over the CSS the built site ships (every linked and inline stylesheet of
// every page, plus dist/_astro/*.css), and the token-pair contrast check.
// Needs the site served at BASE_URL (node serve.mjs --background) and built at SITE_DIST.

import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import { contrast, over, parseColor, sameRgb, toHex } from './lib/color.mjs';
import {
  collectShippedCss,
  colourUses,
  coloursOutsideTokens,
  findTokensFile,
  entranceUses,
  hoverMotion,
  keyframes,
  lightModeRules,
  rasterBackgrounds,
  reducedMotionOff,
  tokenColours,
  uppercaseDeclarations,
  viewTimelines,
} from './lib/css.mjs';
import { builtRoutes, requestPath } from './lib/routes.mjs';
import { BASE_URL, SITE_DIST, SITE_ROOT, fetchPage, requireDist } from './lib/site.mjs';
import { SPEC } from './lib/spec.mjs';

requireDist();
const OUT = process.env.RULES_OUT ?? 'out';
const routes = builtRoutes(SITE_DIST);
const pages = [];
for (const route of routes) {
  const { html, status } = await fetchPage(requestPath(route));
  pages.push({ route, html, status });
}
const sheets = await collectShippedCss({
  dist: SITE_DIST,
  pages,
  fetchText: async (href) => (await fetch(`${BASE_URL}${href}`)).text(),
});

const report = {};
const record = (id, violations, extra = {}) => {
  report[id] = { violations, ...extra };
  return violations;
};
const show = (list, max = 25) => `\n  - ${list.slice(0, max).join('\n  - ')}${list.length > max ? `\n  - ... and ${list.length - max} more` : ''}`;

describe('shipped CSS', () => {
  it('css-no-uppercase (rule 6): no text-transform uppercase or capitalize, no all-caps variants', () => {
    const found = record('css-no-uppercase', uppercaseDeclarations(sheets));
    assert.equal(found.length, 0, `Uppercase in the shipped CSS:${show(found)}`);
  });

  it('css-no-light-mode (rule 1): no prefers-color-scheme query, light theme or light color-scheme', () => {
    const found = record('css-no-light-mode', lightModeRules(sheets));
    assert.equal(found.length, 0, `Light mode in the shipped CSS:${show(found)}`);
  });

  it('css-one-entrance-keyframe (rule 20): one 14 px rise over 0.9 s, bound to view() below the fold, off under reduced motion', () => {
    const frames = keyframes(sheets, SPEC.entrance.translatePx);
    const names = [...new Set(frames.map((f) => f.name))];
    const entrance = [...new Set(frames.filter((f) => f.entrance).map((f) => f.name))];
    const problems = frames.filter((f) => !f.entrance).map((f) => `@keyframes ${f.name} (${f.animates.join(', ')}) at ${f.source} is not the entrance keyframe`);
    if (entrance.length !== 1) problems.push(`${entrance.length} entrance keyframes (${entrance.join(', ') || 'none'}) from opacity 0 and translateY(14px); there must be exactly one`);
    const uses = entranceUses(sheets, entrance);
    if (entrance.length === 1) {
      if (!uses.some((u) => /\b0?\.9s\b|\b900ms\b/.test(u.value))) problems.push(`no rule runs "${entrance[0]}" over 0.9 s`);
      if (!uses.some((u) => /cubic-bezier\(\s*0?\.2\s*,\s*0?\.7\s*,\s*0?\.2\s*,\s*1\s*\)/.test(u.value))) problems.push(`no rule runs "${entrance[0]}" with cubic-bezier(0.2, 0.7, 0.2, 1)`);
      if (!viewTimelines(sheets)) problems.push('no animation-timeline: view() for the sections below the fold');
    }
    if (!reducedMotionOff(sheets)) problems.push('no @media (prefers-reduced-motion: reduce) rule that sets animation: none');
    record('css-one-entrance-keyframe', problems, { entrance, names });
    assert.equal(problems.length, 0, `Entrance motion:${show(problems)}`);
  });

  it('css-hover-colour-only (rule 21): :hover changes colour, not position, size or shadow', () => {
    const found = record('css-hover-colour-only', hoverMotion(sheets));
    assert.equal(found.length, 0, `Hover rules that move, scale or shadow:${show(found)}`);
  });

  it('css-no-raster-decoration (rule 18): no raster image as a CSS background', () => {
    const found = record('css-no-raster-decoration', rasterBackgrounds(sheets));
    assert.equal(found.length, 0, `Raster backgrounds:${show(found)}`);
  });

  it('css-colour-tokens: the shipped CSS uses no colour outside the token file', (t) => {
    const tokens = findTokensFile(SITE_ROOT, SPEC.colours.page);
    if (!tokens) {
      record('css-colour-tokens', [], { skipped: 'no token file with the night-blue page colour (site/tokens.css) yet' });
      t.skip('no token file with the night-blue page colour yet (site/tokens.css, or src/styles/tokens.css carrying #0b0f1f)');
      return;
    }
    const colours = tokenColours(tokens.css);
    const missing = Object.entries(SPEC.colours)
      .flatMap(([name, v]) => (Array.isArray(v) ? v.map((x, i) => [`${name}[${i}]`, x]) : [[name, v]]))
      .filter(([, hex]) => ![...colours.values()].some((c) => sameRgb(c, parseColor(hex), 0.6)))
      .map(([name, hex]) => `${name} ${hex} is not a token in ${tokens.file}`);
    const outside = coloursOutsideTokens(colourUses(sheets), colours);
    const problems = [...missing, ...outside];
    record('css-colour-tokens', problems, { tokens: tokens.file });
    assert.equal(problems.length, 0, `Colours outside the tokens:${show(problems)}`);
  });
});

describe('token pairs', () => {
  it('token-pairs-contrast (rule 29): every text colour of the palette is 4.9:1 or more on the page and on a card', () => {
    const c = Object.fromEntries(Object.entries(SPEC.colours).map(([k, v]) => [k, Array.isArray(v) ? v.map(parseColor) : parseColor(v)]));
    const page = c.page;
    // The hero card's surface in the report: white at 4 % over the page.
    const card = over({ ...c.white, a: 0.04 }, page);
    const pairs = [];
    const add = (label, fg, bg, bgLabel) => pairs.push({ label: `${label} on ${bgLabel}`, ratio: contrast(over(fg, bg), bg), fg: toHex(over(fg, bg)), bg: toHex(bg) });
    for (const [bg, bgLabel] of [[page, 'page'], [card, 'card (white 4 %)']]) {
      add('foreground', c.foreground, bg, bgLabel);
      for (const a of SPEC.textAlphas) add(`white ${Math.round(a * 100)} %`, { ...c.white, a }, bg, bgLabel);
      add('accent', c.accent, bg, bgLabel);
      c.gradient.forEach((g, i) => add(`gradient stop ${i + 1}`, g, bg, bgLabel));
      add('tier green', c.tierGreen, bg, bgLabel);
      add('tier amber', c.tierAmber, bg, bgLabel);
      add('tier red', c.tierRed, bg, bgLabel);
    }
    add('primary button text', c.primaryText, c.primary, 'primary');
    add('primary button text', c.primaryText, c.primaryHover, 'primary hover');
    add('page colour', c.page, c.white, 'a white button');
    const low = pairs.filter((p) => p.ratio < SPEC.contrastMin).map((p) => `${p.label}: ${p.fg} on ${p.bg} is ${p.ratio.toFixed(2)}:1`);
    record('token-pairs-contrast', low, { pairs: pairs.map((p) => ({ ...p, ratio: Math.round(p.ratio * 100) / 100 })) });
    assert.equal(low.length, 0, `Token pairs under 4.9:1:${show(low)}`);
  });
});

process.on('exit', () => {
  mkdirSync(OUT, { recursive: true });
  writeFileSync(join(OUT, 'static.json'), `${JSON.stringify({ routes, sheets: sheets.map((s) => s.source), report }, null, 2)}\n`);
});
