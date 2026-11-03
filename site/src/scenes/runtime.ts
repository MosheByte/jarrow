/**
 * The shared scene runtime (BUILD-BOOK 8.3, report section 3): two stacked canvases per scene,
 * a smooth one at device resolution underneath and a low-resolution ImageData buffer on top
 * at px = clamp(round(width / 340), 3, 6), scaled up with image-rendering: pixelated.
 *
 * - Every layout comes from mulberry32(7), so every visitor sees the same picture.
 * - The loop runs on requestAnimationFrame at no more than 32 frames a second, and only while
 *   an IntersectionObserver reports the scene on screen.
 * - A ResizeObserver rebuilds the whole scene at its new size (no bitmap scaling).
 * - Under prefers-reduced-motion: reduce, one static frame is drawn and no listener, loop or
 *   visibility observer is attached. Only the ResizeObserver stays, to redraw that one still
 *   frame at a new size.
 * - The root gets data-ready once the first frame is drawn; CSS fades the canvases in over 700 ms.
 *
 * Everything the browser provides comes in through SceneEnv, so runtime.test.ts can drive it.
 */

import { type Grid, gridFor } from './grid.ts';
import { type PixelImage, PixelBuffer } from './pixels.ts';
import { SCENE_SEED, mulberry32 } from './prng.ts';

/** The frame-rate cap. */
export const MAX_FPS = 32;
/** A frame is drawn only when at least this long has passed since the last one. */
export const FRAME_MS = 1000 / MAX_FPS;
/** How long one rise() step takes to ease in, in seconds (the board card's 0.45 s re-fill). */
export const RISE_SECONDS = 0.45;
/** Device pixel ratio cap for the smooth layer. */
const MAX_DPR = 2;

export interface GradientLike {
  addColorStop(offset: number, color: string): void;
}

/** The subset of CanvasRenderingContext2D the smooth layer uses. */
export interface SmoothContext {
  fillStyle: unknown;
  globalAlpha: number;
  setTransform(a: number, b: number, c: number, d: number, e: number, f: number): void;
  fillRect(x: number, y: number, w: number, h: number): void;
  beginPath(): void;
  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number): void;
  fill(): void;
  createLinearGradient(x0: number, y0: number, x1: number, y1: number): GradientLike;
  createRadialGradient(x0: number, y0: number, r0: number, x1: number, y1: number, r1: number): GradientLike;
}

/** The subset of CanvasRenderingContext2D the pixel layer uses. */
export interface PixelContext {
  createImageData(width: number, height: number): PixelImage;
  putImageData(image: PixelImage, dx: number, dy: number): void;
}

export interface CanvasLike {
  width: number;
  height: number;
  style: { width: string; height: string };
  getContext(contextId: '2d', options?: { alpha?: boolean }): unknown;
}

export interface SceneHost {
  root: {
    getBoundingClientRect(): { width: number; height: number; top: number };
    setAttribute(name: string, value: string): void;
  };
  smooth: CanvasLike;
  pixels: CanvasLike;
}

/** What one frame is drawn from. */
export interface Frame {
  /** Scene time in seconds: 0 for the first and the static frame; it only advances on screen. */
  t: number;
  /** Pointer position across the viewport, -1 at the left edge to 1 at the right, eased. */
  pointerX: number;
  /** How far the scene has scrolled up out of view: 0 at rest, 1 once it is gone. */
  scroll: number;
  /** rise() steps taken so far, eased toward the latest count. */
  rise: number;
}

export interface SceneOptions {
  /** The component's data-variant, such as "releases" for the sunrise. */
  variant?: string | undefined;
}

export interface SceneDef<S> {
  readonly name: string;
  /** Whether the scene follows the pointer (never under reduced motion). */
  readonly followsPointer: boolean;
  /** How many rise() steps the scene takes; 0 when it has no sun. */
  readonly maxRise: number;
  build(grid: Grid, rand: () => number, options: SceneOptions): S;
  paintSmooth(ctx: SmoothContext, state: S, frame: Frame, dpr: number): void;
  paintPixels(buf: PixelBuffer, state: S, frame: Frame): void;
}

