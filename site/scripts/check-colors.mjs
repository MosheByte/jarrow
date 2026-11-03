// Fails the build when the built site uses a colour that is not a design token.
//
// Runs after `astro build` and the Pages assembly (the last step of `npm run build`), so CI
// runs it on every pull request. It reads every CSS and HTML file in dist/, and the markup of
// the server-rendered routes (/, /board), which have no HTML file: the renderTemplate`...`
// literals of the compiled components in dist/_worker.js/. It finds every colour literal:
// hex, rgb()/rgba(), hsl()/hsla(), color-mix(), other colour functions, and named colours
// (`transparent`, `currentColor` and `inherit` are not colours of their own). Each one must be:
//
//   a token        a colour declared in site/tokens.css (crates/report generates it), or
//   an alpha       an alpha step tokens.css declares (white at 60 %, the page at 75 %, ...),
//                  written as a literal or as color-mix(var(--jw-x) N%, transparent).
//
// Anything else fails the build, with two narrow, proven exceptions:
//
//   Vendor defaults. Starlight, Pagefind and Tailwind declare default palettes as custom
//   properties on :root (or ::backdrop). They pass only when the check proves they never
//   paint: site/src/styles/global.css redeclares the same property, unlayered, on :root, with
//   a token, in the same stylesheet or in one every page that loads the vendor file loads
//   after it. A vendor default with no such override fails.
//
//   Files no page loads. Pagefind writes its own default-UI stylesheets next to the search
//   index (dist/pagefind/pagefind-*ui.css); Starlight bundles its own copy into the page CSS,
//   so they are skipped, but only while no HTML, CSS or script in dist/ names them.
//
// Mask properties (mask, mask-image, -webkit-mask-*), and custom properties used only inside
// them (Expressive Code's copy icon), are skipped: their colours are alpha masks and never
// paint. Feature queries (@supports preludes) are not declarations. The browser theme colour
// (<meta name="theme-color">) must be --jw-page itself.
//
// Usage: node scripts/check-colors.mjs [--dist <dir>] [--verbose]
// Exit status: 0 when every colour is a token, 1 when any is not, 2 when dist/ is missing.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { formatColor, NAMED_COLORS, parseColor, readTokens, sameColor } from './tokens.mjs';

const SITE = fileURLToPath(new URL('..', import.meta.url));

/**
 * Exceptions for colours that come from a file this change may not edit yet. Each names the
 * file pattern, where the colour is, the exact value, and why; the check fails when an entry
 * no longer matches anything, so it is removed as soon as the source is fixed.
 */
export const PENDING = [
  {
    id: 'docs-theme-color',
    file: /^docs\/(?:.+\/)?index\.html$/,
    where: 'meta theme-color',
    value: '#0b0d12',
    reason:
      'the docs <meta name="theme-color"> comes from the Starlight head option in astro.config.mjs, ' +
      'which the any-loader PR series is editing; it becomes #0b0f1f (--jw-page) there',
  },
];

/** Pagefind's own default-UI stylesheets; Starlight bundles its own copy of the UI CSS. */
const PAGEFIND_UI = /^pagefind\/pagefind-(?:component-|modular-)?ui\.css$/;

const MASK = /^(?:-webkit-)?mask/;
// Properties in which a bare word can be a named colour. Elsewhere (font-family, animation,
// grid-area, transition-property, ...) words like "black" or "tan" are not colours.
const NAMED_OK =
  /^(?:--|color$|background|border|outline|fill|stroke|stop-color|flood-color|lighting-color|caret-color|accent-color|column-rule|text-decoration|text-emphasis|box-shadow|text-shadow|filter|backdrop-filter|scrollbar-color|-webkit-tap-highlight-color|-webkit-text-fill-color|-webkit-text-stroke)/;
