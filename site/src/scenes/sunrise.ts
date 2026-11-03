/**
 * The sunrise (BUILD-BOOK 8.3), for / and /releases: a sun coming up behind three block-stepped
 * ridges, under a sky that runs from night blue to amber.
 *
 * Smooth layer: the 15-stop sky ramp, the sun's glow, and stars in the top third only.
 * Pixel layer: stepped clouds lit from the sun, the scanlined sun disc, three value-noise
 * ridges with sun-lit rims, and three dawn birds.
 *
 * Wide screens: the scene fills the hero, the landscape sits in its bottom band and the sun at
 * 63 % of the width. Under 1024 px the component is a strip on the hero's bottom edge and the
 * sun is centred. Scrolling lifts the sun a little; rise() lifts it one step (BUILD-BOOK 8.7).
 */

import { type Grid, NARROW_BELOW } from './grid.ts';
import { fbm1 } from './noise.ts';
import { type Rgb, SKY, SKY_RGB, SUN_RGB, lerpRgb, rampAt, rgba } from './palette.ts';
import type { PixelBuffer } from './pixels.ts';
import { between, intBetween } from './prng.ts';
import type { Frame, SceneDef, SceneOptions, SmoothContext } from './runtime.ts';

/** Sun position across the width on wide screens (home), and under NARROW_BELOW. */
export const SUN_X_WIDE = 0.63;
export const SUN_X_NARROW = 0.5;
/** rise() steps the sun can take. */
export const MAX_RISE = 4;

/** Widest parallax shift of the nearest layer, in CSS pixels. */
const PARALLAX_PX = 8;

interface Variant {
  /** Landscape band height: fraction of the scene height, clamped (CSS px). Wide screens only. */
  band: number;
  bandMin: number;
  bandMax: number;
  /** Far ridge line, as a fraction of the band from its top. */
  far: number;
  /** Sun radius as a fraction of the band height. */
  radius: number;
  /** Sun centre above the far ridge line, as a fraction of the radius. */
  lift: number;
  sunX: number;
  /** Where the scanline bands start, as a fraction of the radius above the centre. */
  bandsFrom: number;
}

const HOME: Variant = {
  band: 0.32,
  bandMin: 200,
  bandMax: 320,
  far: 0.42,
  radius: 0.36,
  lift: 0,
  sunX: SUN_X_WIDE,
  bandsFrom: 0.5,
};
const RELEASES: Variant = {
  band: 0.36,
  bandMin: 220,
  bandMax: 360,
  far: 0.4,
  radius: 0.55,
  lift: 0.25,
  sunX: 0.72,
  bandsFrom: 0.55,
};

type CloudCell = 0 | 1 | 2 | 3; // empty, body, lit underside, top edge

interface Cloud {
  x: number;
  y: number;
  w: number;
  h: number;
  cells: Uint8Array;
  speed: number; // cells per second
  depth: number;
  front: boolean;
  body: Rgb;
  top: Rgb;
}

interface Ridge {
  /** Top row per column, for columns -margin .. cols + margin - 1. */
  tops: Int16Array;
  margin: number;
  body: Rgb;
  rimFar: Rgb;
  rimNear: Rgb;
  depth: number;
}

interface Star {
  x: number;
  y: number;
  r: number;
  alpha: number;
  warm: boolean;
  phase: number;
  speed: number;
  sparkle: boolean;
}

interface Bird {
  dx: number;
  dy: number;
  phase: number;
}

export interface SunriseState {
  grid: Grid;
  narrow: boolean;
  /** Rows: top of the landscape band and the far ridge line. */
  bandTop: number;
  farLine: number;
  /** Sun centre column and row at rest, radius in cells, rows per rise() step. */
  sunX: number;
  sunY: number;
  radius: number;
  riseStep: number;
  bandsFrom: number;
  bandPeriod: number;
  sky: { offset: number; colour: string }[];
  stars: Star[];
  clouds: Cloud[];
  ridges: Ridge[];
  birds: { x: number; y: number; speed: number; members: Bird[] };
}

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}

