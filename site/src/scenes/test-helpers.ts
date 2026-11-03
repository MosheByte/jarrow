/**
 * Test doubles for the scene tests: a recording 2-D context, canvases, a scene host, and a fake
 * window whose clock, frames and observers the test drives. Imported only by *.test.ts.
 */

import { type Rgb, parseCssColour } from './palette.ts';
import type { PixelImage } from './pixels.ts';
import type { CanvasLike, Disconnect, GradientLike, SceneHost, WindowLike } from './runtime.ts';

/** A 2-D context that records every colour it is given and every pixel upload. */
export class RecordingContext {
  colours: string[] = [];
  arcs: { x: number; y: number; r: number }[] = [];
  puts = 0;
  globalAlpha = 1;
  #fill: unknown = '#000000';

  get fillStyle(): unknown {
    return this.#fill;
  }
  set fillStyle(value: unknown) {
    if (typeof value === 'string') this.colours.push(value);
    this.#fill = value;
  }
  setTransform(): void {}
  fillRect(): void {}
  beginPath(): void {}
  arc(x: number, y: number, r: number): void {
    this.arcs.push({ x, y, r });
  }
  fill(): void {}
  #gradient(): GradientLike {
    return { addColorStop: (_offset: number, colour: string) => void this.colours.push(colour) };
  }
  createLinearGradient(): GradientLike {
    return this.#gradient();
  }
  createRadialGradient(): GradientLike {
    return this.#gradient();
  }
  createImageData(width: number, height: number): PixelImage {
    return { width, height, data: new Uint8ClampedArray(width * height * 4) };
  }
  putImageData(): void {
    this.puts++;
  }
  /** Every recorded colour as RGB; throws on one it cannot read. */
  rgb(): Rgb[] {
    return this.colours.map((c) => {
      const rgb = parseCssColour(c);
      if (!rgb) throw new Error(`unreadable colour ${c}`);
      return rgb;
    });
  }
}

export class FakeCanvas implements CanvasLike {
  width = 300;
  height = 150;
  style = { width: '', height: '' };
  readonly context = new RecordingContext();
  getContext(): unknown {
    return this.context;
  }
}

export function fakeHost(width: number, height: number) {
  const smooth = new FakeCanvas();
  const pixels = new FakeCanvas();
  const attributes = new Map<string, string>();
  const rect = { width, height, top: 0 };
  const host: SceneHost = {
    root: {
      getBoundingClientRect: () => ({ ...rect }),
      setAttribute: (name, value) => void attributes.set(name, value),
    },
    smooth,
    pixels,
  };
  return { host, smooth, pixels, attributes, rect };
}

class FakeObserver implements Disconnect {
  observed: unknown[] = [];
  disconnected = false;
  readonly callback: (...args: never[]) => void;
  constructor(callback: (...args: never[]) => void) {
    this.callback = callback;
  }
  observe(target: unknown): void {
    this.observed.push(target);
  }
  disconnect(): void {
    this.disconnected = true;
  }
}

/** A window the test drives: frames run when it calls frame(), observers fire when it says. */
export function fakeWindow(options: { reducedMotion?: boolean; width?: number } = {}) {
  let clock = 0;
  let nextHandle = 1;
  const pending = new Map<number, (time: number) => void>();
  const listeners: { type: string; listener: unknown }[] = [];
  const removed: string[] = [];
  const intersections: FakeObserver[] = [];
  const resizes: FakeObserver[] = [];
  const queries: string[] = [];
  const counts = { requestFrame: 0, cancelFrame: 0 };

  const win: WindowLike = {
    performance: { now: () => clock },
    requestAnimationFrame(callback) {
      counts.requestFrame++;
      const handle = nextHandle++;
      pending.set(handle, callback);
      return handle;
    },
    cancelAnimationFrame(handle) {
      counts.cancelFrame++;
      pending.delete(handle);
    },
    matchMedia(query) {
      queries.push(query);
      return { matches: Boolean(options.reducedMotion) && query.includes('prefers-reduced-motion: reduce') };
    },
    addEventListener(type, listener) {
      listeners.push({ type, listener });
    },
    removeEventListener(type) {
      removed.push(type);
    },
    innerWidth: options.width ?? 1440,
    devicePixelRatio: 1,
    IntersectionObserver: class extends FakeObserver {
      constructor(callback: (entries: readonly { isIntersecting: boolean }[]) => void) {
        super(callback);
        intersections.push(this);
      }
    },
    ResizeObserver: class extends FakeObserver {
      constructor(callback: () => void) {
        super(callback);
        resizes.push(this);
      }
    },
  };

  return {
    win,
    listeners,
    removed,
    intersections,
    resizes,
    queries,
    counts,
    get pendingFrames() {
      return pending.size;
    },
    /** Moves the clock to `time` (ms) and runs the frame callbacks due. */
    frame(time: number) {
      clock = time;
      const due = [...pending.entries()];
      pending.clear();
      for (const [, callback] of due) callback(time);
    },
    /** Runs frames at `hz` from `from` for `seconds`. */
    run(hz: number, seconds: number, from = clock) {
      const n = Math.round(hz * seconds);
      for (let i = 1; i <= n; i++) this.frame(from + (i * 1000) / hz);
    },
    setVisible(visible: boolean) {
      for (const io of intersections) (io.callback as (e: { isIntersecting: boolean }[]) => void)([{ isIntersecting: visible }]);
    },
    resize() {
      for (const ro of resizes) (ro.callback as () => void)();
    },
  };
}