const COLOR_FUNCTION = /(?<![\w-])(color-mix|light-dark|rgba?|hsla?|hwb|oklab|oklch|lab|lch|color)\(/gi;
const HEX = /(?<![\w&#-])#([0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{3,4})(?![\w-])/gi;
const WORD = /(?<![\w-])([a-z]+)(?![\w(-])/gi;

// ---------------------------------------------------------------------------------------------
// CSS

/** The index of the parenthesis that closes the one at `open`, or -1. */
function closing(text, open) {
  let depth = 0;
  for (let i = open; i < text.length; i++) {
    const c = text[i];
    if (c === '"' || c === "'") {
      const end = text.indexOf(c, i + 1);
      if (end < 0) return -1;
      i = end;
    } else if (c === '(') depth++;
    else if (c === ')' && --depth === 0) return i;
  }
  return -1;
}

/** Splits on `separator` where it is not inside parentheses or quotes. */
export function splitTopLevel(text, separator = ',') {
  const parts = [];
  let depth = 0;
  let quote = '';
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === '\\') i++;
      else if (c === quote) quote = '';
    } else if (c === '"' || c === "'") quote = c;
    else if (c === '(' || c === '[') depth++;
    else if (c === ')' || c === ']') depth--;
    else if (c === separator && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.map((p) => p.trim()).filter(Boolean);
}

/**
 * Parses CSS into its declarations, in source order. Each declaration records its property,
 * value, the innermost style rule's selector, the enclosing at-rules, and whether it sits in a
 * cascade layer. At-rule preludes (@media, @supports, @layer, @import) are not declarations.
 */
export function parseCss(css) {
  const out = [];
  const stack = [];
  let buf = '';
  const flush = () => {
    const text = buf.trim();
    buf = '';
    if (!text || text.startsWith('@')) return;
    const colon = text.indexOf(':');
    if (colon <= 0) return;
    const rawProperty = text.slice(0, colon).trim();
    const property = rawProperty.startsWith('--') ? rawProperty : rawProperty.toLowerCase();
    const value = text
      .slice(colon + 1)
      .replace(/!\s*important\s*$/i, '')
      .trim();
    const rule = stack.findLast((s) => s.kind === 'rule');
    out.push({
      property,
      value,
      selector: rule ? rule.prelude : '',
      atRules: stack.filter((s) => s.kind === 'at').map((s) => s.prelude),
      layered: stack.some((s) => s.kind === 'at' && /^@layer\b/i.test(s.prelude)),
      index: out.length,
    });
  };
  for (let i = 0; i < css.length; i++) {
    const c = css[i];
    if (c === '/' && css[i + 1] === '*') {
      const end = css.indexOf('*/', i + 2);
      i = end < 0 ? css.length : end + 1;
    } else if (c === '"' || c === "'") {
      let j = i + 1;
      while (j < css.length && css[j] !== c) j += css[j] === '\\' ? 2 : 1;
      buf += css.slice(i, j + 1);
      i = j;
    } else if (c === '(') {
      const end = closing(css, i);
      const stop = end < 0 ? css.length - 1 : end;
      buf += css.slice(i, stop + 1);
      i = stop;
    } else if (c === '{') {
      const prelude = buf.trim();
      buf = '';
      stack.push({ kind: prelude.startsWith('@') ? 'at' : 'rule', prelude });
    } else if (c === '}') {
      flush();
      stack.pop();
    } else if (c === ';') {
      flush();
    } else {
      buf += c;
    }
  }
  flush();
  return out;
}

/** Parses `a: b; c: d` (a style attribute). */
export function parseDeclarations(text) {
  return parseCss(`x{${text}}`).map((d) => ({ ...d, selector: '' }));
}

function decodeEntities(text) {
  return text
    .replace(/&quot;/g, '"')
    .replace(/&#0*39;|&#x0*27;|&apos;/gi, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
}

/** Colour literals in the attributes of an SVG (fill, stroke, stop-color, style...). */
function svgColors(svg) {
  const found = [];
  for (const m of svg.matchAll(/\b(fill|stroke|stop-color|flood-color|lighting-color|color)\s*=\s*(["'])(.*?)\2/gi)) {
    found.push(...colorsIn(m[3], m[1].toLowerCase()));
  }
  for (const m of svg.matchAll(/\bstyle\s*=\s*(["'])(.*?)\1/gi)) {
    for (const d of parseDeclarations(m[2])) found.push(...colorsIn(d.value, d.property));
  }
  return found;
}

/**
 * Every colour in a declaration value, as { text, kind }: kind is 'literal' (with .color),
 * 'mix' (a color-mix() or light-dark()), 'function' (a colour function this check cannot
 * resolve, such as one built from var() or in another colour space).
 */
export function colorsIn(value, property = '') {
  if (MASK.test(property)) return [];
  const found = [];
  let text = value;
  // url(): skipped, except SVG data URIs, whose fills paint.
  text = text.replace(/url\(\s*(["']?)(.*?)\1\s*\)/gi, (_, _q, url) => {
    if (/^data:image\/svg\+xml/i.test(url)) {
      let svg = url.replace(/^data:image\/svg\+xml(;[^,]*)?,/i, '');
      try {
        svg = decodeURIComponent(svg);
      } catch {
        // Not percent-encoded; scan as is.
      }
      found.push(...svgColors(svg));
    }
    return ' ';
  });
  text = text.replace(/"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/g, ' ');
  // Colour functions first; their text is blanked so their insides are not counted twice.
  for (;;) {
    COLOR_FUNCTION.lastIndex = 0;
    const m = COLOR_FUNCTION.exec(text);
    if (!m) break;
    const open = m.index + m[0].length - 1;
    const end = closing(text, open);
    const stop = end < 0 ? text.length - 1 : end;
    const fn = text.slice(m.index, stop + 1);
    const name = m[1].toLowerCase();
    if (name === 'color-mix' || name === 'light-dark') found.push({ text: fn, kind: 'mix' });
    else {
      const color = parseColor(fn);
      found.push(color ? { text: fn, kind: 'literal', color } : { text: fn, kind: 'function' });
    }
    text = text.slice(0, m.index) + ' '.repeat(fn.length) + text.slice(stop + 1);
  }
  for (const m of text.matchAll(HEX)) {
    const color = parseColor(`#${m[1]}`);
    if (color) found.push({ text: `#${m[1]}`, kind: 'literal', color });
  }
  if (NAMED_OK.test(property)) {
    // var(--name) and other identifiers that only look like words are not colours.
    const words = text.replace(/var\(\s*--[\w-]+/g, ' ');
    for (const m of words.matchAll(WORD)) {
      const word = m[1].toLowerCase();
      if (Object.hasOwn(NAMED_COLORS, word)) found.push({ text: m[1], kind: 'literal', color: parseColor(word) });
    }
  }
  return found;
}

// ---------------------------------------------------------------------------------------------
// The palette

/** The colours tokens.css declares: [{ name, color }]. */
export function palette(tokens) {
  const entries = [];
  for (const [name, value] of tokens) {
    const color = parseColor(value);
    if (color) entries.push({ name, color });
  }
  return entries;
}

function inPalette(color, entries) {
  if (color.a === 0) return { status: 'transparent' };
  const hit = entries.find((e) => sameColor(e.color, color));
  if (!hit) return null;
  return { status: hit.color.a < 1 ? 'alpha' : 'token', token: hit.name };
}

/** The share each side of a color-mix() gets (CSS Color 5, 'percentage normalization'). */
function mixShares(p1, p2) {
  let a = p1;
  let b = p2;
  if (a === undefined && b === undefined) [a, b] = [50, 50];
  else if (a === undefined) a = 100 - b;
  else if (b === undefined) b = 100 - a;
  const sum = a + b;
  if (sum <= 0) return null;
  // A sum under 100 % also lowers the result's alpha by that factor.
  const alpha = sum < 100 ? sum / 100 : 1;
  return [(a / sum) * alpha, (b / sum) * alpha];
}

function mixSide(text) {
  const m = /^(.*?)\s*(-?\d+(?:\.\d+)?)%\s*$|^\s*(-?\d+(?:\.\d+)?)%\s+(.*)$/.exec(text);
  if (!m) return { color: text.trim(), percent: undefined };
  return m[1] !== undefined
    ? { color: m[1].trim(), percent: Number(m[2]) }
    : { color: m[4].trim(), percent: Number(m[3]) };
}

/** Classifies one colour found by colorsIn() against the palette. */
export function classify(item, tokens, entries) {
  if (item.kind === 'literal') {
    return inPalette(item.color, entries) ?? { status: 'foreign', detail: `${formatColor(item.color)} is not a token` };
  }
  if (item.kind === 'function') {
    return { status: 'foreign', detail: `${item.text} is not a token value` };
  }
  const inner = item.text.slice(item.text.indexOf('(') + 1, -1);
  if (/^light-dark/i.test(item.text)) {
    const parts = splitTopLevel(inner);
    const results = parts.flatMap((p) => colorsIn(p, 'color')).map((c) => classify(c, tokens, entries));
    const bad = results.find((r) => r.status === 'foreign');
    return bad ?? { status: 'token' };
  }
  const parts = splitTopLevel(inner);
  if (parts.length !== 3 || !/^in\s/i.test(parts[0])) {
    return { status: 'foreign', detail: `${item.text} is not a mix this check can read` };
  }
  const sides = [mixSide(parts[1]), mixSide(parts[2])];
  const shares = mixShares(sides[0].percent, sides[1].percent);
  if (!shares) return { status: 'foreign', detail: `${item.text} mixes nothing` };
  const transparent = sides.map((s) => /^transparent$/i.test(s.color));
  if (transparent[0] === transparent[1]) {
    return transparent[0]
      ? { status: 'transparent' }
      : { status: 'foreign', detail: `${item.text} mixes two colours into a new one` };
  }
  const k = transparent[0] ? 1 : 0;
  const side = sides[k].color;
  const share = shares[k];
  // currentColor is the element's own (checked) text colour.
  if (/^currentcolor$/i.test(side)) return { status: 'alpha', token: 'currentColor' };
  const variable = /^var\(\s*(--[\w-]+)\s*(?:,(.*))?\)$/s.exec(side);
  if (variable) {
    const [, name, fallback] = variable;
    if (fallback) {
      for (const c of colorsIn(fallback, '--fallback')) {
        const r = classify(c, tokens, entries);
        if (r.status === 'foreign') return r;
      }
    }
    if (!name.startsWith('--jw-')) {
      // An alpha of another variable (Starlight's aside accent, say); that variable's own
      // value is checked where it is declared.
      return { status: 'alpha', token: name };
    }
    const base = parseColor(tokens.get(name) ?? '');
    if (!base) return { status: 'foreign', detail: `${name} is not a colour token` };
    const color = { ...base, a: base.a * share };
    return (
      inPalette(color, entries) ?? {
        status: 'foreign',
        detail: `${name} at ${Math.round(share * 1000) / 10} % is not an alpha step tokens.css declares`,
      }
    );
  }
  const literal = parseColor(side);
  if (!literal) return { status: 'foreign', detail: `${item.text} is not a mix this check can read` };
  const color = { ...literal, a: literal.a * share };
  return inPalette(color, entries) ?? { status: 'foreign', detail: `${item.text} is not a token alpha` };
}

// ---------------------------------------------------------------------------------------------
// HTML

/** Stylesheets (in order), inline styles, style attributes, colour attributes, theme-color. */
export function htmlSources(html) {
  const text = html.replace(/<!--[\s\S]*?-->/g, ' ');
  const styles = [...text.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)].map((m) => m[1]);
  const markup = text
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, ' ');
  const stylesheets = [];
  const attributes = [];
  const declarations = [];
  for (const tag of markup.matchAll(/<([a-z][\w:-]*)\b((?:[^<>"']|"[^"]*"|'[^']*')*)>/gi)) {
    const name = tag[1].toLowerCase();
    const attrs = {};
    for (const a of tag[2].matchAll(/([\w:-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))/g)) {
      attrs[a[1].toLowerCase()] = decodeEntities(a[2] ?? a[3] ?? a[4] ?? '');
    }
    if (name === 'link' && /\bstylesheet\b/i.test(attrs.rel ?? '') && attrs.href) stylesheets.push(attrs.href);
    if (name === 'meta' && (attrs.name ?? '').toLowerCase() === 'theme-color' && attrs.content) {
      attributes.push({ where: 'meta theme-color', property: 'color', value: attrs.content });
    }
    for (const attr of ['fill', 'stroke', 'stop-color', 'flood-color', 'lighting-color', 'color', 'bgcolor']) {
      if (attrs[attr] !== undefined) attributes.push({ where: `<${name} ${attr}>`, property: attr === 'bgcolor' ? 'background' : attr, value: attrs[attr] });
    }
    if (attrs.style) {
      for (const d of parseDeclarations(attrs.style)) declarations.push({ ...d, where: `<${name} style>` });
    }
  }
  return { styles, stylesheets, attributes, declarations };
}

/** Skips a JavaScript string or template literal starting at `i`; returns the index after it. */
function skipString(js, i) {
  const quote = js[i];
  let j = i + 1;
  while (j < js.length) {
    const c = js[j];
    if (c === '\\') j += 2;
    else if (c === quote) return j + 1;
    else if (quote === '`' && c === '$' && js[j + 1] === '{') j = skipExpression(js, j + 2);
    else j++;
  }
  return j;
}

/** Skips a `${...}` expression body starting after `${`; returns the index after its `}`. */
function skipExpression(js, i) {
  let depth = 1;
  let j = i;
  while (j < js.length && depth > 0) {
    const c = js[j];
    if (c === '"' || c === "'" || c === '`') j = skipString(js, j);
    else {
      if (c === '{') depth++;
      else if (c === '}') depth--;
      j++;
    }
  }
  return j;
}

/**
 * The markup of compiled .astro components in a server bundle: the literal parts of every
 * renderTemplate`...` tagged template, with each ${expression} replaced by a space. Astro's
 * own runtime templates (its default error pages) are plain strings and are not included.
 */
export function astroTemplates(js) {
  const out = [];
  const tag = 'renderTemplate`';
  for (let start = js.indexOf(tag); start >= 0; start = js.indexOf(tag, start + tag.length)) {
    let text = '';
    let j = start + tag.length;
    while (j < js.length) {
      const c = js[j];
      if (c === '\\') {
        text += js.slice(j, j + 2);
        j += 2;
      } else if (c === '`') break;
      else if (c === '$' && js[j + 1] === '{') {
        j = skipExpression(js, j + 2);
        text += ' ';
      } else {
        text += c;
        j++;
      }
    }
    out.push(text);
  }
  return out;
}

/** Custom properties of a stylesheet that are only ever used inside mask properties. */
function maskOnlyProperties(decls) {
  const uses = new Map();
  for (const d of decls) {
    for (const m of d.value.matchAll(/var\(\s*(--[\w-]+)/g)) {
      uses.set(m[1], [...(uses.get(m[1]) ?? []), d.property]);
    }
  }
  const masks = new Set();
  for (const [name, properties] of uses) if (properties.every((p) => MASK.test(p))) masks.add(name);
  return masks;
}

// ---------------------------------------------------------------------------------------------
// The check

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

function rootTarget(selector) {
  const parts = splitTopLevel(selector);
  if (parts.length === 0) return null;
  let qualified = false;
  let backdrop = false;
  for (const part of parts) {
    const root = /^(?::root|html)?((?:\[[^\]]*\])*)(?::root)?$/i.exec(part);
    const back = /^(?:(?::root|html)?((?:\[[^\]]*\])*)\s+)?::backdrop$/i.exec(part);
    if (root && part !== '') qualified ||= root[1] !== '' || /\[/.test(part);
    else if (back) {
      backdrop = true;
      qualified ||= Boolean(back[1]);
    } else return null;
  }
  return { qualified, backdrop };
}

function selectorList(selector) {
  return splitTopLevel(selector).map((s) => s.toLowerCase());
}

/**
 * Checks a built site. Returns { failures, pending, stale, counts, vendor, skipped }.
 * `dist` is the directory; `tokens` the Map from readTokens().
 */
export function checkDist(dist, tokens, { pending = PENDING } = {}) {
  const entries = palette(tokens);
  const files = walk(dist).map((path) => ({ path, rel: relative(dist, path).split(sep).join('/') }));
  const css = files.filter((f) => f.rel.endsWith('.css'));
  const html = files.filter((f) => f.rel.endsWith('.html'));
  const templates = files.filter((f) => f.rel.startsWith('_worker.js/') && /\.m?js$/.test(f.rel));
  const scripts = files.filter((f) => /\.m?js$/.test(f.rel));

  const failures = [];
  const vendor = [];
  const skipped = [];
  const counts = { files: 0, colours: 0, token: 0, alpha: 0, transparent: 0, vendor: 0, pending: 0 };
  const pendingHits = new Map(pending.map((p) => [p.id, 0]));

  const record = (rel, where, property, value, item, result) => {
    counts.colours++;
    if (result.status !== 'foreign') {
      counts[result.status]++;
      return;
    }
    const exception = pending.find(
      (p) => p.file.test(rel) && p.where === where && p.value.toLowerCase() === item.text.toLowerCase(),
    );
    if (exception) {
      counts.pending++;
      pendingHits.set(exception.id, pendingHits.get(exception.id) + 1);
      return;
    }
    failures.push({ file: rel, where, property, value, colour: item.text, detail: result.detail });
  };

  // Pages and the stylesheets they load, in order.
  const pages = html.map((f) => {
    const text = readFileSync(f.path, 'utf8');
    const sources = htmlSources(text);
    const sheets = sources.stylesheets.map((href) => {
      const clean = href.split(/[?#]/)[0];
      if (clean.startsWith('/')) return clean.slice(1);
      const base = f.rel.includes('/') ? f.rel.slice(0, f.rel.lastIndexOf('/') + 1) : '';
      return new URL(clean, `file:///${base}`).pathname.slice(1);
    });
    return { ...f, sources, sheets };
  });
  const loadedBy = new Map();
  for (const page of pages) {
    for (const sheet of page.sheets) loadedBy.set(sheet, [...(loadedBy.get(sheet) ?? []), page]);
  }

  // Declarations of every stylesheet, and Jarrow's mapping (unlayered custom properties on
  // :root whose values hold only tokens).
  const parsed = new Map(css.map((f) => [f.rel, parseCss(readFileSync(f.path, 'utf8'))]));
  const mapping = new Map();
  for (const [rel, decls] of parsed) {
    const map = new Map();
    for (const d of decls) {
      if (d.layered || !d.property.startsWith('--')) continue;
      const list = selectorList(d.selector);
      if (!list.includes(':root')) continue;
      const clean = colorsIn(d.value, d.property).every((c) => classify(c, tokens, entries).status !== 'foreign');
      if (clean) map.set(d.property, { index: d.index, backdrop: list.includes('::backdrop') });
    }
    mapping.set(rel, map);
  }

  const overridden = (rel, d) => {
    if (!d.property.startsWith('--')) return null;
    const target = rootTarget(d.selector);
    if (!target) return null;
    // An unlayered default on a qualified selector (:root[data-theme]) outranks :root.
    if (!d.layered && target.qualified) return null;
    const ok = (m) => m && (!target.backdrop || m.backdrop);
    const same = mapping.get(rel)?.get(d.property);
    if (ok(same) && (d.layered || same.index > d.index)) return rel;
    const loaders = loadedBy.get(rel) ?? [];
    if (loaders.length === 0) return null;
    let by = null;
    for (const page of loaders) {
      const at = page.sheets.indexOf(rel);
      const other = page.sheets.find(
        (sheet, k) => sheet !== rel && ok(mapping.get(sheet)?.get(d.property)) && (d.layered || k > at),
      );
      if (!other) return null;
      by = other;
    }
    return by;
  };

  const texts = new Map();
  const textOf = (f) => {
    if (!texts.has(f.rel)) texts.set(f.rel, readFileSync(f.path, 'utf8'));
    return texts.get(f.rel);
  };
  const referenced = (rel) => {
    if (loadedBy.has(rel)) return true;
    const name = rel.slice(rel.lastIndexOf('/') + 1);
    return [...html, ...css, ...scripts].some((f) => f.rel !== rel && textOf(f).includes(name));
  };

  for (const f of css) {
    if (PAGEFIND_UI.test(f.rel) && !referenced(f.rel)) {
      skipped.push({ file: f.rel, why: "Pagefind's default UI stylesheet; no page, stylesheet or script loads it" });
      continue;
    }
    counts.files++;
    const decls = parsed.get(f.rel);
    const masks = maskOnlyProperties(decls);
    for (const d of decls) {
      // A custom property only ever used as a mask (an icon drawn through mask-image) is a mask.
      if (masks.has(d.property)) continue;
      for (const item of colorsIn(d.value, d.property)) {
        let result = classify(item, tokens, entries);
        if (result.status === 'foreign') {
          const by = overridden(f.rel, d);
          if (by) {
            vendor.push({ file: f.rel, selector: d.selector, property: d.property, colour: item.text, by });
            result = { status: 'vendor' };
          }
        }
        record(f.rel, d.selector || d.atRules.at(-1) || '(top level)', d.property, d.value, item, result);
      }
    }
  }

  const checkSources = (rel, sources) => {
    for (const style of sources.styles) {
      for (const d of parseCss(style)) {
        for (const item of colorsIn(d.value, d.property)) {
          record(rel, `<style> ${d.selector}`.trim(), d.property, d.value, item, classify(item, tokens, entries));
        }
      }
    }
    for (const d of sources.declarations) {
      for (const item of colorsIn(d.value, d.property)) {
        record(rel, d.where, d.property, d.value, item, classify(item, tokens, entries));
      }
    }
    for (const a of sources.attributes) {
      // none, currentColor, url(#paint) and SMIL's fill="freeze" are not colours.
      for (const item of colorsIn(a.value, a.property)) {
        const result = classify(item, tokens, entries);
        // The browser's theme colour is the page colour (report rule 1), not any token.
        if (a.where === 'meta theme-color' && result.status !== 'foreign' && result.token !== '--jw-page') {
          failures.push({ file: rel, where: a.where, property: a.property, value: a.value, colour: item.text, detail: 'theme-color is not --jw-page' });
          continue;
        }
        record(rel, a.where, a.property, a.value, item, result);
      }
    }
  };

  for (const page of pages) {
    counts.files++;
    checkSources(page.rel, page.sources);
  }
  // Server-rendered routes have no HTML file in dist/; their markup is in the worker's
  // compiled component templates.
  for (const f of templates) {
    const markup = astroTemplates(textOf(f));
    if (markup.length === 0) continue;
    counts.files++;
    const sources = htmlSources(markup.join('\n'));
    checkSources(f.rel, { ...sources, stylesheets: [] });
  }

  const stale = pending.filter((p) => pendingHits.get(p.id) === 0);
  return { failures, stale, counts, vendor, skipped, pending: pending.filter((p) => pendingHits.get(p.id) > 0) };
}

function main(argv) {
  const verbose = argv.includes('--verbose');
  const distArg = argv.indexOf('--dist');
  const dist = distArg >= 0 ? argv[distArg + 1] : join(SITE, 'dist');
  if (!existsSync(dist)) {
    console.error(`check-colors: ${dist} does not exist; run astro build first.`);
    return 2;
  }
  const tokens = readTokens();
  const { failures, stale, counts, vendor, skipped, pending } = checkDist(dist, tokens);
  const summary =
    `${counts.files} files, ${counts.colours} colour values: ${counts.token} tokens, ${counts.alpha} token alphas, ` +
    `${counts.transparent} transparent, ${counts.vendor} vendor defaults replaced by the token mapping` +
    (counts.pending ? `, ${counts.pending} pending (named above)` : '');
  if (verbose) {
    for (const v of vendor) console.log(`  vendor default ${v.file}: ${v.selector} { ${v.property}: ${v.colour} } -> ${v.by}`);
    for (const s of skipped) console.log(`  skipped ${s.file}: ${s.why}`);
  }
  for (const p of pending) console.warn(`check-colors: pending ${p.id}: ${p.value} in ${p.where}: ${p.reason}`);
  let status = 0;
  for (const p of stale) {
    console.error(`check-colors: the pending exception ${p.id} matches nothing any more; remove it from scripts/check-colors.mjs.`);
    status = 1;
  }
  if (failures.length > 0) {
    console.error(`check-colors: ${failures.length} colour${failures.length === 1 ? '' : 's'} outside site/tokens.css:`);
    for (const f of failures.slice(0, 60)) {
      console.error(`  ${f.file}: ${f.where} { ${f.property}: ${f.value.length > 120 ? `${f.value.slice(0, 117)}...` : f.value} }: ${f.detail}`);
    }
    if (failures.length > 60) console.error(`  ... and ${failures.length - 60} more`);
    console.error('Use a token from site/tokens.css (crates/report/src/tokens.rs), or add the value there first.');
    status = 1;
  }
  if (status === 0) console.log(`check-colors: ${summary}. Every colour is a token.`);
  return status;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main(process.argv.slice(2));
}
