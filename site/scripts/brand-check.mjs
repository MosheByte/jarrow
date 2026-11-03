// Fails the build on any third-party mark that is not an official SVG recorded in
// site/BRAND-SOURCES.md: no placeholder, generic glyph, drawn lookalike or emoji may stand in
// for a brand (docs/design-research/horizon.md, section 7 and rule 22 of the thirty rules).
//
// Rules, by the id the report prints:
//
//   svg-without-source     an <svg> (or an SVG data URI) with neither data-brand-source nor
//                          data-icon="lucide"
//   svg-source-unrecorded  a data-brand-source that is not a literal URL listed as the mark
//                          source of a logo brand in the Brands table of BRAND-SOURCES.md
//   svg-brand-not-hidden   an <svg> with data-brand-source but without aria-hidden="true"
//   lucide-unmarked        a Lucide icon component used without data-icon="lucide"
//   lucide-brand-glyph     a Lucide icon whose name is a brand's (an old brand glyph)
//   icon-near-brand        a Lucide icon within 40 characters of a brand name
//   emoji-near-brand       an emoji within 40 characters of a brand name
//   simple-icons           an import of the simple-icons package or the Iconify simple-icons set
//
// Lucide icons are allowed for UI actions only. Brand names are the "Names matched" column of the
// Brands table, matched case-sensitively as whole words; distances are counted in source
// characters between the icon (the whole tag) and the name. The scan covers site/src and
// site/public, skipping node_modules, dist, .astro and *.test.* files, which never ship.
//
// Offenders that sit in files this check's own pull request could not edit are listed in
// scripts/brand-check-baseline.json. They are reported and allowed up to the listed count, so
// the list can only shrink; anything not on it fails. --strict ignores the list.
//
// Usage: node scripts/brand-check.mjs [--strict]
// npm test runs it through scripts/brand-check.test.mjs.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join, relative, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const SITE_ROOT = fileURLToPath(new URL('..', import.meta.url));
export const BRAND_SOURCES = 'BRAND-SOURCES.md';
export const BASELINE = 'scripts/brand-check-baseline.json';
export const PROXIMITY = 40;

const SCAN_DIRS = ['src', 'public'];
const SKIP_DIRS = new Set(['node_modules', 'dist', '.astro', '.wrangler']);
const TEXT_EXTENSIONS = new Set([
  '.astro', '.md', '.mdx', '.html', '.htm', '.svg', '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.json', '.css', '.yml', '.yaml', '.txt',
]);
const TEST_FILE = /\.test\.[cm]?[jt]sx?$/;

