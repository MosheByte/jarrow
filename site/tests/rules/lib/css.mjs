// The shipped CSS and the stylesheet-level rules: no uppercase (6), no light mode (1), one
// entrance keyframe (20), hover changes colour only (21), no SVG wave masks (19), no raster
// decoration (18), and the colour token check.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';

import postcss from 'postcss';

import { colourLiterals, parseColor, sameRgb, toHex } from './color.mjs';

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

/**
 * Collects the CSS a visitor's browser receives: every stylesheet the pages link, every inline
 * <style> block, and every CSS file Astro emitted under dist/_astro. `pages` are
 * { route, html } objects fetched from the running site; `fetchText(path)` fetches a linked
 * stylesheet that is not on disk.
 */
export async function collectShippedCss({ dist, pages, fetchText }) {
  const sheets = new Map();
  for (const file of walk(join(dist, '_astro'))) {
    if (file.endsWith('.css')) {
      const rel = `/${relative(dist, file).split(sep).join('/')}`;
      sheets.set(rel, readFileSync(file, 'utf8'));
    }
  }
  for (const { route, html } of pages) {
    let i = 0;
    for (const m of html.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)) {
      sheets.set(`${route} <style> #${++i}`, m[1]);
    }
    for (const m of html.matchAll(/<link\b[^>]*>/gi)) {
      const tag = m[0];
      if (!/\brel=["']?stylesheet/i.test(tag)) continue;
      const href = tag.match(/\bhref=["']?([^"' >]+)/i)?.[1];
      if (!href || sheets.has(href)) continue;
      if (/^https?:\/\//i.test(href)) {
        sheets.set(href, `/* external stylesheet ${href}: not read */`);
        continue;
      }
      const onDisk = join(dist, href.replace(/^\/+/, '').split('?')[0]);
      sheets.set(href, existsSync(onDisk) ? readFileSync(onDisk, 'utf8') : await fetchText(href));
    }
  }
  return [...sheets].map(([source, css]) => ({ source, css, root: parseCss(css, source) }));
}

export function parseCss(css, source = 'inline') {
  try {
    return postcss.parse(css, { from: source });
  } catch (error) {
    // A stylesheet the browser would still partly apply; report it rather than crash.
    return postcss.parse(`/* unparsable: ${String(error.message).replaceAll('*/', '')} */`, { from: source });
  }
}

/** Ancestors of a node, innermost first: rules and at-rules. */
function ancestors(node) {
  const out = [];
  for (let p = node.parent; p && p.type !== 'root'; p = p.parent) out.push(p);
  return out;
}

function selectorChain(node) {
  return ancestors(node)
    .filter((a) => a.type === 'rule')
    .map((a) => a.selector)
    .reverse()
    .join(' ');
}

function mediaChain(node) {
  return ancestors(node)
    .filter((a) => a.type === 'atrule')
    .map((a) => `@${a.name} ${a.params}`)
    .reverse()
    .join(' ');
}

function where(sheet, decl) {
  const line = decl.source?.start?.line;
  const sel = selectorChain(decl) || '(top level)';
  const media = mediaChain(decl);
  return `${sheet.source}${line ? `:${line}` : ''} ${media ? `${media} ` : ''}${sel} { ${decl.prop}: ${decl.value} }`.slice(0, 300);
}

function eachDecl(sheets, fn) {
  for (const sheet of sheets) sheet.root.walkDecls((decl) => fn(decl, sheet));
}

/** Rule 6: no uppercase, title case or all-caps variants. */
export function uppercaseDeclarations(sheets) {
  const found = [];
  eachDecl(sheets, (decl, sheet) => {
    const prop = decl.prop.toLowerCase();
    const value = decl.value.toLowerCase();
    if (prop === 'text-transform' && /\b(uppercase|capitalize)\b/.test(value)) found.push(where(sheet, decl));
    if ((prop === 'font-variant-caps' || prop === 'font-variant') && /\b(all-small-caps|all-petite-caps|small-caps|petite-caps|titling-caps|unicase)\b/.test(value)) {
      found.push(where(sheet, decl));
    }
  });
  return found;
}

/** Rule 1: dark only. Any prefers-color-scheme query, light theme selector or light color-scheme. */
export function lightModeRules(sheets) {
  const found = [];
  for (const sheet of sheets) {
    sheet.root.walkAtRules((at) => {
      if (/prefers-color-scheme/i.test(at.params)) found.push(`${sheet.source}:${at.source?.start?.line ?? '?'} @${at.name} ${at.params}`);
    });
    sheet.root.walkRules((rule) => {
      if (/\[data-theme=["']?light|\.light\b|\.theme-light\b|:root\.light/i.test(rule.selector)) {
        found.push(`${sheet.source}:${rule.source?.start?.line ?? '?'} ${rule.selector.slice(0, 160)}`);
      }
    });
    sheet.root.walkDecls('color-scheme', (decl) => {
      if (/\blight\b/i.test(decl.value)) found.push(where(sheet, decl));
    });
  }
  return found;
}

function framesOf(at) {
  const frames = {};
  at.each((node) => {
    if (node.type !== 'rule') return;
    const decls = {};
    node.walkDecls((d) => {
      decls[d.prop.toLowerCase()] = d.value.trim();
    });
    for (const key of node.selector.split(',').map((s) => s.trim().toLowerCase())) {
      const k = key === 'from' ? '0%' : key === 'to' ? '100%' : key;
      frames[k] = { ...(frames[k] ?? {}), ...decls };
    }
  });
  return frames;
}

/** True for the one entrance keyframe: from opacity 0 and 14 px down, to the resting state. */
export function isEntranceFrames(frames, translatePx = 14) {
  const first = frames['0%'];
  if (!first) return false;
  const opacityZero = first.opacity !== undefined && parseFloat(first.opacity) === 0;
  const transform = (first.transform ?? '').toLowerCase().replace(/\s+/g, '');
  const translate = (first.translate ?? '').toLowerCase().trim().replace(/\s+/g, ' ');
  const p = String(translatePx);
  const viaTransform =
    transform === `translatey(${p}px)` ||
    new RegExp(`^translate(3d)?\\(0(px)?,${p}px(,0(px)?)?\\)$`).test(transform);
  const viaTranslate = new RegExp(`^0(px)? ${p}px$`).test(translate);
  return opacityZero && (viaTransform || viaTranslate);
}

/** Rule 20: every @keyframes in the shipped CSS, with whether it is the entrance keyframe. */
export function keyframes(sheets, translatePx = 14) {
  const list = [];
  for (const sheet of sheets) {
    sheet.root.walkAtRules(/^(-webkit-)?keyframes$/i, (at) => {
      const frames = framesOf(at);
      list.push({
        name: at.params.trim(),
        source: `${sheet.source}:${at.source?.start?.line ?? '?'}`,
        entrance: isEntranceFrames(frames, translatePx),
        animates: [...new Set(Object.values(frames).flatMap((f) => Object.keys(f)))],
      });
    });
  }
  return list;
}

const MOTION_PROPS = /^(transform|translate|scale|rotate|top|bottom|left|right|inset(-.*)?|margin(-.*)?|box-shadow|--tw-(translate|scale|rotate|skew)[a-z-]*|--tw-shadow)$/i;

function allowedHoverMotion(decl) {
  const prop = decl.prop.toLowerCase();
  const v = decl.value.replace(/\s+/g, ' ').trim().toLowerCase();
  const scale103 = /^(1\.03|103%)( (1\.03|103%))?$/;
  if ((prop === 'scale' || /^--tw-scale-[xyz]$/.test(prop)) && scale103.test(v)) return true;
  if (prop === 'transform' && /^scale\((1\.03|103%)\)$/.test(v)) return true;
  if (prop === 'translate' && /^0(px)? -4px$/.test(v)) return true;
  if (prop === 'transform' && /^translatey\(-4px\)$|^translate\(0(px)?, ?-4px\)$/.test(v)) return true;
  if (prop === '--tw-translate-y' && (v === '-4px' || /calc\(var\(--spacing\) \* -1\)/.test(v))) return true;
  if (/^--tw-translate-x$/.test(prop) && /^0(px)?$/.test(v)) return true;
  // var()-valued shorthands take their values from the --tw-* properties checked above.
  if (/^(scale|translate|rotate|transform)$/.test(prop) && /^var\(--tw-/.test(v)) return 'deferred';
  return false;
}

/**
 * Rule 21: inside a :hover rule, only colour may change; the two named exceptions (a 3 %
 * image scale and a 4 px caption rise) must sit under prefers-reduced-motion: no-preference.
 */
export function hoverMotion(sheets) {
  const found = [];
  eachDecl(sheets, (decl, sheet) => {
    if (!MOTION_PROPS.test(decl.prop)) return;
    const selectors = `${selectorChain(decl)}`;
    if (!/:hover/.test(selectors)) return;
    const ok = allowedHoverMotion(decl);
    if (ok === 'deferred') return;
    const motionSafe = /prefers-reduced-motion:\s*no-preference/i.test(mediaChain(decl));
    if (ok && motionSafe) return;
    if (/^box-shadow$|^--tw-shadow$/i.test(decl.prop) && /^(none|0 0 #0000|0 0 0 0 transparent)$/i.test(decl.value.trim())) return;
    found.push(`${where(sheet, decl)}${ok && !motionSafe ? ' (allowed transform, but not under prefers-reduced-motion: no-preference)' : ''}`);
  });
  return found;
}

/** Rule 20: the animation and animation-name declarations that run one of `names`. */
export function entranceUses(sheets, names) {
  const uses = [];
  if (!names.length) return uses;
  const re = new RegExp(`(^|[\\s,])(${names.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})([\\s,]|$)`);
  eachDecl(sheets, (decl, sheet) => {
    if (/^animation(-name)?$/i.test(decl.prop) && re.test(decl.value)) {
      // The duration and easing may sit in sibling declarations of the same rule.
      const siblings = decl.parent.nodes.filter((n) => n.type === 'decl' && /^animation/i.test(n.prop)).map((n) => n.value);
      uses.push({ value: siblings.join(' '), where: where(sheet, decl) });
    }
  });
  return uses;
}

/** Rule 20: some rule binds an animation to the view timeline (scroll-driven reveals). */
export function viewTimelines(sheets) {
  let found = false;
  eachDecl(sheets, (decl) => {
    if (/^animation-timeline$/i.test(decl.prop) && /view\(/i.test(decl.value)) found = true;
    if (/^animation$/i.test(decl.prop) && /\bview\(/i.test(decl.value)) found = true;
  });
  return found;
}

/** Rules 16 and 20: prefers-reduced-motion: reduce turns animations off. */
export function reducedMotionOff(sheets) {
  let found = false;
  eachDecl(sheets, (decl) => {
    if (!/prefers-reduced-motion:\s*reduce/i.test(mediaChain(decl))) return;
    if (/^animation(-name)?$/i.test(decl.prop) && /^none\b/i.test(decl.value.trim())) found = true;
  });
  return found;
}

/** Rule 18: no raster image used as decoration through CSS. */
export function rasterBackgrounds(sheets) {
  const found = [];
  const raster = /url\(\s*["']?(data:image\/(png|jpe?g|gif|webp|avif)|[^)"']+\.(png|jpe?g|gif|webp|avif)(\?[^)"']*)?["']?\s*\))/i;
  eachDecl(sheets, (decl, sheet) => {
    if (/^(background(-image)?|content|border-image(-source)?|list-style(-image)?|cursor)$/i.test(decl.prop) && raster.test(decl.value)) {
      found.push(where(sheet, decl));
    }
  });
  return found;
}

const COLOUR_PROPS = /(color|background|border|outline|shadow|fill|stroke|caret|accent|column-rule|text-decoration|^--)/i;

/** Every concrete colour written in the shipped CSS: { value, rgb, where }. */
export function colourUses(sheets) {
  const uses = [];
  eachDecl(sheets, (decl, sheet) => {
    if (!COLOUR_PROPS.test(decl.prop)) return;
    for (const lit of colourLiterals(decl.value)) {
      const rgb = parseColor(lit);
      if (rgb) uses.push({ value: lit, rgb, where: where(sheet, decl) });
    }
  });
  return uses;
}

/**
 * The 8.2 token file, if it exists. Looked up as $JARROW_TOKENS_CSS, then site/tokens.css, then
 * site/src/styles/tokens.css when that file already carries the night-blue page colour.
 */
export function findTokensFile(siteRoot, pageColour = '#0b0f1f') {
  const candidates = [process.env.JARROW_TOKENS_CSS, join(siteRoot, 'tokens.css'), join(siteRoot, 'src', 'styles', 'tokens.css')].filter(Boolean);
  for (const file of candidates) {
    if (!existsSync(file)) continue;
    const css = readFileSync(file, 'utf8');
    if (file === candidates.at(-1) && !css.toLowerCase().includes(pageColour)) continue;
    return { file, css };
  }
  return null;
}

/** Custom property name -> colour, for every colour-valued custom property in a token file. */
export function tokenColours(css) {
  const tokens = new Map();
  parseCss(css, 'tokens.css').walkDecls(/^--/, (decl) => {
    const c = parseColor(decl.value);
    if (c) tokens.set(decl.prop, c);
  });
  return tokens;
}

/** Colour uses whose RGB matches no token (any alpha of a token colour is allowed). */
export function coloursOutsideTokens(uses, tokens) {
  const allowed = [...tokens.values()];
  const bad = new Map();
  for (const use of uses) {
    if (use.rgb.a === 0) continue;
    if (allowed.some((t) => sameRgb(t, use.rgb, 0.6))) continue;
    const hex = toHex(use.rgb);
    if (!bad.has(hex)) bad.set(hex, use);
  }
  return [...bad].map(([hex, use]) => `${hex} (${use.value}) at ${use.where}`);
}
