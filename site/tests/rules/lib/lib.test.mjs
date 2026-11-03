// Unit tests of the helpers: colour maths, route mapping, the stylesheet scanners, a few judges
// on synthetic measurements, and that docs/design/thirty-rules.md matches lib/rules.mjs.
// Run with: npm run unit (no server, no browser).

import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import { contrast, over, parseColor, toHex } from './color.mjs';
import { hoverMotion, isEntranceFrames, keyframes, lightModeRules, parseCss, rasterBackgrounds, uppercaseDeclarations } from './css.mjs';
import { JUDGES, whiteStep } from './judge.mjs';
import { RULES } from './rules.mjs';
import { horizonMatch, pageRoute, routesForChangedFiles, slugFor } from './routes.mjs';
import { REPO_ROOT } from './site.mjs';
import { pixelSize } from './spec.mjs';

const sheet = (css) => [{ source: 'test.css', css, root: parseCss(css) }];

describe('colour', () => {
  it('parses the formats browsers and stylesheets use', () => {
    assert.equal(toHex(parseColor('#0b0f1f')), '#0b0f1f');
    assert.equal(toHex(parseColor('#fff')), '#ffffff');
    assert.deepEqual(parseColor('rgba(255, 255, 255, 0.6)'), { r: 255, g: 255, b: 255, a: 0.6 });
    assert.equal(parseColor('rgb(255 180 84 / 50%)').a, 0.5);
    assert.equal(toHex(parseColor('color(srgb 1 0.705882 0.329412)')), '#ffb454');
    assert.equal(toHex(parseColor('oklch(1 0 0)')), '#ffffff');
    assert.equal(parseColor('transparent').a, 0);
    assert.equal(parseColor('var(--x)'), null);
  });

  it('measures contrast as WCAG does', () => {
    assert.equal(contrast(parseColor('#000'), parseColor('#fff')).toFixed(1), '21.0');
    // The report's measurement: white at 50 % over Horizon's page is 5.3:1.
    const page = parseColor('#100a1f');
    const half = over({ ...parseColor('#fff'), a: 0.5 }, page);
    assert.equal(contrast(half, page).toFixed(1), '5.3');
  });

  it('recognises a white step drawn as a solid grey', () => {
    const page = parseColor('#0b0f1f');
    const sixty = over({ r: 255, g: 255, b: 255, a: 0.6 }, page);
    assert.ok(Math.abs(whiteStep({ ...sixty, a: 1 }) - 0.6) < 0.01);
    assert.equal(whiteStep(parseColor('#ffb454')), null);
  });
});

describe('routes', () => {
  it('maps page and docs files to routes', () => {
    assert.equal(pageRoute('index'), '/');
    assert.equal(pageRoute('board'), '/board/');
    assert.equal(pageRoute('board/[id]'), null);
    assert.equal(pageRoute('404'), '/404');
    const { routes, notes } = routesForChangedFiles([
      'site/src/pages/authors.astro',
      'site/src/content/docs/docs/faq.md',
      'site/src/content/docs/docs/index.md',
      'site/src/pages/board/[id].astro',
      'crates/report/src/tokens.rs',
    ]);
    assert.deepEqual(routes, ['/', '/board/', '/docs/', '/authors/', '/docs/faq/']);
    assert.equal(notes.length, 1);
  });

  it('pairs routes with Horizon captures and names them the same way', () => {
    assert.equal(horizonMatch('/'), 'home');
    assert.equal(horizonMatch('/board/'), 'stats');
    assert.equal(horizonMatch('/docs/faq/', new Set(['docs-faq', 'docs-install'])), 'docs-faq');
    assert.equal(horizonMatch('/docs/tiers/', new Set(['docs-faq', 'docs-install'])), 'docs-install');
    assert.equal(slugFor('/'), 'home');
    assert.equal(slugFor('/docs/faq/'), 'docs-faq');
  });

  it('every route mapped on disk has both Horizon captures', () => {
    const dir = join(REPO_ROOT, 'docs', 'design-research', 'horizon');
    if (!existsSync(dir)) return;
    for (const route of ['/', '/board/', '/docs/', '/authors/', '/roadmap/', '/releases/', '/privacy/', '/404', '/convert/', '/about/']) {
      const slug = horizonMatch(route);
      for (const w of [1440, 390]) assert.ok(existsSync(join(dir, `${slug}-${w}-first.png`)), `${slug}-${w}-first.png for ${route}`);
    }
  });

  it('pixel size follows clamp(round(w / 340), 3, 6)', () => {
    assert.deepEqual([390, 1020, 1440, 1530, 2040, 4000].map(pixelSize), [3, 3, 4, 5, 6, 6]);
  });
});