const SVG_OPEN = /<svg(?=[\s/>])/g;
const SVG_DATA_URI = /data:image\/svg\+xml/gi;
const IMPORT = /\bimport\s+([^'";]+?)\s+from\s+['"]([^'"]+)['"]/g;
const LUCIDE_PACKAGE = /^(?:@lucide\/|lucide(?:-[\w-]+)?(?:\/|$))/;
const SIMPLE_ICONS =
  /(?:\bfrom\s+|\bimport\s*\(\s*|\brequire\s*\(\s*|\bimport\s+)['"]simple-icons(?:\/[^'"]*)?['"]|['"]simple-icons:[\w-]+['"]/g;
const EMOJI = /\p{Emoji_Presentation}|\p{Extended_Pictographic}️|[#*0-9]️?⃣/gu;
const URL_IN_CELL = /https?:\/\/[^\s<>|)`]+/;

/**
 * Reads the Brands table of BRAND-SOURCES.md: the first table after a "## Brands" heading,
 * with the columns Brand, Shown as, Mark source and Names matched.
 */
export function parseBrandSources(markdown) {
  const lines = markdown.split(/\r?\n/);
  const heading = lines.findIndex((line) => /^##\s+Brands\s*$/.test(line));
  if (heading === -1) throw new Error(`${BRAND_SOURCES} has no "## Brands" heading.`);
  let i = heading + 1;
  while (i < lines.length && !lines[i].trimStart().startsWith('|')) {
    if (/^#{1,2}\s/.test(lines[i])) break;
    i += 1;
  }
  const rows = [];
  while (i < lines.length && lines[i].trimStart().startsWith('|')) rows.push(lines[i++]);
  if (rows.length < 3) throw new Error(`${BRAND_SOURCES}: the Brands table is missing or empty.`);

  const cells = (row) => row.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map((c) => c.trim());
  const header = cells(rows[0]).map((c) => c.toLowerCase());
  const column = (name) => {
    const index = header.indexOf(name);
    if (index === -1) throw new Error(`${BRAND_SOURCES}: the Brands table has no "${name}" column.`);
    return index;
  };
  const col = {
    brand: column('brand'),
    shownAs: column('shown as'),
    source: column('mark source'),
    names: column('names matched'),
  };

  const brands = rows.slice(2).map((row) => {
    const c = cells(row);
    const clean = (s) => (s ?? '').replace(/`/g, '').trim();
    const name = clean(c[col.brand]);
    const shown = clean(c[col.shownAs]).toLowerCase();
    const shownAs = shown.startsWith('logo') ? 'logo' : shown.startsWith('text') ? 'text' : 'not used';
    const sourceMatch = URL_IN_CELL.exec(c[col.source] ?? '');
    const names = clean(c[col.names])
      .split(',')
      .map((n) => n.trim())
      .filter(Boolean);
    if (!name) throw new Error(`${BRAND_SOURCES}: a Brands row has no brand name: ${row}`);
    if (names.length === 0) throw new Error(`${BRAND_SOURCES}: "${name}" lists no names to match.`);
    if (shownAs === 'logo' && !sourceMatch) throw new Error(`${BRAND_SOURCES}: logo brand "${name}" has no mark source URL.`);
    return { name, shownAs, source: shownAs === 'logo' ? sourceMatch[0] : null, names };
  });
  return brands;
}

/** Everything the per-file check needs from BRAND-SOURCES.md. */
export function contextFrom(brands) {
  const names = [...new Set(brands.flatMap((b) => b.names))].sort((a, b) => b.length - a.length);
  const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return {
    sources: new Set(brands.filter((b) => b.shownAs === 'logo').map((b) => b.source)),
    nameRegex: new RegExp(`(?<![\\p{L}\\p{N}_])(?:${names.map(escape).join('|')})(?![\\p{L}\\p{N}_])`, 'gu'),
    glyphNames: new Set(names.map((n) => n.toLowerCase().replace(/[^a-z0-9]/g, ''))),
  };
}

/** Index just past the ">" that closes the tag opening at `start`, skipping quotes and {...}. */
export function tagEnd(text, start) {
  let quote = null;
  let depth = 0;
  for (let i = start + 1; i < text.length; i += 1) {
    const c = text[i];
    if (quote) {
      if (c === quote) quote = null;
    } else if (c === '"' || c === "'" || (depth > 0 && c === '`')) {
      quote = c;
    } else if (c === '{') {
      depth += 1;
    } else if (c === '}') {
      depth = Math.max(0, depth - 1);
    } else if (c === '>' && depth === 0) {
      return i + 1;
    }
  }
  return text.length;
}

/** The value of an attribute in a tag: { literal } for "x" or 'x', { expression } for {x}. */
export function attribute(tag, name) {
  const re = new RegExp(`[\\s/]${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|\\{([^}]*)\\})`);
  const m = re.exec(tag);
  if (!m) return null;
  return m[3] !== undefined ? { expression: m[3].trim() } : { literal: m[1] ?? m[2] };
}

function isTrue(value) {
  return value !== null && (value.literal === 'true' || value.expression === 'true');
}

/** Local names that a file binds to Lucide icons, from its import statements. */
export function lucideBindings(text) {
  const bindings = [];
  for (const m of text.matchAll(IMPORT)) {
    const clause = m[1].trim();
    if (!LUCIDE_PACKAGE.test(m[2]) || /^type\s/.test(clause)) continue;
    const icon = m[2].split('/').pop() ?? '';
    const named = /\{([^}]*)\}/.exec(clause);
    for (const part of clause.replace(/\{[^}]*\}/, '').split(',').map((s) => s.trim()).filter(Boolean)) {
      const namespace = /^\*\s+as\s+([A-Za-z_$][\w$]*)$/.exec(part);
      if (namespace) bindings.push({ local: `${namespace[1]}.`, icon: '' });
      else if (/^[A-Za-z_$][\w$]*$/.test(part)) bindings.push({ local: part, icon });
    }
    for (const spec of named ? named[1].split(',') : []) {
      const s = spec.trim();
      if (!s || /^type\s/.test(s)) continue;
      const alias = /^([\w$]+)\s+as\s+([\w$]+)$/.exec(s);
      bindings.push({ local: alias ? alias[2] : s, icon: alias ? alias[1] : s });
    }
  }
  return bindings;
}

