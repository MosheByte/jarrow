// Colour parsing, compositing and WCAG contrast, for the colour rules (1, 7, 8, 29) and the
// colour token check. Handles what browsers return from getComputedStyle and what CSS files
// contain: hex, rgb()/rgba(), hsl()/hsla(), oklch(), oklab(), color(srgb ...) and named colours.

const NAMED = {
  black: '#000000',
  white: '#ffffff',
  red: '#ff0000',
  green: '#008000',
  blue: '#0000ff',
  yellow: '#ffff00',
  orange: '#ffa500',
  purple: '#800080',
  gray: '#808080',
  grey: '#808080',
  silver: '#c0c0c0',
  navy: '#000080',
  teal: '#008080',
  maroon: '#800000',
  olive: '#808000',
  lime: '#00ff00',
  aqua: '#00ffff',
  cyan: '#00ffff',
  fuchsia: '#ff00ff',
  magenta: '#ff00ff',
  pink: '#ffc0cb',
  brown: '#a52a2a',
  gold: '#ffd700',
  indigo: '#4b0082',
  violet: '#ee82ee',
  coral: '#ff7f50',
  salmon: '#fa8072',
  tomato: '#ff6347',
  crimson: '#dc143c',
  orchid: '#da70d6',
  plum: '#dda0dd',
  khaki: '#f0e68c',
  ivory: '#fffff0',
  beige: '#f5f5dc',
  lavender: '#e6e6fa',
  whitesmoke: '#f5f5f5',
  gainsboro: '#dcdcdc',
  lightgray: '#d3d3d3',
  lightgrey: '#d3d3d3',
  darkgray: '#a9a9a9',
  darkgrey: '#a9a9a9',
  dimgray: '#696969',
  dimgrey: '#696969',
  rebeccapurple: '#663399',
};

/** CSS named colours this parser knows, for scanning stylesheets. */
export const NAMED_COLOURS = Object.keys(NAMED);

const clamp01 = (v) => Math.min(1, Math.max(0, v));

function num(token, scale = 1) {
  const t = token.trim();
  if (t === 'none') return 0;
  if (t.endsWith('%')) return (parseFloat(t) / 100) * scale;
  return parseFloat(t);
}

function hue(token) {
  const t = token.trim();
  if (t === 'none') return 0;
  if (t.endsWith('deg')) return parseFloat(t);
  if (t.endsWith('rad')) return (parseFloat(t) * 180) / Math.PI;
  if (t.endsWith('turn')) return parseFloat(t) * 360;
  if (t.endsWith('grad')) return parseFloat(t) * 0.9;
  return parseFloat(t);
}

/** Splits "a b c / d" or "a, b, c, d" into components and an alpha token. */
function components(inner, expected = 3) {
  let body = inner.trim();
  let alpha = null;
  const slash = body.lastIndexOf('/');
  if (slash !== -1) {
    alpha = body.slice(slash + 1).trim();
    body = body.slice(0, slash).trim();
  }
  const parts = body.includes(',') ? body.split(',').map((s) => s.trim()) : body.split(/\s+/);
  if (alpha === null && parts.length === expected + 1) alpha = parts.pop();
  return { parts, alpha };
}

function alphaOf(token) {
  if (token === null || token === undefined) return 1;
  return clamp01(num(token, 1));
}

function linearToSrgb(c) {
  const v = c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
  return clamp01(v) * 255;
}

function oklabToRgb(L, a, b) {
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ ** 3;
  const m = m_ ** 3;
  const s = s_ ** 3;
  return [
    linearToSrgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
    linearToSrgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
    linearToSrgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
  ];
}

function hslToRgb(h, s, l) {
  const k = (n) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [f(0) * 255, f(8) * 255, f(4) * 255];
}

/**
 * Parses a CSS colour. Returns { r, g, b, a } with r, g, b in 0..255 and a in 0..1, or null
 * for anything that is not a concrete colour (currentColor, var(), inherit, gradients...).
 */
