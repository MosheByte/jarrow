/**
 * The ember field (BUILD-BOOK 8.3), for /board: a field of heat, densest at the bottom of the
 * hero and thinning upward, ordered-dithered onto the night-to-amber ramp. Two noise layers
 * scroll upward at different speeds so the heat rises in tongues; a few sparks climb out of it.
 */

import { type Grid, bayer8 } from './grid.ts';
import { noiseField } from './noise.ts';
import { EMBER_RGB, SKY, SKY_RGB, SUN_RGB, lerpRgb } from './palette.ts';
import type { PixelBuffer } from './pixels.ts';
import { between, hash3 } from './prng.ts';
import type { Frame, SceneDef, SmoothContext } from './runtime.ts';

/** Where the heat begins, as a fraction of the height from the top. */
export const EMBER_TOP = 0.3;

interface Spark {
  x: number;
  y: number;
  speed: number; // cells per second, upward
  sway: number;
}

export interface EmberState {
  grid: Grid;
  /** Base heat per row, 0 at EMBER_TOP and above. */
  profile: Float32Array;
  slow: Float32Array;
  fast: Float32Array;
  sparks: Spark[];
}

function smoothstep(a: number, b: number, v: number): number {
  const t = v <= a ? 0 : v >= b ? 1 : (v - a) / (b - a);
  return t * t * (3 - 2 * t);
}

/** Base heat at a height (0 at the top, 1 at the bottom): rising toward the bottom edge. */
export function emberProfile(u: number): number {
  const rise = smoothstep(EMBER_TOP, 0.9, u);
  // The last rows cool again, so the section edge is the field's own dithered falloff.
  const edge = 1 - 0.55 * smoothstep(0.93, 1, u);
  return Math.pow(rise, 1.5) * edge;
}

function build(grid: Grid, rand: () => number): EmberState {
  const { cols, rows, px } = grid;
  const profile = new Float32Array(rows);
  for (let y = 0; y < rows; y++) profile[y] = emberProfile(rows > 1 ? y / (rows - 1) : 1);
  // Noise scales are in CSS pixels, so the flames have the same size at every cell size.
  const slow = noiseField(cols, rows, px / 70, px / 46, 41);
  const fast = noiseField(cols, rows, px / 26, px / 18, 59);
  const sparks: Spark[] = [];
  const count = Math.max(5, Math.round(cols / 30));
  for (let i = 0; i < count; i++) {
    sparks.push({
      x: rand() * cols,
      y: rows * between(rand, EMBER_TOP + 0.2, 1),
      speed: between(rand, 5, 12) / px,
      sway: rand() * Math.PI * 2,
    });
  }
  return { grid, profile, slow, fast, sparks };
}

function paintSmooth(ctx: SmoothContext, s: EmberState, _frame: Frame, dpr: number): void {
  const { width, height } = s.grid;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 1;
  const g = ctx.createLinearGradient(0, 0, 0, height);
  g.addColorStop(0, SKY[0]);
  g.addColorStop(EMBER_TOP + 0.15, SKY[0]);
  g.addColorStop(0.9, SKY[1]);
  g.addColorStop(1, SKY[0]);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, width, height);
}

function paintPixels(buf: PixelBuffer, s: EmberState, frame: Frame): void {
  const { cols, rows, px } = s.grid;
  const levels = EMBER_RGB.length - 1;
  // Upward scroll, in whole cells.
  const slowOffset = Math.floor((frame.t * 9) / px) % rows;
  const fastOffset = Math.floor((frame.t * 17) / px) % rows;
  const tick = Math.floor(frame.t * 6);
  for (let y = 0; y < rows; y++) {
    const base = s.profile[y]!;
    if (base <= 0.001) continue;
    const slowRow = ((y + slowOffset) % rows) * cols;
    const fastRow = ((y + fastOffset) % rows) * cols;
    for (let x = 0; x < cols; x++) {
      const n = s.slow[slowRow + x]! * 0.62 + s.fast[fastRow + x]! * 0.38;
      const flicker = (hash3(x, y, tick) - 0.5) * 0.22;
      const heat = base * (0.15 + 1.05 * n * n * 1.6) + flicker - 0.14;
      if (heat <= 0) continue;
      const level = Math.min(levels, Math.floor(Math.pow(Math.min(1, heat), 1.45) * levels + bayer8(x, y)));
      if (level <= 0) continue;
      buf.set(x, y, EMBER_RGB[level]!);
    }
  }
  // Sparks climb out of the field and cool as they go.
  const ceiling = rows * (EMBER_TOP + 0.05);
  const span = rows - ceiling;
  for (const spark of s.sparks) {
    const travelled = (spark.speed * frame.t) % span;
    let y = Math.round(spark.y - travelled);
    if (y < ceiling) y += Math.round(span);
    const x = Math.round(spark.x + Math.sin(frame.t * 0.8 + spark.sway) * 1.5);
    const cool = 1 - y / rows;
    buf.set(x, y, cool < 0.45 ? SUN_RGB[2]! : lerpRgb(SUN_RGB[4]!, SKY_RGB[9]!, Math.min(1, (cool - 0.45) * 2.5)));
  }
}

export const ember: SceneDef<EmberState> = {
  name: 'ember',
  followsPointer: false,
  maxRise: 0,
  build,
  paintSmooth,
  paintPixels,
};
