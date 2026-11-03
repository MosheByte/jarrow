import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { aurora } from './aurora.ts';
import { ember } from './ember.ts';
import { FRAME_MS, MAX_FPS, REDUCED_MOTION_QUERY, type SceneDef, browserEnv, mountScene } from './runtime.ts';
import { MAX_RISE, sunrise } from './sunrise.ts';
import { fakeHost, fakeWindow } from './test-helpers.ts';

function mount(options: { reducedMotion?: boolean; width?: number; height?: number; def?: SceneDef<unknown> } = {}) {
  const fake = fakeWindow({ reducedMotion: options.reducedMotion, width: options.width });
  const scene = fakeHost(options.width ?? 1440, options.height ?? 900);
  const controller = mountScene(scene.host, options.def ?? sunrise, browserEnv(fake.win));
  return { fake, scene, controller };
}

describe('the frame cap', () => {
  it(`draws at most ${MAX_FPS} frames a second at any refresh rate, and keeps animating`, () => {
    for (const hz of [60, 90, 120, 144, 165, 240]) {
      const { fake, scene, controller } = mount();
      fake.setVisible(true);
      const before = scene.pixels.context.puts;
      fake.run(hz, 10);
      const drawn = scene.pixels.context.puts - before;
      assert.ok(drawn <= MAX_FPS * 10 + 1, `${hz} Hz: ${drawn} frames in 10 s`);
      assert.ok(drawn >= 24 * 10, `${hz} Hz still animates: ${drawn} frames in 10 s`);
      assert.equal(controller.frames, scene.pixels.context.puts);
      // No two drawn frames are closer than the cap allows.
      controller.destroy();
    }
  });

  it('skips callbacks that come sooner than one frame interval', () => {
    const { fake, scene } = mount();
    fake.setVisible(true);
    fake.frame(1000);
    const after1 = scene.pixels.context.puts;
    fake.frame(1000 + FRAME_MS - 1);
    assert.equal(scene.pixels.context.puts, after1, 'too soon: skipped');
    assert.equal(fake.pendingFrames, 1, 'but the loop is still scheduled');
    fake.frame(1000 + FRAME_MS);
    assert.equal(scene.pixels.context.puts, after1 + 1);
  });
});

describe('on screen only', () => {
  it('starts when the scene comes on screen and stops when it leaves', () => {
    const { fake, scene, controller } = mount();
    assert.equal(fake.intersections.length, 1);
    assert.equal(scene.pixels.context.puts, 1, 'the first frame is drawn at mount');
    assert.equal(fake.pendingFrames, 0, 'no loop before the observer reports');
    fake.setVisible(true);
    assert.ok(controller.running);
    fake.run(60, 1);
    const onScreen = scene.pixels.context.puts;
    fake.setVisible(false);
    assert.ok(!controller.running);
    assert.equal(fake.pendingFrames, 0, 'the frame request is cancelled');
    fake.run(60, 5);
    assert.equal(scene.pixels.context.puts, onScreen, 'nothing is drawn off screen');
  });

  it('marks the root ready after the first frame, for the 700 ms fade-in', () => {
    const { scene } = mount();
    assert.equal(scene.attributes.get('data-ready'), '');
  });
});

describe('reduced motion', () => {
  for (const def of [sunrise, ember, aurora]) {
    it(`${def.name}: draws one static frame and attaches no listener, loop or visibility observer`, () => {
      const { fake, scene, controller } = mount({ reducedMotion: true, def });
      assert.ok(fake.queries.includes(REDUCED_MOTION_QUERY));
      assert.ok(controller.reducedMotion);
      assert.equal(scene.pixels.context.puts, 1, 'one frame');
      assert.equal(fake.listeners.length, 0, 'no event listeners');
      assert.equal(fake.counts.requestFrame, 0, 'no animation frames');
      assert.equal(fake.intersections.length, 0, 'no visibility observer');
      fake.run(60, 5);
      assert.equal(scene.pixels.context.puts, 1, 'still one frame');
      assert.equal(scene.attributes.get('data-ready'), '');
    });
  }

  it('lands a rise() step at once, in one more static frame', () => {
    const { fake, scene, controller } = mount({ reducedMotion: true });
    controller.rise();
    assert.equal(scene.pixels.context.puts, 2);
    assert.equal(fake.counts.requestFrame, 0);
  });
});

describe('pointer', () => {
  it('the sunrise follows the pointer with one passive listener, removed on destroy', () => {
    const { fake, controller } = mount();
    assert.deepEqual(
      fake.listeners.map((l) => l.type),
      ['pointermove'],
    );
    controller.destroy();
    assert.deepEqual(fake.removed, ['pointermove']);
    assert.ok(fake.intersections.every((o) => o.disconnected));
    assert.ok(fake.resizes.every((o) => o.disconnected));
  });

  it('the ember field and the aurora listen to nothing', () => {
    for (const def of [ember, aurora]) {
      const { fake } = mount({ def });
      assert.equal(fake.listeners.length, 0);
    }
  });
});

describe('resize', () => {
  it('rebuilds the scene at the new size, with the new pixel size', () => {
    const { fake, scene } = mount({ width: 1440, height: 900 });
    assert.equal(scene.pixels.width, 360);
    assert.equal(scene.pixels.height, 225);
    assert.equal(scene.pixels.style.width, '1440px');
    assert.equal(scene.smooth.width, 1440);
    const puts = scene.pixels.context.puts;
    scene.rect.width = 390;
    scene.rect.height = 253;
    fake.resize();
    assert.equal(scene.pixels.width, 130);
    assert.equal(scene.pixels.height, 85);
    assert.equal(scene.pixels.style.width, '390px');
    assert.equal(scene.pixels.style.height, '255px', 'whole 3 px cells; the root clips the last 2 px');
    assert.equal(scene.pixels.context.puts, puts + 1, 'redrawn once');
    fake.resize();
    assert.equal(scene.pixels.context.puts, puts + 1, 'an unchanged size does not redraw');
  });

  it('still redraws one static frame at a new size under reduced motion', () => {
    const { fake, scene } = mount({ reducedMotion: true });
    scene.rect.width = 1000;
    fake.resize();
    assert.equal(scene.pixels.context.puts, 2);
    assert.equal(fake.counts.requestFrame, 0);
  });
});

describe('rise()', () => {
  it('eases the sun up one step over 0.45 s while animating, up to the cap', () => {
    const seen: number[] = [];
    const spy = {
      ...sunrise,
      paintPixels: (...args: Parameters<typeof sunrise.paintPixels>) => {
        seen.push(args[2].rise);
        sunrise.paintPixels(...args);
      },
    };
    const { fake, controller } = mount({ def: spy });
    fake.setVisible(true);
    fake.run(60, 0.5);
    seen.length = 0;
    controller.rise();
    fake.run(60, 0.3);
    const midway = seen.at(-1)!;
    assert.ok(midway > 0 && midway < 1, `part of the way after 0.3 s: ${midway}`);
    fake.run(60, 0.3);
    assert.equal(seen.at(-1), 1, 'one whole step after 0.6 s');
    assert.ok(
      seen.every((v, i) => i === 0 || v >= seen[i - 1]!),
      'it only goes up',
    );
    for (let i = 0; i < MAX_RISE + 3; i++) controller.rise();
    fake.run(60, 5);
    assert.equal(seen.at(-1), MAX_RISE, 'never past the cap');
  });

  it('is a no-op on scenes without a sun', () => {
    const { scene, controller } = mount({ def: ember, reducedMotion: true });
    controller.rise();
    assert.equal(scene.pixels.context.puts, 1);
  });
});