export function parseColor(input) {
  if (typeof input !== 'string') return null;
  const s = input.trim().toLowerCase();
  if (s === 'transparent') return { r: 0, g: 0, b: 0, a: 0 };
  if (NAMED[s]) return parseColor(NAMED[s]);
  let m = s.match(/^#([0-9a-f]{3,8})$/);
  if (m) {
    let hex = m[1];
    if (hex.length === 3 || hex.length === 4) hex = [...hex].map((c) => c + c).join('');
    if (hex.length !== 6 && hex.length !== 8) return null;
    const v = (i) => parseInt(hex.slice(i, i + 2), 16);
    return { r: v(0), g: v(2), b: v(4), a: hex.length === 8 ? v(6) / 255 : 1 };
  }
  m = s.match(/^([a-z-]+)\((.*)\)$/);
  if (!m) return null;
  const [, fn, inner] = m;
  if (inner.includes('var(') || inner.includes('calc(') || inner.includes('from ')) return null;
  const { parts, alpha } = components(inner, fn === 'color' ? 4 : 3);
  const a = alphaOf(alpha);
  if (fn === 'rgb' || fn === 'rgba') {
    if (parts.length < 3) return null;
    const [r, g, b] = parts.map((p) => num(p, 255));
    return { r, g, b, a };
  }
  if (fn === 'hsl' || fn === 'hsla') {
    if (parts.length < 3) return null;
    const [r, g, b] = hslToRgb(hue(parts[0]), clamp01(num(parts[1], 1) / (parts[1].endsWith('%') ? 1 : 100)), clamp01(num(parts[2], 1) / (parts[2].endsWith('%') ? 1 : 100)));
    return { r, g, b, a };
  }
  if (fn === 'oklch') {
    if (parts.length < 3) return null;
    const L = num(parts[0], 1);
    const C = num(parts[1], 0.4);
    const H = (hue(parts[2]) * Math.PI) / 180;
    const [r, g, b] = oklabToRgb(L, C * Math.cos(H), C * Math.sin(H));
    return { r, g, b, a };
  }
  if (fn === 'oklab') {
    if (parts.length < 3) return null;
    const [r, g, b] = oklabToRgb(num(parts[0], 1), num(parts[1], 0.4), num(parts[2], 0.4));
    return { r, g, b, a };
  }
  if (fn === 'color') {
    const [space, ...rest] = parts;
    if (space !== 'srgb' || rest.length < 3) return null;
    const [r, g, b] = rest.map((p) => num(p, 1) * 255);
    return { r, g, b, a };
  }
  return null;
}

/** "#rrggbb" for a parsed colour, ignoring alpha. */
export function toHex({ r, g, b }) {
  return `#${[r, g, b].map((v) => Math.round(v).toString(16).padStart(2, '0')).join('')}`;
}

/** Composites a colour with alpha over an opaque background. */
export function over(fg, bg) {
  const a = fg.a;
  return {
    r: fg.r * a + bg.r * (1 - a),
    g: fg.g * a + bg.g * (1 - a),
    b: fg.b * a + bg.b * (1 - a),
    a: 1,
  };
}

function channel(v) {
  const c = v / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

export function luminance({ r, g, b }) {
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** WCAG 2 contrast ratio of two opaque colours. */
export function contrast(a, b) {
  const la = luminance(a);
  const lb = luminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

/** True when two colours are the same to within `tolerance` per 8-bit channel. */
export function sameRgb(a, b, tolerance = 1.5) {
  return Math.abs(a.r - b.r) <= tolerance && Math.abs(a.g - b.g) <= tolerance && Math.abs(a.b - b.b) <= tolerance;
}

/** Finds every colour literal in a CSS value string. */
export function colourLiterals(value) {
  const found = [];
  const re = /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|color)\([^()]*\)/g;
  for (const m of value.matchAll(re)) found.push(m[0]);
  const words = value.toLowerCase().match(/\b[a-z]+\b/g) ?? [];
  for (const w of words) if (NAMED[w]) found.push(w);
  return found;
}
