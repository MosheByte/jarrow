/** Value noise for ridges and the ember field: lattice values from an integer hash. */

import { hash3 } from './prng.ts';

function smooth(t: number): number {
  return t * t * (3 - 2 * t);
}

/** 1-D value noise in [0, 1): random lattice values, smoothstep between them. */
export function valueNoise1(x: number, seed: number): number {
  const i = Math.floor(x);
  const f = x - i;
  const a = hash3(i, 0, 0, seed);
  const b = hash3(i + 1, 0, 0, seed);
  return a + (b - a) * smooth(f);
}

/** Sum of octaves of 1-D value noise, each twice the frequency and half the weight; in [0, 1). */
export function fbm1(x: number, seed: number, octaves = 4): number {
  let sum = 0;
  let weight = 1;
  let total = 0;
  let freq = 1;
  for (let o = 0; o < octaves; o++) {
    sum += valueNoise1(x * freq, seed + o * 101) * weight;
    total += weight;
    weight *= 0.5;
    freq *= 2;
  }
  return sum / total;
}

/** 2-D value noise in [0, 1). */
export function valueNoise2(x: number, y: number, seed: number): number {
  const ix = Math.floor(x);
  const iy = Math.floor(y);
  const fx = smooth(x - ix);
  const fy = smooth(y - iy);
  const a = hash3(ix, iy, 1, seed);
  const b = hash3(ix + 1, iy, 1, seed);
  const c = hash3(ix, iy + 1, 1, seed);
  const d = hash3(ix + 1, iy + 1, 1, seed);
  const top = a + (b - a) * fx;
  const bottom = c + (d - c) * fx;
  return top + (bottom - top) * fy;
}

/**
 * A field of 2-D noise sampled once per cell, `cols` wide and `rows` tall, wrapping in y so
 * it can scroll forever. Two octaves; values in [0, 1).
 */
export function noiseField(cols: number, rows: number, scaleX: number, scaleY: number, seed: number): Float32Array {
  const field = new Float32Array(cols * rows);
  const period = rows * scaleY;
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const sx = x * scaleX;
      const sy = y * scaleY;
      // Blend with the sample one period up so the field wraps without a seam.
      const w = y / rows;
      const n0 = valueNoise2(sx, sy, seed) * 0.65 + valueNoise2(sx * 2.1, sy * 2.1, seed + 7) * 0.35;
      const n1 =
        valueNoise2(sx, sy - period, seed) * 0.65 + valueNoise2(sx * 2.1, (sy - period) * 2.1, seed + 7) * 0.35;
      field[y * cols + x] = n0 * (1 - w) + n1 * w;
    }
  }
  return field;
}
