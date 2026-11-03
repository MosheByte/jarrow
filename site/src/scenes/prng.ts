/**
 * Deterministic randomness for the scenes. Every layout decision comes from one seeded
 * generator, and every per-frame flicker from an integer hash, so every visitor sees the
 * same picture at the same size and the same moment. Math.random is never used.
 */

/** The seed every scene starts from (BUILD-BOOK 8.3: seed 7). */
export const SCENE_SEED = 7;

/** A mulberry32-style generator: 32-bit state, returns floats in [0, 1). */
export function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 15), z | 1);
    z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
    return ((z ^ (z >>> 14)) >>> 0) / 4294967296;
  };
}

/** A float in [min, max) from a generator. */
export function between(rand: () => number, min: number, max: number): number {
  return min + (max - min) * rand();
}

/** An integer in [min, max] from a generator. */
export function intBetween(rand: () => number, min: number, max: number): number {
  return min + Math.floor(rand() * (max - min + 1));
}

/** A stateless hash of up to three integers to a float in [0, 1). */
export function hash3(x: number, y: number, z: number, seed = SCENE_SEED): number {
  let h = Math.imul(x | 0, 0x27d4eb2d) ^ Math.imul(y | 0, 0x165667b1) ^ Math.imul(z | 0, 0x9e3779b1);
  h = Math.imul(h ^ seed, 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}
