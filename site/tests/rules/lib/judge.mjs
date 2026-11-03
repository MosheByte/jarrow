// Judges the rendered measurements (lib/probe.mjs, lib/browser.mjs) against the thirty rules.
// Each judge takes the samples - one per page and width - and returns
// { violations: string[], checked: number, note?: string }. A judge with checked === 0 found
// nothing its rule applies to, and the test reports it as skipped rather than passed.

import { contrast, over, parseColor, sameRgb, toHex } from './color.mjs';
import { SPEC, pixelSize } from './spec.mjs';

const C = Object.fromEntries(Object.entries(SPEC.colours).map(([k, v]) => [k, Array.isArray(v) ? v.map(parseColor) : parseColor(v)]));
const PAGE = C.page;
const WHITE = C.white;

/** Brands the site names (the report's section 7 and the loaders), for rule 22. */
const BRANDS = ['GitHub Sponsors', 'GitHub', 'Discord', 'Modrinth', 'CurseForge', 'NeoForge', 'Fabric', 'Forge', 'Quilt', 'Minecraft', 'Mojang', 'Microsoft', 'Windows', 'Linux', 'macOS', 'Apple', 'Rust', 'Java', 'Prism Launcher', 'MultiMC', 'ATLauncher', 'Cloudflare', 'Anthropic', 'Claude', 'npm', 'crates.io', 'Codecov', 'Sentry', 'Bluesky', 'Mastodon', 'YouTube', 'Reddit'];

