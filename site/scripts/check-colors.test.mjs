import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { after, describe, it } from 'node:test';

import {
  astroTemplates,
  checkDist,
  classify,
  colorsIn,
  htmlSources,
  palette,
  parseCss,
  splitTopLevel,
} from './check-colors.mjs';
import { parseColor, readTokens, sameColor } from './tokens.mjs';

const tokens = readTokens();
const entries = palette(tokens);
const statusOf = (value, property = 'color') =>
  colorsIn(value, property).map((c) => classify(c, tokens, entries).status);

describe('parseColor', () => {
  it('reads every hex length', () => {
    assert.deepEqual(parseColor('#fff'), { r: 255, g: 255, b: 255, a: 1 });
    assert.deepEqual(parseColor('#0b0f1f'), { r: 11, g: 15, b: 31, a: 1 });
    assert.equal(parseColor('#ffffff99').a, 0x99 / 255);
    assert.equal(parseColor('#0000').a, 0);
  });

  it('reads rgb() and hsl() in both syntaxes, and named colours', () => {
    assert.ok(sameColor(parseColor('rgb(255 255 255 / 0.6)'), parseColor('#fff9')));
    assert.ok(sameColor(parseColor('rgba(255, 180, 84, 0.45)'), parseColor('rgb(255 180 84 / 45%)')));
    assert.ok(sameColor(parseColor('hsl(0, 0%, 100%)'), parseColor('white')));
    assert.deepEqual(parseColor('red'), { r: 255, g: 0, b: 0, a: 1 });
    assert.equal(parseColor('transparent').a, 0);
  });

  it('refuses what it cannot resolve', () => {
    assert.equal(parseColor('hsl(var(--sl-hue-orange), 82%, 63%)'), null);
    assert.equal(parseColor('oklch(0.7 0.1 50)'), null);
    assert.equal(parseColor('#12345'), null);
  });
});

describe('the palette', () => {
  it('holds the tokens and their declared alpha steps', () => {
    assert.deepEqual(statusOf('#0b0f1f'), ['token']);
    assert.deepEqual(statusOf('#ffb454'), ['token']);
    assert.deepEqual(statusOf('#fff'), ['token']);
    assert.deepEqual(statusOf('rgb(255 255 255 / 0.6)'), ['alpha']);
    assert.deepEqual(statusOf('#ffffff0a'), ['alpha'], 'white at 4 % after minification');
    assert.deepEqual(statusOf('#ffffffa6'), ['alpha'], 'white at 65 % rounds to 0xa6');
    assert.deepEqual(statusOf('rgb(11 15 31 / 0.75)'), ['alpha']);
  });

  it('rejects colours and alphas tokens.css does not declare', () => {
    assert.deepEqual(statusOf('#0b0d12'), ['foreign']);
    assert.deepEqual(statusOf('#ff5ca8'), ['foreign']);
    assert.deepEqual(statusOf('rgb(255 255 255 / 0.37)'), ['foreign']);
    assert.deepEqual(statusOf('#000'), ['foreign']);
  });

  it('treats every fully transparent value as transparent, and the keywords as no colour', () => {
    assert.deepEqual(statusOf('#0000'), ['transparent']);
    assert.deepEqual(statusOf('rgba(0, 0, 0, 0)'), ['transparent']);
    assert.deepEqual(statusOf('transparent'), []);
    assert.deepEqual(statusOf('currentColor'), []);
    assert.deepEqual(statusOf('inherit'), []);
  });
});

describe('colorsIn', () => {
  it('finds hex, functions and named colours in a value', () => {
    const found = colorsIn('0 1px 2px #000, 0 0 0 1px rgb(1 2 3), inset 0 0 red', 'box-shadow');
    assert.deepEqual(
      found.map((f) => f.text),
      ['rgb(1 2 3)', '#000', 'red'],
    );
  });

  it('does not read names out of variables, strings, urls or non-colour properties', () => {
    assert.deepEqual(colorsIn('var(--jw-tier-red)', 'color'), []);
    assert.deepEqual(colorsIn("'Arial Black', sans-serif", 'font-family'), []);
    assert.deepEqual(colorsIn('Arial Black', 'font-family'), []);
    assert.deepEqual(colorsIn('url(#paint) red', 'mask-image'), [], 'masks never paint');
    assert.deepEqual(colorsIn('url(/a.png#ff0000)', 'background'), []);
    assert.deepEqual(colorsIn('"#ff0000"', 'content'), []);
  });

  it('reads the fills of an SVG data URI', () => {
    const svg = "url(\"data:image/svg+xml,%3Csvg fill='%23ff0000'%3E%3C/svg%3E\")";
    assert.deepEqual(statusOf(svg, 'background-image'), ['foreign']);
  });
});