export interface Disconnect {
  disconnect(): void;
}

export interface SceneEnv {
  now(): number;
  requestFrame(callback: (time: number) => void): number;
  cancelFrame(handle: number): void;
  reducedMotion(): boolean;
  /** Calls back with whether the target is on screen; undefined where unsupported. */
  observeVisibility: ((target: unknown, callback: (visible: boolean) => void) => Disconnect) | undefined;
  /** Calls back when the target's size changes; undefined where unsupported. */
  observeSize: ((target: unknown, callback: () => void) => Disconnect) | undefined;
  /** Adds a passive window listener and returns its removal. */
  listen(type: 'pointermove', handler: (event: { clientX: number }) => void): () => void;
  viewport(): { width: number; dpr: number };
}

export interface SceneController {
  readonly name: string;
  readonly reducedMotion: boolean;
  /** Frames drawn so far. */
  readonly frames: number;
  /** Whether the animation loop is running (on screen and motion allowed). */
  readonly running: boolean;
  /** The sun rises one step (sunrise only; a no-op elsewhere). */
  rise(): void;
  destroy(): void;
}

function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

/** Mounts a scene on its two canvases and returns its controller. */
export function mountScene<S>(
  host: SceneHost,
  def: SceneDef<S>,
  env: SceneEnv,
  options: SceneOptions = {},
): SceneController {
  const reduced = env.reducedMotion();
  const smoothCtx = host.smooth.getContext('2d', { alpha: false }) as SmoothContext | null;
  const pixelCtx = host.pixels.getContext('2d') as PixelContext | null;

  let grid: Grid | null = null;
  let state: S | null = null;
  let image: PixelImage | null = null;
  let buffer: PixelBuffer | null = null;
  let dpr = 1;
  let frames = 0;
  let ready = false;
  let running = false;
  let handle = 0;
  let last = Number.NEGATIVE_INFINITY;
  let sceneTime = 0;
  let pointerX = 0;
  let pointerTarget = 0;
  let scroll = 0;
  let riseNow = 0;
  let riseTarget = 0;
  const cleanups: (() => void)[] = [];

  function rebuild(): boolean {
    if (!smoothCtx || !pixelCtx) return false;
    const rect = host.root.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return false;
    const next = gridFor(rect.width, rect.height);
    if (grid && next.width === grid.width && next.height === grid.height) return false;
    grid = next;
    dpr = Math.min(MAX_DPR, Math.max(1, env.viewport().dpr || 1));
    host.smooth.width = Math.round(next.width * dpr);
    host.smooth.height = Math.round(next.height * dpr);
    host.smooth.style.width = `${next.width}px`;
    host.smooth.style.height = `${next.height}px`;
    host.pixels.width = next.cols;
    host.pixels.height = next.rows;
    // Whole cells only: the buffer is scaled by exactly px, and the root's overflow cuts the rest.
    host.pixels.style.width = `${next.cols * next.px}px`;
    host.pixels.style.height = `${next.rows * next.px}px`;
    image = pixelCtx.createImageData(next.cols, next.rows);
    buffer = new PixelBuffer(image);
    state = def.build(next, mulberry32(SCENE_SEED), options);
    return true;
  }

  function draw(): void {
    if (!smoothCtx || !pixelCtx || !state || !buffer || !image) return;
    const frame: Frame = { t: sceneTime, pointerX, scroll, rise: riseNow };
    def.paintSmooth(smoothCtx, state, frame, dpr);
    buffer.clear();
    def.paintPixels(buffer, state, frame);
    pixelCtx.putImageData(image, 0, 0);
    frames++;
    if (!ready) {
      ready = true;
      host.root.setAttribute('data-ready', '');
    }
  }

  function tick(now: number): void {
    handle = 0;
    if (!running) return;
    handle = env.requestFrame(tick);
    // Frame skipping: at most MAX_FPS frames a second whatever the display's refresh rate.
    if (now - last < FRAME_MS) return;
    const dt = last === Number.NEGATIVE_INFINITY ? 0 : Math.min(0.1, (now - last) / 1000);
    last = now;
    sceneTime += dt;
    const ease = 1 - Math.exp(-3 * dt);
    pointerX += (pointerTarget - pointerX) * ease;
    const step = dt / RISE_SECONDS;
    riseNow = riseNow < riseTarget ? Math.min(riseTarget, riseNow + step) : riseTarget;
    const rect = host.root.getBoundingClientRect();
    scroll = rect.height > 0 ? clamp01(-rect.top / rect.height) : 0;
    draw();
  }

  function setRunning(on: boolean): void {
    if (on === running) return;
    running = on;
    if (on) {
      last = Number.NEGATIVE_INFINITY;
      handle = env.requestFrame(tick);
    } else if (handle) {
      env.cancelFrame(handle);
      handle = 0;
    }
  }

  if (smoothCtx && pixelCtx) {
    rebuild();
    draw();

    if (env.observeSize) {
      const sizes = env.observeSize(host.root, () => {
        if (rebuild()) draw();
      });
      cleanups.push(() => sizes.disconnect());
    }

    if (!reduced) {
      if (env.observeVisibility) {
        const visibility = env.observeVisibility(host.root, (visible) => setRunning(visible));
        cleanups.push(() => visibility.disconnect());
      }
      if (def.followsPointer) {
        cleanups.push(
          env.listen('pointermove', (event) => {
            const width = env.viewport().width;
            if (width > 0) pointerTarget = Math.max(-1, Math.min(1, (event.clientX / width) * 2 - 1));
          }),
        );
      }
    }
  }

  return {
    name: def.name,
    reducedMotion: reduced,
    get frames() {
      return frames;
    },
    get running() {
      return running;
    },
    rise() {
      if (def.maxRise <= 0 || riseTarget >= def.maxRise) return;
      riseTarget += 1;
      // Without a running loop (reduced motion, or off screen) the step lands at once.
      if (!running) {
        riseNow = riseTarget;
        draw();
      }
    },
    destroy() {
      setRunning(false);
      for (const cleanup of cleanups.splice(0)) cleanup();
    },
  };
}

