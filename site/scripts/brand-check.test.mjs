// Run with: npm test (node --test). The last two suites run the check over the real site tree,
// so npm test, and with it CI, fails on any brand mark that site/BRAND-SOURCES.md does not allow.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import { BRANDS } from '../src/brands/brands.ts';
import {
  BRAND_SOURCES,
  PROXIMITY,
  SITE_ROOT,
  applyBaseline,
  attribute,
  checkText,
  contextFrom,
  escapeRegExp,
  formatViolation,
  lucideBindings,
  parseBrandSources,
  runBrandCheck,
  tagEnd,
} from './brand-check.mjs';

const TABLE = `# Brand and icon sources

## Brands

| Brand | Shown as | Mark source | Names matched |
| --- | --- | --- | --- |
| GitHub | logo | https://primer.style/octicons/icon/mark-github-16/ | GitHub |
| Forge | text | — | Forge, MinecraftForge |
| NeoForge | text | — | NeoForge |
| crates.io | text | — | crates.io |
| Discord | not used | — | Discord |

## Sources and terms
`;

const SOURCE = 'https://primer.style/octicons/icon/mark-github-16/';
const context = contextFrom(parseBrandSources(TABLE));
const rules = (text, file = 'src/x.astro') => checkText(file, text, context).map((v) => v.rule);
const pad = (n) => ' '.repeat(n);

describe('parseBrandSources', () => {
  it('reads every row of the Brands table', () => {
    assert.deepEqual(parseBrandSources(TABLE), [
      { name: 'GitHub', shownAs: 'logo', source: SOURCE, names: ['GitHub'] },
      { name: 'Forge', shownAs: 'text', source: null, names: ['Forge', 'MinecraftForge'] },
      { name: 'NeoForge', shownAs: 'text', source: null, names: ['NeoForge'] },
      { name: 'crates.io', shownAs: 'text', source: null, names: ['crates.io'] },
      { name: 'Discord', shownAs: 'not used', source: null, names: ['Discord'] },
    ]);
  });

  it('refuses a file without the table, a missing column or a logo without a source', () => {
    assert.throws(() => parseBrandSources('# Nothing here\n'), /no "## Brands" heading/);
    assert.throws(() => parseBrandSources('## Brands\n\n| Brand | Shown as |\n| --- | --- |\n| A | text |\n'), /no "mark source" column/);
    assert.throws(
      () => parseBrandSources('## Brands\n\n| Brand | Shown as | Mark source | Names matched |\n| - | - | - | - |\n| A | logo | — | A |\n'),
      /logo brand "A" has no mark source URL/,
    );
  });
});

describe('tagEnd and attribute', () => {
  it('skips ">" inside quotes and expressions', () => {
    const text = '<svg width={a > b ? 1 : 2} title="x > y" data-brand-source="u">rest';
    assert.equal(text.slice(0, tagEnd(text, 0)), '<svg width={a > b ? 1 : 2} title="x > y" data-brand-source="u">');
  });

  it('escapes every regular-expression metacharacter', () => {
    const s = 'a\\b^c$d.e*f+g?h(i)j[k]l{m}n|o';
    assert.ok(new RegExp(`^${escapeRegExp(s)}$`).test(s));
    assert.equal(escapeRegExp('Icons.'), 'Icons\\.');
  });

  it('reads literal and expression values', () => {
    assert.deepEqual(attribute('<svg data-brand-source="https://a/" aria-hidden={true}>', 'data-brand-source'), { literal: 'https://a/' });
    assert.deepEqual(attribute('<svg data-brand-source={src}>', 'data-brand-source'), { expression: 'src' });
    assert.equal(attribute('<svg aria-hidden="true">', 'data-brand-source'), null);
  });
});