describe('color-mix()', () => {
  it('passes a token at a declared alpha step', () => {
    assert.deepEqual(statusOf('color-mix(in oklab, var(--jw-white) 60%, transparent)'), ['alpha']);
    assert.deepEqual(statusOf('color-mix(in srgb, transparent, var(--jw-page) 75%)'), ['alpha']);
    assert.deepEqual(statusOf('color-mix(in oklab, currentcolor 50%, transparent)'), ['alpha']);
  });

  it('fails an alpha step tokens.css does not declare, and a mix of two colours', () => {
    assert.deepEqual(statusOf('color-mix(in oklab, var(--jw-white) 37%, transparent)'), ['foreign']);
    assert.deepEqual(statusOf('color-mix(in srgb, var(--jw-accent), var(--jw-page))'), ['foreign']);
    assert.deepEqual(statusOf('color-mix(in srgb, #123456 50%, transparent)'), ['foreign']);
  });

  it('checks the literal fallback of a variable', () => {
    assert.deepEqual(statusOf('color-mix(in srgb, var(--x, #123456) 50%, transparent)'), ['foreign']);
  });
});

describe('parseCss', () => {
  it('records selectors, layers and at-rules, and skips @supports preludes', () => {
    const decls = parseCss(
      '@layer base{:root{--a:#fff}}@supports (color: color-mix(in lab, red, red)){.x{color:#000}}' +
        '@media print{:root{--b:url("data:x;y")}}.y{background:#0b0f1f;}',
    );
    assert.deepEqual(
      decls.map((d) => [d.selector, d.property, d.layered, d.atRules.length]),
      [
        [':root', '--a', true, 1],
        ['.x', 'color', false, 1],
        [':root', '--b', false, 1],
        ['.y', 'background', false, 0],
      ],
    );
    assert.equal(decls[2].value, 'url("data:x;y")');
  });

  it('splits at top-level commas only', () => {
    assert.deepEqual(splitTopLevel(':root, :is(a, b), ::backdrop'), [':root', ':is(a, b)', '::backdrop']);
  });
});

describe('htmlSources', () => {
  it('finds stylesheets in order, styles, style attributes, colour attributes and theme-color', () => {
    const s = htmlSources(
      '<link rel="stylesheet" href="/a.css"><meta name="theme-color" content="#0b0f1f">' +
        '<link rel="preload" href="/f.woff2"><link href="/b.css" rel="stylesheet" media="print">' +
        '<style>.x{color:red}</style><p style="color: #123">x</p><svg><path fill="#fff" stroke="currentColor"/></svg>' +
        '<script>const s = "<p style=\'color:#bad\'>";</script>',
    );
    assert.deepEqual(s.stylesheets, ['/a.css', '/b.css']);
    assert.deepEqual(s.styles, ['.x{color:red}']);
    assert.deepEqual(
      s.declarations.map((d) => d.value),
      ['#123'],
    );
    assert.deepEqual(
      s.attributes.map((a) => [a.where, a.value]),
      [
        ['meta theme-color', '#0b0f1f'],
        ['<path fill>', '#fff'],
        ['<path stroke>', 'currentColor'],
      ],
    );
  });
});

describe('astroTemplates', () => {
  it('reads the literal parts of renderTemplate tagged templates only', () => {
    const js =
      'const a = `<style>:root{--x:#bad}</style>`;\n' +
      'return renderTemplate`<p style="color:#0b0f1f">${x ? renderTemplate`<i fill="#fff">` : `${"}"}`}</p>`;';
    const parts = astroTemplates(js);
    assert.equal(parts.length, 2);
    assert.match(parts[0], /^<p style="color:#0b0f1f"> <\/p>$/);
    assert.equal(parts[1], '<i fill="#fff">');
  });
});