/** Sky stops: night over the copy, the warm stops packed into the band, the last ones below the far ridge. */
function skyStops(height: number, nightEnd: number, far: number, bandCss: number): { offset: number; colour: string }[] {
  const stops: { offset: number; colour: string }[] = [];
  let previous = 0;
  for (let i = 0; i < SKY.length; i++) {
    let y: number;
    if (i <= 5) y = nightEnd * Math.pow(i / 5, 0.6);
    else if (i <= 11) y = nightEnd + ((far - nightEnd) * (i - 5)) / 6;
    else y = far + (i - 11) * 0.07 * bandCss;
    const offset = Math.max(previous, clamp(y / height, 0, 1));
    previous = offset;
    stops.push({ offset, colour: SKY[i]! });
  }
  return stops;
}

/** Index of the sky stop at a height, for tinting clouds against the sky behind them. */
function skyIndexAt(sky: { offset: number }[], offset: number): number {
  let index = 0;
  for (let i = 0; i < sky.length; i++) if (sky[i]!.offset <= offset) index = i;
  return index;
}

/**
 * A cloud mask: stepped mounds standing on one row, each row narrower than the one below by a
 * random step on each side, and a lit underside that runs past the body on both sides.
 */
function cloudMask(rand: () => number, w: number, big: boolean): { cells: Uint8Array; w: number; h: number } {
  const bodyRows = big ? Math.max(3, Math.round(w * between(rand, 0.2, 0.26))) : intBetween(rand, 1, 2);
  const reach = Math.max(2, Math.round(w * (big ? between(rand, 0.08, 0.16) : between(rand, 0.1, 0.3))));
  const fullW = w + reach * 2;
  const h = bodyRows + 1;
  const cells = new Uint8Array(fullW * h);
  // A low base across the whole width, then one or two taller, narrower humps on it.
  const mounds = big ? intBetween(rand, 2, 3) : 1;
  for (let m = 0; m < mounds; m++) {
    let left = 0;
    let right = w;
    if (m > 0) {
      const centre = w * between(rand, 0.25, 0.75);
      const half = w * between(rand, 0.2, 0.34);
      left = Math.round(centre - half);
      right = Math.round(centre + half);
    }
    const stepMax = !big
      ? Math.max(2, Math.round(w * 0.3))
      : m === 0
        ? Math.max(2, Math.round(w * 0.14))
        : Math.max(1, Math.round(w * 0.035));
    for (let row = bodyRows - 1; row >= 0 && right - left >= 2; row--) {
      for (let x = Math.max(0, left); x < Math.min(w, right); x++) cells[row * fullW + x + reach] = 1;
      left += intBetween(rand, 1, stepMax);
      right -= intBetween(rand, 1, stepMax);
    }
  }
  // Underside: as wide as the bottom body row, plus the reach on each side.
  let minX = fullW;
  let maxX = -1;
  for (let x = 0; x < fullW; x++) {
    if (cells[(bodyRows - 1) * fullW + x]) {
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
    }
  }
  for (let x = Math.max(0, minX - reach); x <= Math.min(fullW - 1, maxX + reach); x++) cells[bodyRows * fullW + x] = 2;
  // Top edge: body cells with nothing above them.
  for (let row = 0; row < bodyRows; row++) {
    for (let x = 0; x < fullW; x++) {
      const i = row * fullW + x;
      if (cells[i] === 1 && (row === 0 || cells[i - fullW] === 0)) cells[i] = 3;
    }
  }
  return { cells, w: fullW, h };
}

function buildRidge(
  grid: Grid,
  seed: number,
  line: number,
  amplitude: number,
  wavelengthPx: number,
  quantum: number,
  colours: { body: Rgb; rimFar: Rgb; rimNear: Rgb },
  depth: number,
  margin: number,
): Ridge {
  const { cols, px } = grid;
  const tops = new Int16Array(cols + margin * 2);
  for (let i = 0; i < tops.length; i++) {
    const x = i - margin;
    const n = fbm1((x * px) / wavelengthPx, seed, 4);
    const top = line - amplitude * (n - 0.5) * 2.4;
    tops[i] = Math.round(top / quantum) * quantum;
  }
  return { tops, margin, depth, ...colours };
}