const near = (a, b, tol) => a !== null && a !== undefined && Math.abs(a - b) <= tol;
const firstFamily = (ff) => (ff ?? '').split(',')[0].replace(/["']/g, '').trim();
const isFamily = (ff, name) => {
  const f = firstFamily(ff);
  if (name === SPEC.fonts.body) return /^Geist( Variable)?$/i.test(f);
  return f.toLowerCase().startsWith(name.toLowerCase());
};
const at = (s) => `${s.route} at ${s.width}`;
const result = (violations, checked, note) => ({ violations, checked, ...(note ? { note } : {}) });
const heroPages = (samples) => samples.filter((s) => !s.probe.isDocs && s.status !== 404);

/** The opaque colour behind an element: its own and its ancestors' backgrounds over white canvas. */
export function backgroundOf(layers) {
  let bg = { ...WHITE };
  for (const layer of [...layers].reverse()) {
    const c = parseColor(layer.color);
    if (c && c.a > 0) bg = over(c, bg);
  }
  return bg;
}

/** The colour text is drawn in, after its alpha and its ancestors' opacity. */
function textColour(t) {
  const c = parseColor(t.color);
  if (!c) return null;
  return { ...c, a: c.a * (t.opacity ?? 1) };
}

/** If an opaque colour equals white at some alpha over the page, that alpha; else null. */
export function whiteStep(c) {
  if (c.a < 1) {
    if (sameRgb(c, WHITE, 2) || sameRgb(c, C.foreground, 2)) return c.a;
    return null;
  }
  const alphas = ['r', 'g', 'b'].map((k) => (c[k] - PAGE[k]) / (255 - PAGE[k]));
  if (Math.max(...alphas) - Math.min(...alphas) > 0.04) return null;
  return alphas.reduce((a, b) => a + b, 0) / 3;
}

const stepOk = (alpha) => SPEC.textAlphas.some((a) => Math.abs(a - alpha) <= 0.025);

function gradientStops(image) {
  return [...(image ?? '').matchAll(/(rgba?\([^)]*\)|#[0-9a-f]{3,8}|oklch\([^)]*\)|color\([^)]*\))/gi)].map((m) => parseColor(m[1])).filter(Boolean);
}

function informative(t) {
  return !t.srOnly && !t.ariaHidden && t.text.length > 0;
}

/* ---------- the judges, by rule number ---------- */

export const JUDGES = {
  1: (samples) => {
    const v = [];
    for (const s of samples) {
      const p = s.probe;
      const bodyBg = parseColor(p.body.bg);
      const htmlBg = parseColor(p.html.bg);
      const shown = bodyBg && bodyBg.a > 0 ? bodyBg : htmlBg;
      if (!shown || shown.a === 0 || !sameRgb(shown, PAGE, 1.5)) v.push(`${at(s)}: page background is ${shown ? toHex(shown) : p.body.bg}, not ${SPEC.colours.page}`);
      if (!/^dark$/.test((p.html.colorScheme ?? '').trim())) v.push(`${at(s)}: color-scheme is "${p.html.colorScheme}", not "dark"`);
      const l = s.light;
      if (l) {
        const lb = parseColor(l.bodyBg)?.a > 0 ? parseColor(l.bodyBg) : parseColor(l.htmlBg);
        if (!lb || lb.a === 0 || !sameRgb(lb, PAGE, 1.5)) v.push(`${at(s)}: with the system set to light the page turns ${lb ? toHex(lb) : l.bodyBg}: there is a light mode`);
        if (!/^dark$/.test((l.colorScheme ?? '').trim())) v.push(`${at(s)}: with the system set to light, color-scheme is "${l.colorScheme}"`);
      }
    }
    return result(v, samples.length);
  },

  2: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      const p = s.probe;
      for (const h of p.headings.filter((h) => h.tag !== 'h3')) {
        checked++;
        if (!isFamily(h.fontFamily, SPEC.fonts.display)) v.push(`${at(s)}: ${h.tag} "${h.text}" is set in ${firstFamily(h.fontFamily)}`);
        else if (h.fontWeight !== SPEC.fonts.displayWeight) v.push(`${at(s)}: ${h.tag} "${h.text}" weight ${h.fontWeight}, not 600`);
      }
      for (const t of p.texts.filter(informative)) {
        if (t.inDisplayHeading || t.gradient) continue;
        checked++;
        if (t.code) {
          if (!isFamily(t.fontFamily, SPEC.fonts.mono)) v.push(`${at(s)}: code "${t.text}" is set in ${firstFamily(t.fontFamily)}`);
        } else if (t.displayNumber) {
          if (!isFamily(t.fontFamily, SPEC.fonts.display)) v.push(`${at(s)}: display number "${t.text}" is set in ${firstFamily(t.fontFamily)}`);
        } else if (!isFamily(t.fontFamily, SPEC.fonts.body)) {
          v.push(`${at(s)}: ${t.tag} "${t.text}" is set in ${firstFamily(t.fontFamily)}, not Geist`);
        }
      }
      const families = new Set(p.fontsLoaded.map((f) => f.toLowerCase()));
      for (const name of [SPEC.fonts.display, SPEC.fonts.body]) {
        if (![...families].some((f) => f.startsWith(name.toLowerCase()))) v.push(`${at(s)}: no ${name} font face loaded`);
      }
    }
    return result(dedupe(v), checked);
  },

  3: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of heroPages(samples)) {
      const h1s = s.probe.headings.filter((h) => h.tag === 'h1');
      if (!h1s.length) {
        v.push(`${at(s)}: no h1`);
        continue;
      }
      for (const h of h1s) {
        checked++;
        const target = SPEC.h1[s.width];
        if (!near(h.fontSize, target, SPEC.h1.tolerancePx)) v.push(`${at(s)}: h1 "${h.text}" is ${h.fontSize} px, not ${target} px`);
        if (h.lineHeight && h.lineHeight / h.fontSize > SPEC.h1.maxLineHeight + 0.005) v.push(`${at(s)}: h1 line height ${(h.lineHeight / h.fontSize).toFixed(3)}, over ${SPEC.h1.maxLineHeight}`);
        if (!near(h.letterSpacing / h.fontSize, SPEC.h1.letterSpacingEm, 0.004)) v.push(`${at(s)}: h1 letter spacing ${(h.letterSpacing / h.fontSize).toFixed(3)} em, not ${SPEC.h1.letterSpacingEm} em`);
      }
    }
    return result(v, checked);
  },

  4: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of heroPages(samples)) {
      for (const h of s.probe.headings.filter((h) => h.tag === 'h2' && h.inMain && !h.inCard && !h.inDocs)) {
        checked++;
        const target = SPEC.h2[s.width];
        if (!near(h.fontSize, target, SPEC.h2.tolerancePx)) v.push(`${at(s)}: h2 "${h.text}" is ${h.fontSize} px, not ${target} px`);
        if (h.lineHeight && !near(h.lineHeight / h.fontSize, SPEC.h2.lineHeight, 0.012)) v.push(`${at(s)}: h2 "${h.text}" line height ${(h.lineHeight / h.fontSize).toFixed(3)}, not ${SPEC.h2.lineHeight}`);
        if (!near(h.letterSpacing / h.fontSize, SPEC.h2.letterSpacingEm, 0.004)) v.push(`${at(s)}: h2 "${h.text}" letter spacing ${(h.letterSpacing / h.fontSize).toFixed(3)} em, not ${SPEC.h2.letterSpacingEm} em`);
      }
    }
    return result(v, checked);
  },

  5: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const p of s.probe.paragraphs.filter((p) => p.words > 0)) {
        checked++;
        const ratio = p.lineHeight ? p.lineHeight / p.fontSize : null;
        if (p.inDocs) {
          if (!near(p.fontSize, SPEC.docsBody.px, 0.25)) v.push(`${at(s)}: docs paragraph "${p.text.slice(0, 40)}" is ${p.fontSize} px, not 16 px`);
          else if (!near(ratio, SPEC.docsBody.lineHeight, 0.012)) v.push(`${at(s)}: docs paragraph "${p.text.slice(0, 40)}" line height ${ratio?.toFixed(3) ?? 'normal'}, not 1.7`);
          continue;
        }
        if (p.fontSize >= SPEC.body.minPx - 0.25 && p.fontSize <= SPEC.body.maxPx + 0.25) {
          if (!near(ratio, SPEC.body.lineHeight, 0.012)) v.push(`${at(s)}: paragraph "${p.text.slice(0, 40)}" line height ${ratio?.toFixed(3) ?? 'normal'}, not 1.625`);
          if (!isFamily(p.fontFamily, SPEC.fonts.body)) v.push(`${at(s)}: paragraph "${p.text.slice(0, 40)}" is set in ${firstFamily(p.fontFamily)}`);
        } else if (!(near(p.fontSize, 12, 0.25) || near(p.fontSize, 14, 0.25))) {
          v.push(`${at(s)}: paragraph "${p.text.slice(0, 40)}" is ${p.fontSize} px: body text is 15 to 18 px, small text 12 or 14 px`);
        }
      }
    }
    return result(v, checked);
  },

  6: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const t of s.probe.texts.filter(informative)) {
        checked++;
        if (/uppercase|capitalize/.test(t.textTransform)) v.push(`${at(s)}: ${t.tag} "${t.text}" has text-transform ${t.textTransform}`);
        if (/small-caps|petite-caps|unicase|titling-caps/.test(t.fontVariantCaps ?? '')) v.push(`${at(s)}: ${t.tag} "${t.text}" has font-variant-caps ${t.fontVariantCaps}`);
      }
    }
    return result(dedupe(v), checked);
  },

  7: (samples) => {
    const v = [];
    let checked = 0;
    const hues = [C.foreground, C.accent, C.primaryText, C.page, C.tierGreen, C.tierAmber, C.tierRed, ...C.gradient];
    for (const s of samples) {
      for (const t of s.probe.texts.filter((t) => informative(t) && !t.gradient && !t.code)) {
        const c = textColour(t);
        if (!c) continue;
        checked++;
        if (c.a < 0.5 - 0.01) {
          v.push(`${at(s)}: ${t.tag} "${t.text}" is drawn at ${Math.round(c.a * 100)} % alpha, below 50 %`);
          continue;
        }
        const step = whiteStep(c);
        if (step !== null) {
          if (!stepOk(step)) v.push(`${at(s)}: ${t.tag} "${t.text}" is white at ${Math.round(step * 100)} %, not one of 100/80/70/65/60/50 %`);
          if (step < 0.5 - 0.01) v.push(`${at(s)}: ${t.tag} "${t.text}" is white at ${Math.round(step * 100)} %, below 50 %`);
          continue;
        }
        if (!hues.some((h) => sameRgb(h, c, 3))) v.push(`${at(s)}: ${t.tag} "${t.text}" is ${toHex(c)}, not a text step or a token colour`);
      }
    }
    return result(dedupe(v), checked);
  },

  8: (samples) => {
    const v = [];
    let checked = 0;
    let primaries = 0;
    for (const s of samples) {
      const p = s.probe;
      for (const b of p.buttons) {
        const bg = parseColor(b.bg);
        if (!bg || bg.a < 0.99 || !sameRgb(bg, C.primary, 2)) continue;
        primaries++;
        checked++;
        const fg = parseColor(b.color);
        if (!fg || !sameRgb(fg, C.primaryText, 2)) v.push(`${at(s)}: primary button "${b.text}" text is ${fg ? toHex(fg) : b.color}, not ${SPEC.colours.primaryText}`);
        const hover = p.hover.find((h) => h.id === b.id)?.bg;
        const hc = parseColor(hover);
        if (!hc || !sameRgb(hc, C.primaryHover, 2)) v.push(`${at(s)}: primary button "${b.text}" on hover is ${hc ? toHex(hc) : hover}, not ${SPEC.colours.primaryHover}`);
      }
      for (const b of p.buttons) {
        const bg = parseColor(b.bg);
        if (bg && bg.a > 0.99 && sameRgb(bg, C.tierAmber, 2)) v.push(`${at(s)}: button "${b.text}" uses the tier amber ${SPEC.colours.tierAmber}; the primary is ${SPEC.colours.primary}`);
      }
      for (const t of p.texts.filter(informative)) {
        const c = textColour(t);
        if (!c) continue;
        for (const [name, tier] of [['green', C.tierGreen], ['amber', C.tierAmber], ['red', C.tierRed]]) {
          if (sameRgb(c, tier, 2) && !t.tier) v.push(`${at(s)}: ${t.tag} "${t.text}" uses the tier ${name} outside a [data-tier] result`);
        }
      }
    }
    if (!samples.some((s) => s.route === '/' && s.probe.buttons.some((b) => sameRgb(parseColor(b.bg) ?? PAGE, C.primary, 2)))) {
      v.push(`/: no primary button in ${SPEC.colours.primary}`);
    }
    return result(dedupe(v), Math.max(checked, primaries, 1));
  },

  9: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const g of s.probe.gradientText) {
        checked++;
        if (!g.inHeading && !g.number) v.push(`${at(s)}: gradient text "${g.text}" outside a heading or display number`);
        const stops = gradientStops(g.image);
        const want = C.gradient;
        if (stops.length < 3 || !want.every((w) => stops.some((st) => sameRgb(st, w, 3)))) v.push(`${at(s)}: gradient "${g.text}" is ${g.image.slice(0, 120)}, not ${SPEC.colours.gradient.join(' -> ')}`);
      }
      for (const h of s.probe.headings) {
        const spans = h.gradients.filter((g) => !g.self);
        if (spans.length > 1) v.push(`${at(s)}: ${h.tag} "${h.text}" has ${spans.length} gradient spans`);
        if (h.gradients.some((g) => g.self)) v.push(`${at(s)}: the whole ${h.tag} "${h.text}" is gradient text`);
      }
    }
    return result(dedupe(v), checked || samples.length);
  },

  10: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      checked++;
      for (const o of s.probe.spacing.slice(0, 15)) v.push(`${at(s)}: ${o.path} ${o.prop} ${o.value} px is not a multiple of 4 px`);
      if (s.probe.spacing.length > 15) v.push(`${at(s)}: ... and ${s.probe.spacing.length - 15} more lengths that are not multiples of 4 px`);
      if (!s.probe.isDocs && s.probe.h1Left !== null && s.status !== 404) {
        const want = s.width >= SPEC.container.wideFrom ? (s.width - SPEC.container.width) / 2 : SPEC.container.gutterNarrow;
        if (!near(s.probe.h1Left, want, 1)) v.push(`${at(s)}: the h1 starts at x = ${Math.round(s.probe.h1Left)} px; a 1152 px column starts at ${want} px`);
      }
    }
    return result(v, checked);
  },

  11: (samples) => {
    const v = [];
    for (const s of samples) {
      const h = s.probe.header;
      if (!h) {
        v.push(`${at(s)}: no site header`);
        continue;
      }
      if (!near(h.height, SPEC.header.height, 0.5)) v.push(`${at(s)}: header is ${Math.round(h.height * 10) / 10} px tall, not 72 px`);
      if (h.position !== 'fixed') v.push(`${at(s)}: header position is ${h.position}, not fixed`);
      const top = parseColor(h.bg);
      if (top && top.a > 0.01) v.push(`${at(s)}: header background at the top is ${h.bg}, not transparent`);
      const sc = s.probe.headerScrolled;
      if (sc) {
        const c = parseColor(sc.bg);
        if (!c || !sameRgb(c, PAGE, 2) || !near(c.a, SPEC.header.scrolledAlpha, 0.02)) v.push(`${at(s)}: header after scrolling is ${sc.bg}, not ${SPEC.colours.page} at 75 %`);
        if (!/blur\(12px\)/.test(sc.backdrop ?? '')) v.push(`${at(s)}: header backdrop after scrolling is "${sc.backdrop}", not blur(12px)`);
      }
    }
    return result(v, samples.length);
  },

  12: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const c of s.probe.cards) {
        checked++;
        const label = `${at(s)}: card ${c.path}${c.marked ? '' : ' (unmarked)'}`;
        const borders = c.borders.filter((b) => b.w > 0 && b.style !== 'none' && (parseColor(b.color)?.a ?? 0) > 0);
        if (borders.length) v.push(`${label} has a ${borders[0].w} px ${borders[0].style} border`);
        if (c.boxShadow !== 'none') v.push(`${label} has a box-shadow`);
        const bg = parseColor(c.bg);
        const alpha = bg ? (bg.a < 1 && sameRgb(bg, WHITE, 2) ? bg.a : whiteStep(bg)) : null;
        if (alpha === null || alpha < SPEC.card.alphaMin - 0.005 || alpha > SPEC.card.alphaMax + 0.005) v.push(`${label} surface is ${c.bg}, not white at 3 to 8 %`);
        if (!near(c.radius, SPEC.card.radius, 0.5)) v.push(`${label} radius is ${c.radius} px, not 16 px`);
        if (c.padding.some((p) => p < SPEC.card.paddingMin - 0.5 || p > SPEC.card.paddingMax + 0.5)) v.push(`${label} padding is ${c.padding.join('/')} px, not 24 to 32 px`);
      }
      for (const sh of s.probe.shadows) {
        // A glow (no offset) is allowed on pills and live dots; a drop shadow is not.
        const offsets = [...sh.boxShadow.matchAll(/(-?\d+(\.\d+)?)px\s+(-?\d+(\.\d+)?)px/g)].map((m) => [parseFloat(m[1]), parseFloat(m[3])]);
        if (offsets.some(([x, y]) => x !== 0 || y !== 0)) v.push(`${at(s)}: ${sh.path} has a drop shadow (${sh.boxShadow.slice(0, 80)})`);
      }
    }
    return result(dedupe(v), checked || samples.length);
  },

  13: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const b of s.probe.buttons) {
        checked++;
        const label = `${at(s)}: button "${b.text || b.path}"`;
        if (!SPEC.buttons.heights.some((h) => near(b.height, h, 0.5))) v.push(`${label} is ${Math.round(b.height * 10) / 10} px tall, not 48, 40 or 36 px`);
        if (!SPEC.buttons.radii.some((r) => near(b.radius, r, 0.25))) v.push(`${label} radius ${b.radius} px, not 10 or 8 px`);
        if (!SPEC.buttons.fontPx.some((f) => near(b.fontSize, f, 0.25))) v.push(`${label} text ${b.fontSize} px, not 15 or 14 px`);
        if (b.fontWeight !== SPEC.buttons.weight) v.push(`${label} weight ${b.fontWeight}, not 500`);
        for (const i of b.icons) if (i.w < SPEC.buttons.iconMinPx - 0.5 || i.w > SPEC.buttons.iconMaxPx + 0.5) v.push(`${label} icon ${Math.round(i.w)} px, not 17 to 18 px`);
        if (b.icons.length && b.text && !near(b.gap, SPEC.buttons.gapPx, 0.25)) v.push(`${label} icon gap ${b.gap ?? 'none'}, not 10 px`);
      }
    }
    return result(dedupe(v), checked);
  },

  14: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const n of s.probe.numbers) {
        checked++;
        if (!/tabular-nums/.test(n.fontVariantNumeric)) v.push(`${at(s)}: ${n.marked ? 'number' : 'display number'} "${n.text}" (${n.path}) has no tabular-nums`);
      }
      for (const u of s.probe.units) {
        checked++;
        if (!near(u.fontSize, SPEC.unit.px, 0.25)) v.push(`${at(s)}: unit "${u.text}" is ${u.fontSize} px, not 14 px`);
        const c = parseColor(u.color);
        const step = c ? whiteStep(c) : null;
        if (step === null || !near(step, SPEC.unit.alpha, 0.025)) v.push(`${at(s)}: unit "${u.text}" colour ${u.color}, not white at 50 %`);
      }
    }
    return result(dedupe(v), checked);
  },

  15: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      const canvases = s.probe.canvases;
      const needsScene = (s.route === '/' || s.route === '/board/') && s.status !== 404;
      if (needsScene && !canvases.length) v.push(`${at(s)}: no scene canvas (the ${s.route === '/' ? 'sunrise' : 'ember field'})`);
      if (!canvases.length) continue;
      checked++;
      const pixel = canvases.filter((c) => c.imageRendering === 'pixelated');
      if (!pixel.length) v.push(`${at(s)}: no canvas with image-rendering: pixelated`);
      const want = pixelSize(s.width);
      for (const c of pixel) {
        const cell = c.rect.w / c.attrW;
        if (!near(cell, want, 0.15)) v.push(`${at(s)}: ${c.path} is ${c.attrW} px wide for ${Math.round(c.rect.w)} CSS px (cell ${cell.toFixed(2)} px), not ${want} px cells`);
      }
    }
    return result(v, checked || samples.filter((s) => s.route === '/' || s.route === '/board/').length);
  },

  16: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      const canvases = s.probe.canvases;
      if (!canvases.length) continue;
      checked++;
      canvases.forEach((c, i) => {
        if (!c.ariaHidden) v.push(`${at(s)}: ${c.path} is not aria-hidden`);
        const fades = [c.transition.self, c.transition.parent].filter(Boolean).some((t) => /opacity|all/.test(t.property) && /(^|,\s*)0\.7s/.test(t.duration));
        if (!fades) v.push(`${at(s)}: ${c.path} has no 700 ms opacity fade-in`);
        const fps = (s.probe.sceneFrames[i]?.frames ?? 0) / 2;
        if (fps > SPEC.scene.maxFps + 1) v.push(`${at(s)}: ${c.path} draws ${fps} frames per second on screen, over 32`);
        const off = s.probe.offscreenFrames?.[i]?.frames;
        if (off !== undefined && off > 0) v.push(`${at(s)}: ${c.path} drew ${off} frames while off screen`);
      });
      const r = s.reduced;
      if (r) {
        const drawn = r.frames.reduce((n, f) => n + f.frames, 0);
        if (drawn > 0) v.push(`${at(s)}: under reduced motion the scenes drew ${drawn} frames in 2 s; they must hold one static frame`);
        if (r.canvases && r.drewBefore === 0) v.push(`${at(s)}: under reduced motion the scenes drew nothing at all`);
      }
    }
    return result(v, checked);
  },

  17: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      const hero = s.probe.hero;
      if (!hero || !hero.canvases.length || s.probe.isDocs) continue;
      checked++;
      const scene = hero.canvases.reduce((a, b) => (b.h * b.w > a.h * a.w ? b : a));
      if (s.width === 1440) {
        const band = hero.rect.bottom - SPEC.scene.band1440;
        if (hero.contentBottom !== null && hero.contentBottom > band + 1) v.push(`${at(s)}: hero copy reaches y = ${Math.round(hero.contentBottom)} px, inside the bottom 230 px of the hero (from y = ${Math.round(band)} px)`);
      } else {
        if (!near(scene.h, SPEC.scene.strip390, 3)) v.push(`${at(s)}: the scene strip is ${Math.round(scene.h)} px tall, not 253 px`);
        if (hero.contentBottom !== null && scene.y < hero.contentBottom - 1) v.push(`${at(s)}: the scene starts at y = ${Math.round(scene.y)} px, above the end of the hero copy (${Math.round(hero.contentBottom)} px)`);
      }
    }
    return result(v, checked);
  },

  18: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const i of s.probe.imgs) {
        checked++;
        if (i.alt === null) v.push(`${at(s)}: <img src="${i.src}"> has no alt attribute`);
        else if (i.w >= 200 && i.alt.trim().split(/\s+/).length < 4) v.push(`${at(s)}: <img src="${i.src}"> (shown ${Math.round(i.w)} px wide) has alt "${i.alt}", not a full sentence`);
      }
    }
    return result(dedupe(v), checked);
  },

  19: (samples) => {
    const v = [];
    for (const s of samples) {
      for (const b of s.probe.borders) {
        const c = parseColor(b.color);
        const hairline = b.w <= 1.01 && b.style === 'solid' && c && sameRgb(c, WHITE, 2) && near(c.a, 0.08, 0.021);
        const quote = b.tag === 'blockquote' && b.side === 'left' && b.w <= 2.01;
        if (!hairline && !quote) v.push(`${at(s)}: ${b.path} border-${b.side} ${b.w} px ${b.style} ${b.color}; separators are 1 px white at 8 %`);
      }
      for (const svg of s.probe.svgs) {
        if (svg.w >= 0.9 * s.width && svg.h <= 240 && !svg.brandSource) v.push(`${at(s)}: ${svg.path} is a full-width SVG (${Math.round(svg.w)} x ${Math.round(svg.h)} px), the shape of a wave divider`);
      }
      for (const m of s.probe.masks ?? []) {
        v.push(`${at(s)}: ${m.path} is a full-width element cut by ${m.mask !== 'none' && m.mask ? `mask ${m.mask}` : `clip-path ${m.clipPath}`}; section edges are the scene's own falloff`);
      }
    }
    return result(dedupe(v), samples.length);
  },

  20: (samples, { entranceNames = [] } = {}) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const a of s.probe.animations) {
        checked++;
        const names = a.name.split(',').map((n) => n.trim());
        for (const name of names) if (!entranceNames.includes(name)) v.push(`${at(s)}: ${a.path} runs animation "${name}", not the one entrance keyframe`);
        if (!/^0\.9s(,\s*0\.9s)*$/.test(a.duration)) v.push(`${at(s)}: ${a.path} animation lasts ${a.duration}, not 0.9 s`);
        if (!a.timing.replace(/\s/g, '').includes('cubic-bezier(0.2,0.7,0.2,1)')) v.push(`${at(s)}: ${a.path} eases with ${a.timing}`);
        if (/infinite/.test(a.iteration)) v.push(`${at(s)}: ${a.path} loops forever; only the scene canvases loop`);
        if (a.belowFold && !/view/.test(a.timeline ?? '')) v.push(`${at(s)}: ${a.path} is below the fold but not bound to the view timeline`);
      }
      if (s.reduced && s.reduced.runningAnimations > 0) v.push(`${at(s)}: ${s.reduced.runningAnimations} animations still run under prefers-reduced-motion: reduce`);
    }
    return result(dedupe(v), checked || samples.length);
  },

  21: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const el of [...s.probe.buttons, ...s.probe.links.filter((l) => l.inNav || l.inHeader)]) {
        checked++;
        const props = el.transitionProperty.split(',').map((p) => p.trim());
        const durs = el.transitionDuration.split(',').map((d) => d.trim());
        const colourIdx = props.findIndex((p) => /^(all|color|background-color|background|border-color|opacity)$/.test(p) || p.startsWith('--tw'));
        const dur = colourIdx === -1 ? null : durs[colourIdx % durs.length];
        if (dur !== '0.15s') v.push(`${at(s)}: "${el.text || el.path}" changes colour over ${dur ?? 'no transition'}, not 150 ms`);
      }
    }
    return result(dedupe(v), checked);
  },

  22: (samples, { brandSources = null } = {}) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const svg of s.probe.svgs) {
        if (svg.brandSource !== null) {
          checked++;
          if (!/^https:\/\//.test(svg.brandSource)) v.push(`${at(s)}: brand mark ${svg.path} has data-brand-source "${svg.brandSource}", not an https URL`);
          if (!svg.ariaHidden) v.push(`${at(s)}: brand mark ${svg.path} is not aria-hidden`);
          if (!svg.nearText) v.push(`${at(s)}: brand mark ${svg.path} has no brand name beside it`);
          if (brandSources !== null && /^https:\/\//.test(svg.brandSource) && !brandSources.includes(svg.brandSource)) v.push(`${at(s)}: ${svg.brandSource} is not recorded in site/BRAND-SOURCES.md`);
        } else if (!svg.ariaHidden && !svg.role && !svg.ariaLabel) {
          checked++;
          v.push(`${at(s)}: inline SVG ${svg.path} is neither a brand mark with data-brand-source nor an aria-hidden icon`);
        } else if (svg.nearText.length <= 80) {
          // An unmarked glyph beside a brand's name stands in for the brand's mark.
          const brand = BRANDS.find((b) => new RegExp(`(^|[^A-Za-z])${b}([^A-Za-z]|$)`).test(svg.nearText));
          if (brand) {
            checked++;
            v.push(`${at(s)}: ${svg.path} beside "${svg.nearText}" has no data-brand-source: a glyph standing in for the ${brand} mark`);
          }
        }
      }
    }
    return result(dedupe(v), checked);
  },

  23: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const l of s.probe.links) {
        if (l.target === '_blank') {
          checked++;
          if (!l.srNewTab) v.push(`${at(s)}: "${l.text || l.href}" opens a new tab without a visually hidden "(opens in a new tab)"`);
        }
        const outbound = /^https?:$/.test(l.protocol) && !l.internal;
        if (!outbound || l.buttonLike || l.brandMark || !l.text) continue;
        checked++;
        if (l.target !== '_blank') v.push(`${at(s)}: outbound text link "${l.text}" (${l.host}) does not open in a new tab`);
        if (l.inBody && !l.arrow) v.push(`${at(s)}: outbound link "${l.text}" in body copy has no arrow glyph`);
      }
    }
    return result(dedupe(v), checked);
  },

  24: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const h of s.probe.headings) {
        if (s.probe.isDocs || h.inDocs) {
          checked++;
          const c = parseColor(h.color);
          if (h.gradients.length) v.push(`${at(s)}: docs ${h.tag} "${h.text}" uses the accent gradient`);
          else if (!c || !sameRgb(c, WHITE, 2) || c.a < 0.99) v.push(`${at(s)}: docs ${h.tag} "${h.text}" is ${h.color}, not white`);
          continue;
        }
        if (h.tag === 'h1' && s.width === 1440 && s.status !== 404) {
          checked++;
          if (h.lines !== 2) v.push(`${at(s)}: h1 "${h.text}" sets on ${h.lines} lines, not 2`);
          const spans = h.gradients.filter((g) => !g.self);
          if (spans.length !== 1) v.push(`${at(s)}: h1 "${h.text}" has ${spans.length} accent-gradient spans, not 1`);
          else if (!spans[0].isLast) v.push(`${at(s)}: h1 "${h.text}" accent "${spans[0].text}" is not its last clause`);
        }
        if (h.tag === 'h2' && h.inMain && !h.inCard) {
          checked++;
          const c = parseColor(h.color);
          if (c && c.a > 0 && !sameRgb(c, WHITE, 2)) v.push(`${at(s)}: h2 "${h.text}" is ${h.color}, not white`);
        }
      }
    }
    return result(dedupe(v), checked);
  },

  25: (samples) => {
    const v = [];
    const home = samples.filter((s) => s.route === '/');
    for (const s of home) {
      const hero = s.probe.hero;
      if (!hero) {
        v.push(`${at(s)}: no hero section`);
        continue;
      }
      if (!/\b\d+\.\d+(\.\d+)?\b/.test(hero.text)) v.push(`${at(s)}: the hero states no version number`);
      const figures = (hero.text.match(/\d[\d,.]*/g) ?? []).length;
      if (figures < 2) v.push(`${at(s)}: the hero states ${figures} figures; it needs a version number and a count`);
      if (!hero.lede) v.push(`${at(s)}: no lede paragraph after the h1`);
      else {
        if (hero.lede.words > SPEC.ledeMaxWords) v.push(`${at(s)}: the lede is ${hero.lede.words} words, over 25`);
        const ledeFigures = (hero.lede.text.match(/\d[\d,.]*/g) ?? []).length;
        if (ledeFigures < 2) v.push(`${at(s)}: the lede has ${ledeFigures} figures, not at least 2`);
      }
    }
    return result(dedupe(v), home.length);
  },

  27: (samples) => {
    const v = [];
    for (const s of samples) {
      const f = s.probe.footer;
      if (!f) {
        v.push(`${at(s)}: no footer`);
        continue;
      }
      const sentences = f.paragraphs.filter((p) => /[.!?]["”)]?$/.test(p));
      if (sentences.length < 3) v.push(`${at(s)}: the footer has ${sentences.length} paragraphs in sentences, not 3`);
      if (!/Not an official Minecraft product/i.test(f.text)) v.push(`${at(s)}: the footer lacks "Not an official Minecraft product."`);
      if (!/Mojang/.test(f.text)) v.push(`${at(s)}: the footer does not name Mojang`);
      if (!f.paragraphs.some((p) => /\b[a-z0-9-]+(\.[a-z0-9-]+)*\.(com|dev|net|org|io|app)\b/i.test(p))) v.push(`${at(s)}: no footer paragraph names the hostnames the page calls`);
      if (f.linkLists > 0) v.push(`${at(s)}: the footer has ${f.linkLists} link columns; it is paragraphs`);
    }
    return result(dedupe(v), samples.length);
  },

  29: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      for (const t of s.probe.texts.filter(informative)) {
        checked++;
        // Gradient text is judged by its stops, below.
        if (t.gradient) continue;
        const bg = backgroundOf(t.bg);
        const c = textColour(t);
        if (!c || c.a === 0) continue;
        const step = whiteStep(c);
        if (t.fontSize < SPEC.minTextPx - 0.01 && step !== null && step <= 0.5 + 0.01) v.push(`${at(s)}: ${t.tag} "${t.text}" is ${t.fontSize} px at ${Math.round(step * 100)} % white, under 12 px`);
        const ratio = contrast(over(c, bg), bg);
        if (ratio < SPEC.contrastMin) v.push(`${at(s)}: ${t.tag} "${t.text}" ${toHex(over(c, bg))} on ${toHex(bg)} is ${ratio.toFixed(2)}:1, under 4.9:1`);
      }
      for (const g of s.probe.gradientText) {
        for (const stop of gradientStops(g.image)) {
          const ratio = contrast(over(stop, PAGE), PAGE);
          if (ratio < SPEC.contrastMin) v.push(`${at(s)}: gradient stop ${toHex(stop)} of "${g.text}" is ${ratio.toFixed(2)}:1 on the page, under 4.9:1`);
        }
      }
    }
    return result(dedupe(v), checked);
  },

  30: (samples) => {
    const v = [];
    let checked = 0;
    for (const s of samples) {
      const n = s.nojs;
      if (!n) continue;
      checked++;
      if (s.status !== 404 && n.status >= 400) v.push(`${at(s)}: without JavaScript the page answers ${n.status}`);
      if (!n.footer) v.push(`${at(s)}: without JavaScript there is no footer`);
      if (n.navLinks < Math.min(3, s.probe.navLinks)) v.push(`${at(s)}: without JavaScript the navigation has ${n.navLinks} links (with it, ${s.probe.navLinks})`);
      if (s.probe.h1Text && n.h1Text !== s.probe.h1Text) v.push(`${at(s)}: without JavaScript the h1 is "${n.h1Text}", not "${s.probe.h1Text}"`);
      if (n.mainTextLength < 0.9 * s.probe.mainTextLength) v.push(`${at(s)}: without JavaScript <main> has ${n.mainTextLength} characters of text, under 90 % of ${s.probe.mainTextLength}`);
    }
    return result(v, checked);
  },
};

function dedupe(list) {
  return [...new Set(list)];
}

/** Rules whose judge above is the automated check. The others are judged by the reviewer. */
export const AUTOMATED = Object.keys(JUDGES).map(Number);
