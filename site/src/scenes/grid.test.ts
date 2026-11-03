import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { BAYER8_THRESHOLDS, bayerMatrix, gridFor, pixelSize, stripHeight } from './grid.ts';
import { mulberry32 } from './prng.ts';

describe('the pixel grid', () => {
  it('uses px = clamp(round(width / 340), 3, 6): 3 at 390, 4 at 1440, 6 at 2040', () => {
    assert.equal(pixelSize(390), 3);
    assert.equal(pixelSize(1440), 4);
    assert.equal(pixelSize(2040), 6);
    assert.equal(pixelSize(320), 3);
    assert.equal(pixelSize(3840), 6);
    // The steps fall where round() crosses a half: 1530 px is the first width at 5.
    assert.equal(pixelSize(1529), 4);
    assert.equal(pixelSize(1530), 5);
  });

  it('gives a 360 x 225 buffer at 1440 x 900, and whole cells at every size', () => {
    assert.deepEqual(gridFor(1440, 900), { width: 1440, height: 900, px: 4, cols: 360, rows: 225 });
    assert.deepEqual(gridFor(390, 253), { width: 390, height: 253, px: 3, cols: 130, rows: 85 });
    assert.deepEqual(gridFor(2040, 900), { width: 2040, height: 900, px: 6, cols: 340, rows: 150 });
    for (const [w, h] of [
      [1366, 768],
      [1023, 700],
      [375, 667],
    ] as const) {
      const g = gridFor(w, h);
      assert.ok(g.cols * g.px >= w && (g.cols - 1) * g.px < w, `cols cover ${w}`);
      assert.ok(g.rows * g.px >= h && (g.rows - 1) * g.px < h, `rows cover ${h}`);
    }
  });

  it('makes the narrow sunrise strip clamp(0.3 * innerHeight, 180, 320) tall', () => {
    assert.equal(stripHeight(844), 253);
    assert.equal(stripHeight(500), 180);
    assert.equal(stripHeight(1200), 320);
  });
});

describe('the Bayer matrix', () => {
  it('is a permutation of 0..63 at 8 x 8, with the classic 2 x 2 core', () => {
    assert.deepEqual(bayerMatrix(2), [0, 2, 3, 1]);
    const m = bayerMatrix(8);
    assert.deepEqual(
      [...m].sort((a, b) => a - b),
      Array.from({ length: 64 }, (_, i) => i),
    );
    assert.equal(BAYER8_THRESHOLDS.length, 64);
    assert.ok(BAYER8_THRESHOLDS.every((v) => v > 0 && v < 1));
  });

  it('spreads neighbouring thresholds apart (ordered, not random)', () => {
    const m = bayerMatrix(8);
    // Horizontal neighbours always differ by at least 16 in an 8 x 8 Bayer matrix.
    for (let y = 0; y < 8; y++) {
      for (let x = 0; x < 7; x++) assert.ok(Math.abs(m[y * 8 + x]! - m[y * 8 + x + 1]!) >= 16);
    }
  });
});

describe('the seeded generator', () => {
  it('repeats exactly for the same seed and differs for another', () => {
    const a = mulberry32(7);
    const b = mulberry32(7);
    const c = mulberry32(8);
    const seqA = Array.from({ length: 32 }, a);
    assert.deepEqual(seqA, Array.from({ length: 32 }, b));
    assert.notDeepEqual(seqA, Array.from({ length: 32 }, c));
    assert.ok(seqA.every((v) => v >= 0 && v < 1));
  });
});
