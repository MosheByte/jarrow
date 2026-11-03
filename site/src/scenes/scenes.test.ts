import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { aurora, MORNING_STAR } from './aurora.ts';
import { EMBER_TOP, ember, emberProfile } from './ember.ts';
import { type Grid, gridFor } from './grid.ts';
import {
  AURORA_RGB,
  EMBER_RGB,
  PALETTE_RGB,
  type Rgb,
  SKY,
  SKY_RGB,
  SUN,
  SUN_RGB,
  isPaletteLerp,
  lerpRgb,
  rgbToHex,
} from './palette.ts';
import { PixelBuffer } from './pixels.ts';
import { SCENE_SEED, mulberry32 } from './prng.ts';
import type { Frame, SceneDef } from './runtime.ts';
import { MAX_RISE, SUN_X_NARROW, SUN_X_WIDE, type SunriseState, sunRow, sunrise } from './sunrise.ts';
import { RecordingContext } from './test-helpers.ts';

const AT_REST: Frame = { t: 0, pointerX: 0, scroll: 0, rise: 0 };

function frameAt(t: number, extra: Partial<Frame> = {}): Frame {
  return { ...AT_REST, t, ...extra };
}

function render<S>(def: SceneDef<S>, grid: Grid, frame: Frame, variant?: string, seed = SCENE_SEED) {
  const state = def.build(grid, mulberry32(seed), { variant });
  const buf = PixelBuffer.create(grid.cols, grid.rows);
  def.paintPixels(buf, state, frame);
  const ctx = new RecordingContext();
  def.paintSmooth(ctx, state, frame, 1);
  return { state, buf, ctx };
}

function uniqueColours(buf: PixelBuffer): Rgb[] {
  const seen = new Map<string, Rgb>();
  for (let i = 0; i < buf.data.length; i += 4) {
    if (buf.data[i + 3] === 0) continue;
    assert.equal(buf.data[i + 3], 255, 'cells are opaque or empty, never translucent');
    const c: Rgb = [buf.data[i]!, buf.data[i + 1]!, buf.data[i + 2]!];
    seen.set(rgbToHex(c), c);
  }
  return [...seen.values()];
}

function setCount(buf: PixelBuffer, y0: number, y1: number): number {
  let n = 0;
  for (let y = y0; y < y1; y++) for (let x = 0; x < buf.width; x++) if (buf.isSet(x, y)) n++;
  return n;
}

const SCENES: [string, SceneDef<unknown>, string | undefined][] = [
  ['sunrise', sunrise as SceneDef<unknown>, undefined],
  ['sunrise (releases)', sunrise as SceneDef<unknown>, 'releases'],
  ['ember field', ember as SceneDef<unknown>, undefined],
  ['aurora band', aurora as SceneDef<unknown>, undefined],
];
const SIZES = [
  [1440, 900],
  [390, 253],
  [2040, 1000],
] as const;

describe('determinism', () => {
  for (const [label, def, variant] of SCENES) {
    it(`${label}: the same seed, size and time give byte-identical buffers`, () => {
      for (const [w, h] of SIZES) {
        for (const t of [0, 3.25, 41.7]) {
          const a = render(def, gridFor(w, h), frameAt(t), variant);
          const b = render(def, gridFor(w, h), frameAt(t), variant);
          assert.ok(Buffer.from(a.buf.data).equals(Buffer.from(b.buf.data)), `${w}x${h} at ${t}s`);
          assert.deepEqual(a.ctx.colours, b.ctx.colours, `smooth layer ${w}x${h} at ${t}s`);
        }
      }
    });

    it(`${label}: another seed gives another picture`, () => {
      const grid = gridFor(1440, 900);
      const a = render(def, grid, frameAt(2), variant, SCENE_SEED);
      const b = render(def, grid, frameAt(2), variant, SCENE_SEED + 1);
      assert.ok(!Buffer.from(a.buf.data).equals(Buffer.from(b.buf.data)));
    });
  }
});