describe('svg rules', () => {
  it('passes an official mark with a recorded source', () => {
    assert.deepEqual(rules(`<svg aria-hidden="true" data-brand-source="${SOURCE}"><path d="M0 0"/></svg> GitHub`), []);
  });

  it('fails an svg without a source', () => {
    assert.deepEqual(rules('<svg viewBox="0 0 24 24"><path d="M0 0"/></svg>'), ['svg-without-source']);
  });

  it('fails a source that BRAND-SOURCES.md does not record, or one that is not a literal', () => {
    assert.deepEqual(rules('<svg aria-hidden="true" data-brand-source="https://simpleicons.org/?q=github"></svg>'), ['svg-source-unrecorded']);
    assert.deepEqual(rules('<svg aria-hidden="true" data-brand-source={source}></svg>'), ['svg-source-unrecorded']);
  });

  it('fails a brand svg that is not aria-hidden', () => {
    assert.deepEqual(rules(`<svg data-brand-source="${SOURCE}"></svg>`), ['svg-brand-not-hidden']);
  });

  it('fails an SVG data URI', () => {
    assert.deepEqual(rules('.x { background: url("data:image/svg+xml,%3Csvg%3E"); }', 'src/x.css'), ['svg-without-source']);
  });

  it('treats a literal svg marked as Lucide as a UI icon', () => {
    assert.deepEqual(rules('<svg data-icon="lucide"><path d="M0 0"/></svg> Save'), []);
    assert.deepEqual(rules('<svg data-icon="lucide"><path d="M0 0"/></svg> Open on GitHub'), ['icon-near-brand']);
  });
});

describe('Lucide rules', () => {
  const file = (body) => `---\nimport ExternalLink from '@lucide/astro/icons/external-link';\n---\n${body}`;

  it('finds default, named, aliased and namespace imports', () => {
    const text = [
      "import ArrowRight from '@lucide/astro/icons/arrow-right';",
      "import { Map as MapIcon, Sun } from '@lucide/astro';",
      "import * as Icons from 'lucide-astro';",
      "import type { IconProps } from '@lucide/astro';",
      "import Other from './Other.astro';",
    ].join('\n');
    assert.deepEqual(lucideBindings(text), [
      { local: 'ArrowRight', icon: 'arrow-right' },
      { local: 'MapIcon', icon: 'Map' },
      { local: 'Sun', icon: 'Sun' },
      { local: 'Icons.', icon: '' },
    ]);
  });

  it('requires data-icon="lucide" on every use', () => {
    assert.deepEqual(rules(file('<a href="/x">Next <ExternalLink size={16} /></a>')), ['lucide-unmarked']);
    assert.deepEqual(rules(file('<a href="/x">Next <ExternalLink data-icon="lucide" size={16} /></a>')), []);
    assert.deepEqual(rules("---\nimport * as I from 'lucide-astro';\n---\n<I.Sun />"), ['lucide-unmarked']);
  });

  it(`fails a Lucide icon within ${PROXIMITY} characters of a brand name, before or after`, () => {
    const icon = '<ExternalLink data-icon="lucide" />';
    assert.deepEqual(rules(file(`${icon}${pad(PROXIMITY)}GitHub`)), ['icon-near-brand']);
    assert.deepEqual(rules(file(`${icon}${pad(PROXIMITY + 1)}GitHub`)), []);
    assert.deepEqual(rules(file(`GitHub${pad(PROXIMITY)}${icon}`)), ['icon-near-brand']);
    assert.deepEqual(rules(file(`GitHub${pad(PROXIMITY + 1)}${icon}`)), []);
  });

  it('matches brand names case-sensitively as whole words', () => {
    const icon = '<ExternalLink data-icon="lucide" />';
    assert.deepEqual(rules(file(`<a href="https://github.com/x">${icon}</a>`)), []);
    assert.deepEqual(rules(file(`NeoForge ${icon}`)), ['icon-near-brand']);
    assert.deepEqual(rules(file(`Forged ${icon}`)), []);
    assert.deepEqual(rules(file(`cratesXio ${icon}`)), []);
    assert.deepEqual(rules(file(`crates.io ${icon}`)), ['icon-near-brand']);
  });

  it('fails a Lucide glyph named after a brand', () => {
    assert.deepEqual(rules("---\nimport { Github as GH } from 'lucide-react';\n---\n"), ['lucide-brand-glyph']);
  });
});

