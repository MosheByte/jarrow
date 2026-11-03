/** The pixel grid every scene is drawn on, and the ordered-dither matrix. */

/** Cell size in CSS pixels: clamp(round(width / 340), 3, 6). 3 at 390, 4 at 1440, 6 at 2040. */
export function pixelSize(width: number): number {
  return Math.min(6, Math.max(3, Math.round(width / 340)));
}

export interface Grid {
  /** CSS size of the scene. */
  width: number;
  height: number;
  /** Cell size in CSS pixels. */
  px: number;
  /** Size of the low-resolution buffer in cells; cols * px >= width, rows * px >= height. */
  cols: number;
  rows: number;
}

/** The grid for a scene of a CSS size. Whole cells only: the last column and row may overhang. */
export function gridFor(width: number, height: number): Grid {
  const w = Math.max(1, Math.round(width));
  const h = Math.max(1, Math.round(height));
  const px = pixelSize(w);
  return { width: w, height: h, px, cols: Math.ceil(w / px), rows: Math.ceil(h / px) };
}

/** Width below which the hero stacks: the sun centres and the sunrise becomes a strip. */
export const NARROW_BELOW = 1024;

/** Height of the sunrise strip on narrow screens: clamp(0.3 * innerHeight, 180, 320). */
export function stripHeight(innerHeight: number): number {
  return Math.min(320, Math.max(180, Math.round(0.3 * innerHeight)));
}

/**
 * The n x n Bayer matrix (n a power of two), built by the usual recursion: each quadrant is
 * the half-size matrix times four plus 0, 2, 3 or 1. Values are a permutation of 0 .. n*n-1.
 */
export function bayerMatrix(n: number): number[] {
  if (n < 1 || (n & (n - 1)) !== 0) throw new Error(`Bayer size must be a power of two: ${n}`);
  if (n === 1) return [0];
  const h = n / 2;
  const half = bayerMatrix(h);
  const offset = [0, 2, 3, 1];
  const m = new Array<number>(n * n);
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const q = (y >= h ? 2 : 0) + (x >= h ? 1 : 0);
      m[y * n + x] = 4 * half[(y % h) * h + (x % h)]! + offset[q]!;
    }
  }
  return m;
}

const BAYER8 = bayerMatrix(8);
/** Thresholds in (0, 1) for an 8x8 ordered dither, row-major. */
export const BAYER8_THRESHOLDS: readonly number[] = BAYER8.map((v) => (v + 0.5) / 64);

/** The 8x8 ordered-dither threshold at a cell, in (0, 1). */
export function bayer8(x: number, y: number): number {
  return BAYER8_THRESHOLDS[(y & 7) * 8 + (x & 7)]!;
}