describe('palette', () => {
  it('starts from exactly the 8.2 ramps: 15 sky stops and 7 sun stops', () => {
    assert.deepEqual(
      [...SKY],
      [
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
      ],
    );
    assert.deepEqual([...SUN], ['#fff8dc', '#ffeeb0', '#ffdd88', '#ffc862', '#ffb454', '#ff9a48', '#ff7a3d']);
  });

  it('accepts ramp stops and their lerps, and rejects anything else', () => {
    assert.ok(isPaletteLerp([0x0b, 0x0f, 0x1f]));
    assert.ok(isPaletteLerp(lerpRgb(SKY_RGB[0]!, SUN_RGB[0]!, 0.37)), 'a lerp between any two stops');
    assert.ok(!isPaletteLerp([0x00, 0xff, 0x00]), 'pure green is not in the ramps');
    assert.ok(!isPaletteLerp([0xa9, 0x8f, 0xff]), "Horizon's violet accent is not in the ramps");
    assert.ok(!isPaletteLerp([0x8b, 0xe0, 0xb4]), 'the tier mint is reserved for results');
  });

  for (const [label, def, variant] of SCENES) {
    it(`${label}: both layers use only ramp colours and their lerps`, () => {
      for (const [w, h] of SIZES) {
        for (const t of [0, 1.5, 12.3, 77.7]) {
          const { buf, ctx } = render(def, gridFor(w, h), frameAt(t, { pointerX: 0.6, scroll: 0.4, rise: 2 }), variant);
          for (const c of uniqueColours(buf)) {
            assert.ok(isPaletteLerp(c), `pixel ${rgbToHex(c)} at ${w}x${h}, ${t}s`);
          }
          for (const c of ctx.rgb()) assert.ok(isPaletteLerp(c), `smooth ${rgbToHex(c)} at ${w}x${h}, ${t}s`);
        }
      }
    });
  }

  it('the ember field paints from its night-to-amber ramp, the aurora from the amber-to-night subset', () => {
    const e = render(ember, gridFor(1440, 600), frameAt(5));
    for (const c of uniqueColours(e.buf)) {
      assert.ok(isPaletteLerp(c, [...EMBER_RGB, ...PALETTE_RGB.slice(15)]), rgbToHex(c));
    }
    const a = render(aurora, gridFor(1440, 540), frameAt(5));
    const auroraCells = uniqueColours(a.buf).filter((c) => isPaletteLerp(c, AURORA_RGB));
    assert.ok(auroraCells.length >= 6, 'the dither spreads the band over at least six ramp steps');
  });
});

describe('sunrise', () => {
  function sunState(w: number, h: number, variant?: string): SunriseState {
    return sunrise.build(gridFor(w, h), mulberry32(SCENE_SEED), { variant });
  }

  it('puts the sun at 63 % of the width on wide screens and 50 % under 1024 px', () => {
    const wide = sunState(1440, 900);
    assert.equal(wide.sunX, Math.round(wide.grid.cols * SUN_X_WIDE));
    const narrow = sunState(390, 253);
    assert.equal(narrow.sunX, Math.round(narrow.grid.cols * SUN_X_NARROW));
    assert.equal(sunState(1023, 300).sunX, Math.round(gridFor(1023, 300).cols * 0.5));
    assert.equal(sunState(1024, 900).sunX, Math.round(gridFor(1024, 900).cols * 0.63));
  });

  it('keeps the landscape in the bottom band at 1440, under the copy', () => {
    const s = sunState(1440, 900);
    const band = (s.grid.rows - s.bandTop) * s.grid.px;
    assert.ok(band >= 200 && band <= 320, `band ${band}px`);
    assert.ok((s.sunY - s.radius) * s.grid.px > 560, 'the sun starts below the hero copy');
  });

  it('hides the lower half of the sun behind the far ridge', () => {
    const s = sunState(1440, 900);
    const far = s.ridges[0]!;
    for (let x = s.sunX - s.radius; x <= s.sunX + s.radius; x++) {
      assert.ok(far.tops[x + far.margin]! <= s.sunY, `ridge top at column ${x} covers the disc's lower half`);
    }
    // And the disc adds nothing below its centre: the picture there is the same without a sun.
    const withSun = PixelBuffer.create(s.grid.cols, s.grid.rows);
    sunrise.paintPixels(withSun, s, AT_REST);
    const withoutSun = PixelBuffer.create(s.grid.cols, s.grid.rows);
    sunrise.paintPixels(withoutSun, { ...s, radius: 0 }, AT_REST);
    const from = (s.sunY + 1) * s.grid.cols * 4;
    assert.ok(Buffer.from(withSun.data.subarray(from)).equals(Buffer.from(withoutSun.data.subarray(from))));
    assert.ok(!Buffer.from(withSun.data).equals(Buffer.from(withoutSun.data)), 'the upper half shows');
  });

  it('cuts scanline gaps into the lower part of the disc, and they drift', () => {
    const s = sunState(1440, 900, 'releases');
    // The disc alone: no clouds, ridges or birds in the way.
    const bare = { ...s, clouds: [], ridges: [], birds: { ...s.birds, members: [] } } as SunriseState;
    const disc = (t: number) => {
      const buf = PixelBuffer.create(s.grid.cols, s.grid.rows);
      sunrise.paintPixels(buf, bare, frameAt(t));
      const rows: boolean[] = [];
      for (let y = s.sunY - s.radius; y < s.sunY + s.radius; y++) rows.push(buf.isSet(s.sunX, y));
      return rows;
    };
    const at0 = disc(0);
    assert.ok(at0.slice(0, Math.floor(s.radius * 0.4)).every(Boolean), 'the top of the disc is whole');
    assert.ok(at0.some((v) => !v), 'there are gaps lower down');
    assert.notDeepEqual(disc(0.9), at0, 'the gaps move with time');
  });

  it('draws stars only in the top third of the scene', () => {
    for (const [w, h] of SIZES) {
      const { ctx, state } = render(sunrise, gridFor(w, h), frameAt(4));
      assert.ok(ctx.arcs.length > 0);
      for (const arc of ctx.arcs) assert.ok(arc.y < state.grid.height / 3, `star at y=${arc.y} of ${h}`);
    }
  });

  it('flies three 3 x 2 dawn birds', () => {
    const s = sunState(1440, 900);
    assert.equal(s.birds.members.length, 3);
    // Paint only the birds: a buffer with nothing else in it.
    const only = { ...s, clouds: [], ridges: [], radius: 0 } as SunriseState;
    const buf = PixelBuffer.create(s.grid.cols, s.grid.rows);
    sunrise.paintPixels(buf, { ...only, sunY: -100 }, AT_REST);
    let cells = 0;
    for (let i = 3; i < buf.data.length; i += 4) if (buf.data[i]) cells++;
    assert.equal(cells, 3 * 3, 'three sprites of three cells each');
  });

  it('rises one step per rise(), up to the cap, and a little with scrolling', () => {
    const s = sunState(1440, 900);
    const rest = sunRow(s, AT_REST);
    assert.equal(sunRow(s, frameAt(0, { rise: 1 })), rest - s.riseStep);
    assert.equal(sunRow(s, frameAt(0, { rise: MAX_RISE })), rest - MAX_RISE * s.riseStep);
    const scrolled = sunRow(s, frameAt(0, { scroll: 1 }));
    assert.ok(scrolled < rest && rest - scrolled <= s.radius * 0.3 + 1, 'scrolling lifts the sun slightly');
  });

  it('moves clouds and ridges with the pointer', () => {
    const grid = gridFor(1440, 900);
    const left = render(sunrise, grid, frameAt(0, { pointerX: -1 }));
    const right = render(sunrise, grid, frameAt(0, { pointerX: 1 }));
    assert.ok(!Buffer.from(left.buf.data).equals(Buffer.from(right.buf.data)));
  });
});