describe('emoji and Simple Icons rules', () => {
  it('fails an emoji near a brand name and ignores trademark signs', () => {
    assert.deepEqual(rules('Sync to GitHub 🚀'), ['emoji-near-brand']);
    assert.deepEqual(rules('Settings ⚙️ for the Discord bot'), ['emoji-near-brand']);
    assert.deepEqual(rules(`🚀${pad(PROXIMITY + 1)}GitHub`), []);
    assert.deepEqual(rules('GitHub® and MinecraftForge™ © 2026'), []);
  });

  it('fails every way of importing Simple Icons', () => {
    for (const line of [
      "import { siGithub } from 'simple-icons';",
      "import siGithub from 'simple-icons/icons/github';",
      "const icons = await import('simple-icons');",
      "const icons = require('simple-icons');",
      '<Icon name="simple-icons:github" />',
    ]) {
      assert.deepEqual(rules(line), ['simple-icons'], line);
    }
  });
});

describe('applyBaseline', () => {
  const v = (subject) => ({ file: 'src/a.astro', line: 1, rule: 'lucide-unmarked', subject, message: '' });
  const baseline = { allowed: [{ file: 'src/a.astro', rule: 'lucide-unmarked', subject: 'Sun', count: 1 }] };

  it('allows listed violations up to their count and fails the rest', () => {
    assert.deepEqual(applyBaseline([v('Sun')], baseline), { fresh: [], known: [v('Sun')], stale: [] });
    assert.deepEqual(applyBaseline([v('Sun'), v('Sun')], baseline).fresh.length, 2);
    assert.deepEqual(applyBaseline([v('Moon')], baseline).fresh, [v('Moon')]);
  });

  it('reports entries that no longer occur', () => {
    assert.deepEqual(applyBaseline([], baseline).stale, baseline.allowed);
  });

  it('refuses a malformed entry', () => {
    assert.throws(() => applyBaseline([], { allowed: [{ file: 'a', rule: 'b', count: 1 }] }), /needs file, rule, subject/);
  });
});

describe('the site tree', () => {
  const result = runBrandCheck();

  it('has no brand-check violation outside the baseline', () => {
    assert.deepEqual(result.fresh.map(formatViolation), []);
  });

  it('has no violation at all in src/brands/', () => {
    const own = result.violations.filter((v) => v.file.startsWith('src/brands/'));
    assert.deepEqual(own.map(formatViolation), []);
  });
});

describe('src/brands/brands.ts and BRAND-SOURCES.md', () => {
  const table = parseBrandSources(readFileSync(join(SITE_ROOT, BRAND_SOURCES), 'utf8'));
  const shown = table.filter((row) => row.shownAs !== 'not used');
  const registry = Object.values(BRANDS);

  it('list the same brands, shown the same way', () => {
    assert.deepEqual(
      registry.map((b) => [b.name, b.shownAs]).sort(),
      shown.map((row) => [row.name, row.shownAs]).sort(),
    );
  });

  it('give every logo brand the same source, an official SVG component and a BrandMark entry', () => {
    const brandMark = readFileSync(join(SITE_ROOT, 'src/brands/BrandMark.astro'), 'utf8');
    for (const b of registry.filter((entry) => entry.shownAs === 'logo')) {
      assert.ok(b.mark, `${b.name} is a logo brand without mark details`);
      const row = table.find((r) => r.name === b.name);
      assert.equal(b.mark.source, row?.source, `${b.name}: brands.ts and ${BRAND_SOURCES} disagree on the mark source`);
      const path = join(SITE_ROOT, 'src/brands', `${b.mark.component}.astro`);
      assert.ok(existsSync(path), `${b.name}: src/brands/${b.mark.component}.astro is missing`);
      const svgs = [...readFileSync(path, 'utf8').matchAll(/<svg(?=[\s/>])/g)];
      assert.equal(svgs.length, 1, `${b.name}: the component must hold exactly one <svg>`);
      const text = readFileSync(path, 'utf8');
      const tag = text.slice(svgs[0].index, tagEnd(text, svgs[0].index));
      assert.deepEqual(attribute(tag, 'data-brand-source'), { literal: b.mark.source });
      assert.deepEqual(attribute(tag, 'aria-hidden'), { literal: 'true' });
      assert.match(brandMark, new RegExp(`import ${b.mark.component} from './${b.mark.component}\\.astro';`));
    }
  });

  it('give text brands no mark', () => {
    for (const b of registry.filter((entry) => entry.shownAs === 'text')) assert.equal(b.mark, undefined, b.name);
  });
});