function lineStarts(text) {
  const starts = [0];
  for (let i = 0; i < text.length; i += 1) if (text[i] === '\n') starts.push(i + 1);
  return starts;
}

function lineOf(starts, index) {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (starts[mid] <= index) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}

const chars = (n) => `${n} character${n === 1 ? '' : 's'}`;

/** The nearest brand-name match to [start, end) within PROXIMITY characters, or null. */
function nearestBrand(matches, start, end) {
  let best = null;
  for (const m of matches) {
    const gap = m.start >= end ? m.start - end : m.end <= start ? start - m.end : 0;
    if (gap <= PROXIMITY && (best === null || gap < best.gap)) best = { name: m.name, gap };
  }
  return best;
}

/** All violations in one file. `file` is the path relative to the site root, with "/". */
export function checkText(file, text, context) {
  const violations = [];
  const starts = lineStarts(text);
  const add = (index, rule, subject, message) =>
    violations.push({ file, line: lineOf(starts, index), rule, subject, message });
  const brandMatches = [...text.matchAll(context.nameRegex)].map((m) => ({
    name: m[0],
    start: m.index,
    end: m.index + m[0].length,
  }));
  const near = (start, end) => nearestBrand(brandMatches, start, end);

  for (const m of text.matchAll(SVG_OPEN)) {
    const end = tagEnd(text, m.index);
    const tag = text.slice(m.index, end);
    const close = text.indexOf('</svg>', end);
    const elementEnd = close === -1 ? end : close + '</svg>'.length;
    const source = attribute(tag, 'data-brand-source');
    const icon = attribute(tag, 'data-icon');
    if (icon?.literal === 'lucide') {
      const brand = near(m.index, elementEnd);
      if (brand) add(m.index, 'icon-near-brand', `svg near ${brand.name}`, `a Lucide icon ${chars(brand.gap)} from "${brand.name}"; use the brand's mark or no icon`);
      continue;
    }
    if (source === null) {
      add(m.index, 'svg-without-source', 'svg', 'an <svg> without data-brand-source (brand marks) or data-icon="lucide" (UI icons)');
      continue;
    }
    if (source.literal === undefined || !context.sources.has(source.literal)) {
      const shown = source.literal ?? `{${source.expression}}`;
      add(m.index, 'svg-source-unrecorded', shown, `data-brand-source ${shown} is not a mark source of a logo brand in ${BRAND_SOURCES}`);
    }
    if (!isTrue(attribute(tag, 'aria-hidden'))) {
      add(m.index, 'svg-brand-not-hidden', 'svg', 'a brand <svg> needs aria-hidden="true"; the brand name is the adjacent text');
    }
  }

  for (const m of text.matchAll(SVG_DATA_URI)) {
    add(m.index, 'svg-without-source', 'data-uri', 'an SVG data URI cannot carry data-brand-source; inline the official SVG instead');
  }

  for (const binding of lucideBindings(text)) {
    const glyph = binding.icon.toLowerCase().replace(/[^a-z0-9]/g, '');
    if (glyph && context.glyphNames.has(glyph)) {
      add(text.indexOf(binding.icon), 'lucide-brand-glyph', binding.icon, `Lucide's "${binding.icon}" glyph stands in for a brand; use the official mark`);
    }
    const escaped = binding.local.replace(/[.$]/g, '\\$&');
    const usage = new RegExp(`<${escaped}${binding.local.endsWith('.') ? '[\\w$]+' : ''}(?=[\\s/>])`, 'g');
    for (const u of text.matchAll(usage)) {
      const end = tagEnd(text, u.index);
      const tag = text.slice(u.index, end);
      const name = u[0].slice(1);
      if (attribute(tag, 'data-icon')?.literal !== 'lucide') {
        add(u.index, 'lucide-unmarked', name, `<${name}> is a Lucide icon and needs data-icon="lucide"`);
      }
      const brand = near(u.index, end);
      if (brand) add(u.index, 'icon-near-brand', `${name} near ${brand.name}`, `Lucide <${name}> ${chars(brand.gap)} from "${brand.name}"; use the brand's mark or no icon`);
    }
  }

  for (const m of text.matchAll(SIMPLE_ICONS)) {
    add(m.index, 'simple-icons', 'simple-icons', 'Simple Icons copies are not official marks; inline the brand\'s own SVG from src/brands/');
  }

  for (const m of text.matchAll(EMOJI)) {
    const brand = near(m.index, m.index + m[0].length);
    if (brand) add(m.index, 'emoji-near-brand', `${m[0]} near ${brand.name}`, `an emoji ${chars(brand.gap)} from "${brand.name}"`);
  }

  return violations;
}

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return SKIP_DIRS.has(name) ? [] : walk(path);
    return TEXT_EXTENSIONS.has(extname(name).toLowerCase()) && !TEST_FILE.test(name) ? [path] : [];
  });
}