describe('ember field', () => {
  it('is densest at the bottom and empty above 30 % of the height', () => {
    const { buf } = render(ember, gridFor(1440, 600), frameAt(6));
    const rows = buf.height;
    assert.equal(setCount(buf, 0, Math.floor(rows * EMBER_TOP * 0.6)), 0, 'nothing in the top of the hero');
    const upper = setCount(buf, Math.floor(rows * 0.35), Math.floor(rows * 0.55));
    const lower = setCount(buf, Math.floor(rows * 0.75), Math.floor(rows * 0.95));
    assert.ok(lower > upper * 2, `lower ${lower} vs upper ${upper}`);
    assert.ok(emberProfile(0.2) === 0 && emberProfile(0.93) > 0.9);
  });

  it('changes from frame to frame (the heat rises)', () => {
    const grid = gridFor(1440, 600);
    assert.ok(!Buffer.from(render(ember, grid, frameAt(1)).buf.data).equals(Buffer.from(render(ember, grid, frameAt(2)).buf.data)));
  });
});

describe('aurora band', () => {
  it('dithers its band onto the ramp and leaves the section below it empty', () => {
    const { buf } = render(aurora, gridFor(1440, 540), frameAt(3));
    const rows = buf.height;
    assert.ok(setCount(buf, Math.floor(rows * 0.4), Math.floor(rows * 0.7)) > buf.width * 4);
    assert.equal(setCount(buf, Math.ceil(rows * 0.8), rows), 0, 'below the front band the page shows');
  });

  it('draws the morning star top right, a 9 x 9 sprite of the sun ramp', () => {
    assert.equal(MORNING_STAR.length, 9);
    assert.ok(MORNING_STAR.every((row) => row.length === 9));
    const grid = gridFor(1440, 540);
    const state = aurora.build(grid, mulberry32(SCENE_SEED), {});
    assert.ok(state.starX > grid.cols * 0.75, 'right side');
    assert.ok(state.starY * grid.px < 140, 'near the top');
    const { buf } = render(aurora, grid, AT_REST);
    const centre = ((state.starY + 4) * buf.width + state.starX + 4) * 4;
    assert.equal(rgbToHex([buf.data[centre]!, buf.data[centre + 1]!, buf.data[centre + 2]!]), SUN[0]);
  });
});