describe('checkDist', () => {
  const root = mkdtempSync(join(tmpdir(), 'check-colors-'));
  after(() => rmSync(root, { recursive: true, force: true }));
  let n = 0;
  const site = (files) => {
    const dir = join(root, String(n++));
    for (const [rel, text] of Object.entries(files)) {
      const path = join(dir, ...rel.split('/'));
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, text);
    }
    return dir;
  };
  const page = (sheets, body = '') =>
    `<!doctype html><html><head>${sheets.map((s) => `<link rel="stylesheet" href="/${s}">`).join('')}` +
    `<meta name="theme-color" content="#0b0f1f"></head><body>${body}</body></html>`;
  const MAPPING = ':root,::backdrop{--sl-color-gray-1:var(--jw-fg);--sl-color-accent:var(--jw-accent)}';

  it('passes a site that uses only tokens', () => {
    const dir = site({
      'index.html': page(['_astro/a.css'], '<p style="color:var(--jw-white-60)">x</p>'),
      '_astro/a.css': ':root{--jw-x:#0b0f1f}.a{color:#ffb454;background:#ffffff0a;border-color:#0000}',
    });
    const r = checkDist(dir, tokens, { pending: [] });
    assert.deepEqual(r.failures, []);
    // --jw-x, the accent, and the page colour in theme-color.
    assert.equal(r.counts.token, 3);
    assert.equal(r.counts.alpha, 1);
    assert.equal(r.counts.transparent, 1);
  });

  it('fails a colour outside the tokens and names the file and rule', () => {
    const dir = site({
      'index.html': page(['_astro/a.css'], '<svg><path fill="#ff0000"/></svg>'),
      '_astro/a.css': '.card{border:1px solid #2a3142}',
    });
    const r = checkDist(dir, tokens, { pending: [] });
    assert.deepEqual(
      r.failures.map((f) => [f.file, f.where, f.colour]),
      [
        ['_astro/a.css', '.card', '#2a3142'],
        ['index.html', '<path fill>', '#ff0000'],
      ],
    );
  });

  it('fails a theme-color that is a token but not the page colour', () => {
    const dir = site({ 'index.html': page([]).replace('#0b0f1f', '#ffb454') });
    const r = checkDist(dir, tokens, { pending: [] });
    assert.equal(r.failures.length, 1);
    assert.match(r.failures[0].detail, /--jw-page/);
  });

  it('passes a layered vendor default that the unlayered mapping replaces', () => {
    const dir = site({
      'docs/index.html': page(['_astro/i.css']),
      '_astro/i.css': `@layer starlight.base{:root,::backdrop{--sl-color-gray-1:#edeef3}}${MAPPING}`,
    });
    const r = checkDist(dir, tokens, { pending: [] });
    assert.deepEqual(r.failures, []);
    assert.equal(r.vendor.length, 1);
  });

  it('fails a vendor default with no mapping, or on an element the mapping does not reach', () => {
    const dir = site({
      'docs/index.html': page(['_astro/i.css']),
      '_astro/i.css':
        '@layer starlight.base{:root{--sl-color-gray-9:#edeef3}.dialog{--sl-color-gray-1:#edeef3}}' + MAPPING,
    });
    const r = checkDist(dir, tokens, { pending: [] });
    assert.equal(r.failures.length, 2);
  });

  it('passes an unlayered vendor default only when every page loads the mapping after it', () => {
    const files = {
      '_astro/print.css': '@media print{:root{--sl-color-gray-1:#23262f}}',
      '_astro/i.css': MAPPING,
    };
    const good = site({ ...files, 'docs/index.html': page(['_astro/print.css', '_astro/i.css']) });
    assert.deepEqual(checkDist(good, tokens, { pending: [] }).failures, []);
    const bad = site({ ...files, 'docs/index.html': page(['_astro/i.css', '_astro/print.css']) });
    assert.equal(checkDist(bad, tokens, { pending: [] }).failures.length, 1);
  });

  it('skips a custom property only ever used as a mask', () => {
    const dir = site({
      'index.html': page(['_astro/ec.css']),
      '_astro/ec.css': ":root{--icon:url(\"data:image/svg+xml,%3Csvg stroke='black'/%3E\")}.c{mask-image:var(--icon)}",
    });
    assert.deepEqual(checkDist(dir, tokens, { pending: [] }).failures, []);
  });

  it("skips Pagefind's default UI files only while nothing loads them", () => {
    const files = { 'pagefind/pagefind-ui.css': '.pagefind-ui{color:#393939}', 'index.html': page([]) };
    const unused = site(files);
    const r = checkDist(unused, tokens, { pending: [] });
    assert.deepEqual(r.failures, []);
    assert.equal(r.skipped.length, 1);
    const used = site({ ...files, 'index.html': page(['pagefind/pagefind-ui.css']) });
    assert.equal(checkDist(used, tokens, { pending: [] }).failures.length, 1);
  });

  it('reads the markup of server-rendered templates', () => {
    const dir = site({
      '_worker.js/chunks/page.mjs': 'return renderTemplate`<meta name="theme-color" content="#0b0d12">`;',
      '_worker.js/chunks/runtime.mjs': 'const t = `<style>:root{--gray-10:#19181b}</style>`;',
    });
    const r = checkDist(dir, tokens, { pending: [] });
    assert.deepEqual(
      r.failures.map((f) => [f.file, f.colour]),
      [['_worker.js/chunks/page.mjs', '#0b0d12']],
    );
  });

  it('applies a pending exception, and reports it once it matches nothing', () => {
    const pending = [{ id: 'p', file: /^docs\//, where: 'meta theme-color', value: '#0b0d12', reason: 'x' }];
    const dir = site({ 'docs/index.html': page([]).replace('#0b0f1f', '#0b0d12') });
    const r = checkDist(dir, tokens, { pending });
    assert.deepEqual(r.failures, []);
    assert.equal(r.counts.pending, 1);
    assert.deepEqual(r.stale, []);
    const fixed = site({ 'docs/index.html': page([]) });
    assert.deepEqual(
      checkDist(fixed, tokens, { pending }).stale.map((p) => p.id),
      ['p'],
    );
  });
});