function build(grid: Grid, rand: () => number, options: SceneOptions): SunriseState {
  const { width, height, px, cols, rows } = grid;
  const narrow = width < NARROW_BELOW;
  const v = options.variant === 'releases' ? RELEASES : HOME;

  // Wide: the landscape takes the bottom band of the hero. Narrow: the strip is all landscape.
  const bandCss = narrow ? height : clamp(v.band * height, v.bandMin, v.bandMax);
  const bandRows = Math.min(rows, Math.round(bandCss / px));
  const bandTop = rows - bandRows;
  const farLine = bandTop + Math.round(bandRows * (narrow ? 0.46 : v.far));
  const radius = Math.max(6, Math.round(bandRows * (narrow ? 0.3 : v.radius)));
  const sunX = Math.round(cols * (narrow ? SUN_X_NARROW : v.sunX));
  const sunY = farLine - Math.round(radius * (narrow ? 0 : v.lift));

  const nightEnd = narrow ? 0.3 * height : bandTop * px;
  const sky = skyStops(height, nightEnd, farLine * px, bandCss);

  // Stars: the top third of the scene only.
  const starCount = clamp(Math.round((width * (height / 3)) / 5200), 10, 140);
  const stars: Star[] = [];
  for (let i = 0; i < starCount; i++) {
    stars.push({
      x: rand() * width,
      y: Math.pow(rand(), 1.35) * (height / 3),
      r: between(rand, 0.45, 1.15),
      alpha: between(rand, 0.3, 0.9),
      warm: rand() < 0.7,
      phase: rand() * Math.PI * 2,
      speed: between(rand, 0.6, 2.2),
      sparkle: rand() < 0.06,
    });
  }

  // Ridges: far, middle, front; value noise quantised to the grid.
  const margin = Math.ceil(PARALLAX_PX / px) + 1;
  const ridges = [
    buildRidge(
      grid,
      11,
      farLine - Math.round(bandRows * 0.05),
      bandRows * 0.2,
      420,
      1,
      { body: SKY_RGB[4]!, rimFar: SKY_RGB[6]!, rimNear: SUN_RGB[4]! },
      0.2,
      margin,
    ),
    buildRidge(
      grid,
      23,
      farLine + Math.round(bandRows * 0.22),
      bandRows * 0.16,
      260,
      2,
      { body: SKY_RGB[2]!, rimFar: SKY_RGB[4]!, rimNear: SKY_RGB[10]! },
      0.5,
      margin,
    ),
    buildRidge(
      grid,
      37,
      farLine + Math.round(bandRows * 0.42),
      bandRows * 0.1,
      190,
      2,
      { body: SKY_RGB[0]!, rimFar: SKY_RGB[2]!, rimNear: SKY_RGB[7]! },
      1,
      margin,
    ),
  ];
  // The sun comes up in a saddle of the far ridge, and its lower half stays behind it: the
  // ridge sinks toward the far line near the disc and never dips below the sun's centre.
  const far = ridges[0]!;
  for (let i = 0; i < far.tops.length; i++) {
    const x = i - far.margin;
    const d = (x - sunX) / (radius * 1.7);
    const pull = Math.exp(-d * d) * 0.75;
    let top = Math.round(far.tops[i]! * (1 - pull) + (sunY + 1) * pull);
    if (Math.abs(x - sunX) <= radius + 2) top = Math.min(top, sunY);
    far.tops[i] = top;
  }

  // Clouds: three big ones behind the sun, five thin streaks in front of it.
  const clouds: Cloud[] = [];
  const cloudTop = narrow ? Math.round(rows * 0.22) : bandTop - Math.round(bandRows * 0.12);
  const cloudBottom = farLine - Math.max(3, Math.round(radius * 0.25));
  const scale = narrow ? 0.7 : 1;
  for (let i = 0; i < 8; i++) {
    const big = i < 3;
    const w = big
      ? Math.round((between(rand, 120, 260) * scale) / px)
      : Math.round(cols * between(rand, 0.08, 0.2));
    const mask = cloudMask(rand, Math.max(6, w), big);
    const y = Math.round(between(rand, cloudTop, Math.max(cloudTop + 1, cloudBottom - mask.h)));
    const k = skyIndexAt(sky, (y * px) / height);
    clouds.push({
      x: rand() * (cols + mask.w) - mask.w / 2,
      y,
      w: mask.w,
      h: mask.h,
      cells: mask.cells,
      speed: between(rand, 0.5, 2.6) / px,
      depth: big ? 0.4 : 0.8,
      front: !big,
      // Dawn clouds stand dark against the brightening sky, with sun-lit edges.
      body: rampAt(SKY_RGB, Math.max(0, k - 0.8) / (SKY.length - 1)),
      top: rampAt(SKY_RGB, (k + 1.2) / (SKY.length - 1)),
    });
  }

  // Three dawn birds in a loose line, flying toward the sun's side of the sky.
  const birds = {
    x: sunX - radius - Math.round(cols * 0.12),
    y: Math.max(2, sunY - Math.round(radius * 0.55)),
    speed: 7 / px,
    members: [
      { dx: 0, dy: 0, phase: rand() },
      { dx: -5, dy: 2, phase: rand() },
      { dx: -10, dy: -1, phase: rand() },
    ],
  };

  return {
    grid,
    narrow,
    bandTop,
    farLine,
    sunX,
    sunY,
    radius,
    riseStep: Math.max(2, Math.round(radius * 0.12)),
    bandsFrom: v.bandsFrom,
    bandPeriod: Math.max(4, Math.round(radius / 5)),
    sky,
    stars,
    clouds,
    ridges,
    birds,
  };
}

