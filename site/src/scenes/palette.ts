/**
 * The scene palette: the two ramps of BUILD-BOOK 8.2 and nothing else. Every colour a scene
 * draws is one of these stops or a straight lerp between two of them (palette.test.ts and
 * scenes.test.ts check both layers of all three scenes).
 */

/** Sky ramp, 15 stops, night blue at the top to amber at the horizon. */
export const SKY = [
  '#0b0f1f',
  '#11173a',
  '#1a1e4a',
  '#2a2158',
  '#3d2a66',
  '#55326f',
  '#713b70',
  '#8f466c',
  '#ad5262',
  '#c66055',
  '#dc7447',
  '#ee8d44',
  '#f8a74a',
  '#ffbf5a',
  '#ffd27a',
] as const;

/** Sun ramp, 7 stops, pale at the top of the disc to deep amber at the bottom. */
export const SUN = ['#fff8dc', '#ffeeb0', '#ffdd88', '#ffc862', '#ffb454', '#ff9a48', '#ff7a3d'] as const;

/** Every colour a scene may start from. The page background is SKY[0]. */
export const PALETTE: readonly string[] = [...SKY, ...SUN];

export type Rgb = readonly [number, number, number];

export function hexToRgb(hex: string): Rgb {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) throw new Error(`not a #rrggbb colour: ${hex}`);
  return [parseInt(m[1]!, 16), parseInt(m[2]!, 16), parseInt(m[3]!, 16)];
}

export function rgbToHex([r, g, b]: Rgb): string {
  return `#${[r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('')}`;
}

export function rgba([r, g, b]: Rgb, alpha: number): string {
  return `rgba(${r}, ${g}, ${b}, ${Math.round(alpha * 1000) / 1000})`;
}

/** Straight lerp between two colours in 8-bit RGB, rounded per channel. */
export function lerpRgb(a: Rgb, b: Rgb, t: number): Rgb {
  const u = t <= 0 ? 0 : t >= 1 ? 1 : t;
  return [
    Math.round(a[0] + (b[0] - a[0]) * u),
    Math.round(a[1] + (b[1] - a[1]) * u),
    Math.round(a[2] + (b[2] - a[2]) * u),
  ];
}

export const SKY_RGB: readonly Rgb[] = SKY.map(hexToRgb);
export const SUN_RGB: readonly Rgb[] = SUN.map(hexToRgb);
export const PALETTE_RGB: readonly Rgb[] = PALETTE.map(hexToRgb);

/** Night colour (the page background) as RGB. */
export const NIGHT: Rgb = SKY_RGB[0]!;

/** A point on a ramp, 0 at the first stop and 1 at the last: a lerp of two adjacent stops. */
export function rampAt(ramp: readonly Rgb[], t: number): Rgb {
  const u = (t <= 0 ? 0 : t >= 1 ? 1 : t) * (ramp.length - 1);
  const i = Math.min(ramp.length - 2, Math.floor(u));
  return lerpRgb(ramp[i]!, ramp[i + 1]!, u - i);
}

/** The ember field's ramp: night to amber, the sky ramp without its palest stop. */
export const EMBER_RGB: readonly Rgb[] = SKY_RGB.slice(0, 14);

/** The aurora band's ramp: the amber-to-night subset, night to the first full amber. */
export const AURORA_RGB: readonly Rgb[] = SKY_RGB.slice(0, 12);

/**
 * True when a colour is a palette stop or lies on the straight line between two stops,
 * allowing one unit of rounding per channel. This is the palette contract of the scenes.
 */
export function isPaletteLerp(c: Rgb, palette: readonly Rgb[] = PALETTE_RGB): boolean {
  for (let i = 0; i < palette.length; i++) {
    const a = palette[i]!;
    if (Math.abs(a[0] - c[0]) <= 1 && Math.abs(a[1] - c[1]) <= 1 && Math.abs(a[2] - c[2]) <= 1) return true;
    for (let j = i + 1; j < palette.length; j++) {
      const b = palette[j]!;
      const d0 = b[0] - a[0];
      const d1 = b[1] - a[1];
      const d2 = b[2] - a[2];
      const len = d0 * d0 + d1 * d1 + d2 * d2;
      if (len === 0) continue;
      const t = ((c[0] - a[0]) * d0 + (c[1] - a[1]) * d1 + (c[2] - a[2]) * d2) / len;
      if (t < 0 || t > 1) continue;
      if (
        Math.abs(a[0] + d0 * t - c[0]) <= 1.01 &&
        Math.abs(a[1] + d1 * t - c[1]) <= 1.01 &&
        Math.abs(a[2] + d2 * t - c[2]) <= 1.01
      ) {
        return true;
      }
    }
  }
  return false;
}

/** Parses "#rrggbb", "rgb(r, g, b)" or "rgba(r, g, b, a)" into RGB, or null. */
export function parseCssColour(value: string): Rgb | null {
  const v = value.trim();
  if (v.startsWith('#')) return /^#[0-9a-f]{6}$/i.test(v) ? hexToRgb(v) : null;
  const m = /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*[\d.]+\s*)?\)$/.exec(v);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}