describe('stylesheet scanners', () => {
  it('finds uppercase, light mode, raster backgrounds and hover motion', () => {
    assert.equal(uppercaseDeclarations(sheet('.a{text-transform:uppercase}.b{text-transform:none}')).length, 1);
    assert.equal(lightModeRules(sheet('@media (prefers-color-scheme: light){a{color:red}} [data-theme=light]{x:y}')).length, 2);
    assert.equal(rasterBackgrounds(sheet('.a{background:url(/x.webp)} .b{background:url(/x.svg)}')).length, 1);
    const hover = hoverMotion(
      sheet(`
      .a:hover{transform:translateY(-1px)}
      .b:hover{color:red}
      @media (prefers-reduced-motion: no-preference){ .c:hover img{scale:1.03} }
      .d:hover{scale:1.03}
      .e{&:hover{box-shadow:0 2px 4px #000}}
    `),
    );
    assert.equal(hover.length, 3, hover.join('\n'));
  });

  it('knows the entrance keyframe', () => {
    assert.ok(isEntranceFrames({ '0%': { opacity: '0', transform: 'translateY(14px)' } }));
    assert.ok(isEntranceFrames({ '0%': { opacity: '0', translate: '0 14px' } }));
    assert.ok(!isEntranceFrames({ '0%': { opacity: '0', transform: 'translateY(8px)' } }));
    const frames = keyframes(sheet('@keyframes rise{0%{opacity:0;transform:translateY(14px)}} @keyframes spin{to{transform:rotate(1turn)}}'));
    assert.deepEqual(frames.map((f) => [f.name, f.entrance]), [['rise', true], ['spin', false]]);
  });
});

describe('judges', () => {
  const base = { route: '/', width: 1440, status: 200 };
  it('rule 11 measures the header', () => {
    const probe = { header: { height: 72, position: 'fixed', bg: 'rgba(0, 0, 0, 0)' }, headerScrolled: { bg: 'rgba(11, 15, 31, 0.75)', backdrop: 'blur(12px)' } };
    assert.deepEqual(JUDGES[11]([{ ...base, probe }]).violations, []);
    const bad = { header: { height: 77, position: 'static', bg: 'rgb(16, 19, 26)' }, headerScrolled: null };
    assert.equal(JUDGES[11]([{ ...base, probe: bad }]).violations.length, 3);
  });

  it('rule 23 wants the hidden "(opens in a new tab)" on outbound text links', () => {
    const link = { protocol: 'https:', internal: false, buttonLike: false, brandMark: false, text: 'GitHub', host: 'github.com', inBody: false };
    const probe = { links: [{ ...link, target: '_blank', srNewTab: true }, { ...link, target: '_blank', srNewTab: false }, { ...link, target: null }] };
    assert.equal(JUDGES[23]([{ ...base, probe }]).violations.length, 2);
  });
});

describe('docs/design/thirty-rules.md', () => {
  const file = join(REPO_ROOT, 'docs', 'design', 'thirty-rules.md');
  it('has the thirty rules, each marked with the checks lib/rules.mjs names', { skip: !existsSync(file) }, () => {
    const lines = readFileSync(file, 'utf8').split(/\r?\n/).filter((l) => /^\d+\. /.test(l));
    assert.equal(lines.length, 30);
    for (const line of lines) {
      const n = Number(line.match(/^(\d+)\./)[1]);
      const rule = RULES[n];
      assert.ok(rule, `rule ${n}`);
      const marker = line.split(' — ').at(-1);
      const automated = [...(marker.match(/automated: ([^;]+)/)?.[1] ?? '').matchAll(/`([^`]+)`/g)].map((m) => m[1]);
      assert.deepEqual(automated, rule.checks, `rule ${n}: automated checks in the doc`);
      assert.equal(/review/.test(marker), Boolean(rule.review), `rule ${n}: review marker`);
    }
  });
});