/** Centre row of the sun for a frame: risen by rise() steps and, a little, by scrolling. */
export function sunRow(state: SunriseState, frame: Frame): number {
  return Math.round(state.sunY - frame.rise * state.riseStep - frame.scroll * 0.3 * state.radius);
}

/** How strongly the sun lights a column, 0 to 1. */
function glowAt(state: SunriseState, x: number): number {
  const d = (x - state.sunX) / (state.grid.cols * 0.3);
  return Math.exp(-d * d);
}

function paintSmooth(ctx: SmoothContext, s: SunriseState, frame: Frame, dpr: number): void {
  const { width, height, px } = s.grid;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 1;

  const sky = ctx.createLinearGradient(0, 0, 0, height);
  for (const stop of s.sky) sky.addColorStop(stop.offset, stop.colour);
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, width, height);

  // The glow around the sun, breathing slowly.
  const cx = (s.sunX + 0.5) * px;
  const cy = sunRow(s, frame) * px;
  const r = s.radius * px;
  const pulse = 0.44 + 0.04 * Math.sin(frame.t * 0.9);
  const glow = ctx.createRadialGradient(cx, cy, r * 0.2, cx, cy, r * 3.4);
  glow.addColorStop(0, rgba(SUN_RGB[3]!, pulse));
  glow.addColorStop(0.35, rgba(SUN_RGB[4]!, pulse * 0.4));
  glow.addColorStop(1, rgba(SKY_RGB[11]!, 0));
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, width, height);

  // The narrow strip starts in the page colour, so its top edge does not show.
  if (s.narrow) {
    const fade = ctx.createLinearGradient(0, 0, 0, height * 0.4);
    fade.addColorStop(0, rgba(SKY_RGB[0]!, 1));
    fade.addColorStop(1, rgba(SKY_RGB[0]!, 0));
    ctx.fillStyle = fade;
    ctx.fillRect(0, 0, width, height * 0.4);
  }

  // Stars: round and smooth, twinkling; a few carry a four-point glint.
  const warm = rgba(SUN_RGB[0]!, 1);
  const cool = rgba(lerpRgb(SUN_RGB[0]!, SKY_RGB[3]!, 0.35), 1);
  for (const star of s.stars) {
    const a = star.alpha * (0.6 + 0.4 * Math.sin(frame.t * star.speed + star.phase));
    if (a <= 0.02) continue;
    ctx.globalAlpha = a;
    ctx.fillStyle = star.warm ? warm : cool;
    ctx.beginPath();
    ctx.arc(star.x, star.y, star.r, 0, Math.PI * 2);
    ctx.fill();
    if (star.sparkle) {
      const arm = star.r * 4;
      ctx.globalAlpha = a * 0.5;
      ctx.fillRect(star.x - arm, star.y - 0.35, arm * 2, 0.7);
      ctx.fillRect(star.x - 0.35, star.y - arm, 0.7, arm * 2);
    }
  }
  ctx.globalAlpha = 1;
}