const keyOf = (v) => `${v.file}\u0000${v.rule}\u0000${v.subject}`;

/** Splits violations into new ones and ones the baseline allows; also lists stale entries. */
export function applyBaseline(violations, baseline) {
  const allowed = new Map();
  for (const entry of baseline?.allowed ?? []) {
    if (!entry.file || !entry.rule || entry.subject === undefined || !(entry.count > 0)) {
      throw new Error(`${BASELINE}: every entry needs file, rule, subject and a positive count: ${JSON.stringify(entry)}`);
    }
    allowed.set(keyOf(entry), (allowed.get(keyOf(entry)) ?? 0) + entry.count);
  }
  const groups = new Map();
  for (const v of violations) groups.set(keyOf(v), [...(groups.get(keyOf(v)) ?? []), v]);

  const fresh = [];
  const known = [];
  for (const [key, list] of groups) {
    if (list.length <= (allowed.get(key) ?? 0)) known.push(...list);
    else fresh.push(...list);
  }
  const stale = (baseline?.allowed ?? []).filter((entry) => (groups.get(keyOf(entry))?.length ?? 0) < entry.count);
  return { fresh, known, stale };
}

/** Runs the whole check over a site directory. */
export function runBrandCheck({ root = SITE_ROOT, strict = false } = {}) {
  const brands = parseBrandSources(readFileSync(join(root, BRAND_SOURCES), 'utf8'));
  const context = contextFrom(brands);
  const violations = SCAN_DIRS.flatMap((dir) => walk(join(root, dir)))
    .flatMap((path) => checkText(relative(root, path).split(sep).join('/'), readFileSync(path, 'utf8'), context))
    .sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
  const baselinePath = join(root, BASELINE);
  const baseline = !strict && existsSync(baselinePath) ? JSON.parse(readFileSync(baselinePath, 'utf8')) : null;
  return { brands, violations, ...applyBaseline(violations, baseline) };
}

export function formatViolation(v) {
  return `${v.file}:${v.line}  ${v.rule}  ${v.message}`;
}

function main() {
  const strict = process.argv.includes('--strict');
  const { fresh, known, stale } = runBrandCheck({ strict });
  if (known.length > 0) {
    console.log(`brand-check: ${known.length} known violation(s) allowed by ${BASELINE} until wave B fixes them:`);
    for (const v of known) console.log(`  ${formatViolation(v)}`);
  }
  for (const entry of stale) {
    console.log(`brand-check: ${BASELINE} allows ${entry.count} × ${entry.rule} "${entry.subject}" in ${entry.file}, fewer remain; lower or remove the entry.`);
  }
  if (fresh.length > 0) {
    console.error(`brand-check: ${fresh.length} violation(s). Every brand mark must be an official SVG from ${BRAND_SOURCES}:`);
    for (const v of fresh) console.error(`  ${formatViolation(v)}`);
    process.exit(1);
  }
  console.log('brand-check: no new violations.');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