/** The parts of `window` the runtime touches. */
export interface WindowLike {
  performance: { now(): number };
  requestAnimationFrame(callback: (time: number) => void): number;
  cancelAnimationFrame(handle: number): void;
  matchMedia(query: string): { matches: boolean };
  addEventListener(type: 'pointermove', listener: (event: { clientX: number }) => void, options?: { passive?: boolean }): void;
  removeEventListener(type: 'pointermove', listener: (event: { clientX: number }) => void): void;
  innerWidth: number;
  devicePixelRatio: number;
  IntersectionObserver?:
    | (new (
        callback: (entries: readonly { isIntersecting: boolean }[]) => void,
        options?: { rootMargin?: string },
      ) => Disconnect & { observe(target: never): void })
    | undefined;
  ResizeObserver?: (new (callback: () => void) => Disconnect & { observe(target: never): void }) | undefined;
}

export const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';

/** The runtime's view of a real browser window. */
export function browserEnv(win: WindowLike): SceneEnv {
  const IO = win.IntersectionObserver;
  const RO = win.ResizeObserver;
  return {
    now: () => win.performance.now(),
    requestFrame: (callback) => win.requestAnimationFrame(callback),
    cancelFrame: (handle) => win.cancelAnimationFrame(handle),
    reducedMotion: () => win.matchMedia(REDUCED_MOTION_QUERY).matches,
    observeVisibility: IO
      ? (target, callback) => {
          const observer = new IO(
            (entries) => {
              const entry = entries[entries.length - 1];
              if (entry) callback(entry.isIntersecting);
            },
            { rootMargin: '64px 0px' },
          );
          observer.observe(target as never);
          return observer;
        }
      : undefined,
    observeSize: RO
      ? (target, callback) => {
          const observer = new RO(callback);
          observer.observe(target as never);
          return observer;
        }
      : undefined,
    listen: (type, handler) => {
      win.addEventListener(type, handler, { passive: true });
      return () => win.removeEventListener(type, handler);
    },
    viewport: () => ({ width: win.innerWidth, dpr: win.devicePixelRatio || 1 }),
  };
}