function paintCloud(buf: PixelBuffer, s: SunriseState, cloud: Cloud, frame: Frame): void {
  const span = s.grid.cols + cloud.w * 2;
  const shift = Math.round((frame.pointerX * PARALLAX_PX * cloud.depth) / s.grid.px);
  let x0 = Math.round(cloud.x + cloud.speed * frame.t) + shift;
  x0 = ((((x0 + cloud.w) % span) + span) % span) - cloud.w;
  const rim0 = SKY_RGB[9]!;
  const rim1 = SUN_RGB[3]!;
  for (let row = 0; row < cloud.h; row++) {
    for (let col = 0; col < cloud.w; col++) {
      const cell = cloud.cells[row * cloud.w + col] as CloudCell;
      if (cell === 0) continue;
      const x = x0 + col;
      const colour = cell === 2 ? lerpRgb(rim0, rim1, glowAt(s, x)) : cell === 3 ? cloud.top : cloud.body;
      buf.set(x, cloud.y + row, colour);
    }
  }
}

function paintSun(buf: PixelBuffer, s: SunriseState, frame: Frame): void {
  const r = s.radius;
  const cy = sunRow(s, frame);
  const top = cy - r;
  const bandsTop = Math.round(cy - r * s.bandsFrom);
  const bandsDepth = cy + r - bandsTop;
  const period = s.bandPeriod;
  // The bands drift upward, the way the sun moves.
  const drift = frame.t * 1.6;
  for (let y = top; y <= cy + r; y++) {
    const dy = y + 0.5 - cy;
    if (Math.abs(dy) > r) continue;
    if (y >= bandsTop) {
      const depth = (y - bandsTop) / bandsDepth;
      const gap = 1 + Math.floor(depth * (period - 2) * 0.75);
      const phase = (((y - bandsTop + drift) % period) + period) % period;
      if (phase < gap) continue;
    }
    const half = Math.floor(Math.sqrt(r * r - dy * dy));
    const colour = rampAt(SUN_RGB, (y - top) / (2 * r));
    buf.span(s.sunX - half, s.sunX + half + 1, y, colour);
  }
}

function paintRidge(buf: PixelBuffer, s: SunriseState, ridge: Ridge, frame: Frame): void {
  const shift = Math.round((frame.pointerX * PARALLAX_PX * ridge.depth) / s.grid.px);
  for (let x = 0; x < s.grid.cols; x++) {
    const i = clamp(x - shift + ridge.margin, 0, ridge.tops.length - 1);
    const top = ridge.tops[i]!;
    buf.column(x, top + 1, ridge.body);
    buf.set(x, top, lerpRgb(ridge.rimFar, ridge.rimNear, glowAt(s, x)));
  }
}

const BIRD_UP = ['#.#', '.#.'];
const BIRD_DOWN = ['.#.', '#.#'];

function paintBirds(buf: PixelBuffer, s: SunriseState, frame: Frame): void {
  const { birds } = s;
  const span = s.grid.cols + 24;
  const ink = { '#': SKY_RGB[1]! };
  for (const bird of birds.members) {
    let x = Math.round(birds.x + birds.speed * frame.t) + bird.dx;
    x = ((((x + 12) % span) + span) % span) - 12;
    const bob = Math.round(Math.sin(frame.t * 1.3 + bird.phase * 6.28) * 0.8);
    const flap = Math.floor(frame.t / 0.28 + bird.phase * 4) % 2 === 0;
    buf.sprite(x, birds.y + bird.dy + bob, flap ? BIRD_UP : BIRD_DOWN, ink);
  }
}

function paintPixels(buf: PixelBuffer, s: SunriseState, frame: Frame): void {
  for (const cloud of s.clouds) if (!cloud.front) paintCloud(buf, s, cloud, frame);
  paintSun(buf, s, frame);
  for (const cloud of s.clouds) if (cloud.front) paintCloud(buf, s, cloud, frame);
  paintBirds(buf, s, frame);
  for (const ridge of s.ridges) paintRidge(buf, s, ridge, frame);
}

export const sunrise: SceneDef<SunriseState> = {
  name: 'sunrise',
  followsPointer: true,
  maxRise: MAX_RISE,
  build,
  paintSmooth,
  paintPixels,
};
