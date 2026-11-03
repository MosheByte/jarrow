/**
 * The aurora band (BUILD-BOOK 8.3), for /authors and the community sections: two wave bands,
 * each a sum of sines per column, bright at its stepped lower edge and fading upward in rays.
 * Brightness is ordered-dithered with an 8x8 Bayer matrix onto the amber-to-night subset of the
 * sky ramp; the band's edges are cut by the section's overflow: hidden, so they are the only
 * divider. A morning star, Jarrow's own sprite, sits top right with a slow halo.
 */

import { type Grid, bayer8 } from './grid.ts';
import { AURORA_RGB, SKY, SKY_RGB, SUN_RGB, lerpRgb, rgba } from './palette.ts';
import type { PixelBuffer } from './pixels.ts';
import { between } from './prng.ts';
import type { Frame, SceneDef, SmoothContext } from './runtime.ts';

/**
 * The morning star: a diamond core with four long rays, three tones of the sun ramp. The
 * outer ray tips ("3") show only at the top of each twinkle.
 */
export const MORNING_STAR = [
  '....3....',
  '....2....',
  '...212...',
  '..21012..',
  '321000123',
  '..21012..',
  '...212...',
  '....2....',
  '....3....',
] as const;

interface Star {
  x: number;
  y: number;
  phase: number;
  speed: number;
}

export interface AuroraState {
  grid: Grid;
  stars: Star[];
  /** Morning star position, top-left cell. */
  starX: number;
  starY: number;
  /** exp(-d / width) by whole-row distance d, for the two bands. */
  frontFalloff: Float32Array;
  backFalloff: Float32Array;
}

/** Lower edge of the front and back bands at a CSS x, in rows. */
export function bandEdges(x: number, t: number, rows: number): { front: number; back: number } {
  const front = rows * (0.64 + 0.065 * Math.sin(0.0042 * x + 0.31 * t) + 0.03 * Math.sin(0.0127 * x - 0.52 * t + 1.3));
  const back =
    rows * (0.45 + 0.085 * Math.sin(0.0029 * x - 0.23 * t + 0.7) + 0.035 * Math.sin(0.0161 * x + 0.44 * t + 2.1));
  return { front, back };
}

function falloff(rows: number, width: number): Float32Array {
  const table = new Float32Array(rows + 1);
  for (let d = 0; d <= rows; d++) table[d] = Math.exp(-d / width);
  return table;
}

function build(grid: Grid, rand: () => number): AuroraState {
  const { cols, rows, px } = grid;
  const stars: Star[] = [];
  const count = Math.max(8, Math.round(cols / 18));
  for (let i = 0; i < count; i++) {
    stars.push({
      x: Math.floor(rand() * cols),
      y: Math.floor(Math.pow(rand(), 1.2) * rows * 0.55),
      phase: rand() * Math.PI * 2,
      speed: between(rand, 0.5, 1.8),
    });
  }
  const size = MORNING_STAR.length;
  return {
    grid,
    stars,
    starX: cols - Math.max(size + 2, Math.round(cols * 0.11)) - size,
    starY: Math.max(2, Math.round(96 / px) - Math.floor(size / 2)),
    frontFalloff: falloff(rows, rows * 0.15),
    backFalloff: falloff(rows, rows * 0.19),
  };
}

function paintSmooth(ctx: SmoothContext, s: AuroraState, frame: Frame, dpr: number): void {
  const { width, height, px } = s.grid;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 1;
  ctx.fillStyle = SKY[0];
  ctx.fillRect(0, 0, width, height);
  // The morning star's halo, breathing by 8 %.
  const size = MORNING_STAR.length * px;
  const cx = s.starX * px + size / 2;
  const cy = s.starY * px + size / 2;
  const strength = 0.16 * (1 + 0.08 * Math.sin(frame.t * 0.8));
  const halo = ctx.createRadialGradient(cx, cy, size * 0.2, cx, cy, size * 2.6);
  halo.addColorStop(0, rgba(SUN_RGB[2]!, strength));
  halo.addColorStop(1, rgba(SKY_RGB[3]!, 0));
  ctx.fillStyle = halo;
  ctx.fillRect(cx - size * 3, cy - size * 3, size * 6, size * 6);
}

function paintPixels(buf: PixelBuffer, s: AuroraState, frame: Frame): void {
  const { cols, rows, px } = s.grid;
  const t = frame.t;

  // Stars first: the aurora covers the ones it passes over.
  const bright = SUN_RGB[0]!;
  const dim = lerpRgb(SUN_RGB[0]!, SKY_RGB[3]!, 0.45);
  for (const star of s.stars) {
    const blink = 0.5 + 0.5 * Math.sin(t * star.speed + star.phase);
    if (blink < 0.2) continue;
    buf.set(star.x, star.y, blink > 0.55 ? bright : dim);
    if (blink > 0.93) {
      buf.set(star.x - 1, star.y, dim);
      buf.set(star.x + 1, star.y, dim);
      buf.set(star.x, star.y - 1, dim);
      buf.set(star.x, star.y + 1, dim);
    }
  }

  const levels = AURORA_RGB.length - 1;
  const cut = 2; // rows of falloff below each band's lower edge
  for (let x = 0; x < cols; x++) {
    const cx = (x + 0.5) * px;
    const { front, back } = bandEdges(cx, t, rows);
    const frontEdge = Math.round(front);
    const backEdge = Math.round(back);
    const frontRays = 0.45 + 0.55 * Math.pow(0.5 + 0.5 * Math.sin(0.019 * cx + 2.4 * Math.sin(0.0035 * cx + 0.16 * t)), 1.6);
    const backRays =
      0.4 + 0.6 * Math.pow(0.5 + 0.5 * Math.sin(0.026 * cx - 1.9 * Math.sin(0.0041 * cx - 0.12 * t) + 1.1), 1.6);
    const bottom = Math.min(rows - 1, Math.max(frontEdge, backEdge) + cut);
    for (let y = 0; y <= bottom; y++) {
      const fd = frontEdge - y;
      const bd = backEdge - y;
      const f = fd >= 0 ? s.frontFalloff[fd]! : Math.max(0, 1 + fd / (cut + 1));
      const b = bd >= 0 ? s.backFalloff[bd]! : Math.max(0, 1 + bd / (cut + 1));
      const lightF = f * 0.9 * frontRays;
      const lightB = b * 0.66 * backRays;
      const light = 1 - (1 - lightF) * (1 - lightB);
      if (light < 0.02) continue;
      const level = Math.min(levels, Math.floor(Math.pow(light, 1.35) * levels + bayer8(x, y)));
      if (level <= 0) continue;
      buf.set(x, y, AURORA_RGB[level]!);
    }
  }

  // The morning star, over everything.
  const twinkle = 0.5 + 0.5 * Math.sin(t * 1.1);
  const inks = {
    '0': SUN_RGB[0]!,
    '1': SUN_RGB[1]!,
    '2': SUN_RGB[2]!,
    ...(twinkle >= 0.5 ? { '3': SUN_RGB[4]! } : {}),
  };
  buf.sprite(s.starX, s.starY, MORNING_STAR, inks);
}

export const aurora: SceneDef<AuroraState> = {
  name: 'aurora',
  followsPointer: false,
  maxRise: 0,
  build,
  paintSmooth,
  paintPixels,
};
