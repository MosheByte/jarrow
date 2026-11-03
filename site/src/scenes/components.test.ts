// The scene components and their CSS: markup, accessibility, motion and colour rules.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { describe, it } from 'node:test';

import { SKY, SUN } from './palette.ts';

const dir = new URL('../components/scenes/', import.meta.url);
const read = (name: string) => readFileSync(new URL(name, dir), 'utf8');
const COMPONENTS = [
  ['SunriseScene.astro', 'sunrise'],
  ['EmberScene.astro', 'ember'],
  ['AuroraScene.astro', 'aurora'],
] as const;

describe('scene components', () => {
  for (const [file, name] of COMPONENTS) {
    it(`${file}: an aria-hidden canvas pair over a CSS fallback, mounted by its own scene`, () => {
      const source = read(file);
      assert.match(source, new RegExp(`class="jw-scene jw-scene-${name}" data-scene="${name}"`));
      assert.match(source, /data-scene="[a-z]+"[^>]*aria-hidden="true"/);
      const canvases = source.match(/<canvas[^>]*>/g) ?? [];
      assert.equal(canvases.length, 2);
      assert.ok(canvases.every((c) => c.includes('aria-hidden="true"')));
      assert.ok(canvases.some((c) => c.includes('data-layer="smooth"')));
      assert.ok(canvases.some((c) => c.includes('data-layer="pixels"')));
      assert.match(source, new RegExp(`mountAll\\('${name}', ${name}\\)`));
      assert.match(source, /import '\.\/scene\.css';/);
      assert.doesNotMatch(source, /<img|\.png|\.webp|\.jpg/, 'no raster imagery');
    });
  }

  it('scene.css: pixelated top layer, 700 ms fade on data-ready, the narrow strip', () => {
    const css = read('scene.css');
    assert.match(css, /\.jw-scene > \.jw-scene-pixels \{[^}]*image-rendering: pixelated;/);
    assert.match(css, /\.jw-scene > canvas \{[^}]*opacity: 0;[^}]*transition: opacity 700ms/);
    assert.match(css, /\.jw-scene\[data-ready\] > canvas \{\s*opacity: 1;/);
    assert.match(css, /@media \(max-width: 1023\.98px\) \{\s*\.jw-scene-sunrise \{[^}]*height: clamp\(180px, 30svh, 320px\);/);
    assert.doesNotMatch(css, /text-transform/);
  });

  it('scene.css uses only stops of the 8.2 ramps', () => {
    const css = read('scene.css');
    const hexes = css.match(/#[0-9a-fA-F]{3,8}\b/g) ?? [];
    assert.ok(hexes.length > 0);
    const allowed = new Set<string>([...SKY, ...SUN]);
    for (const hex of hexes) assert.ok(allowed.has(hex.toLowerCase()), `${hex} is not a ramp stop`);
    assert.doesNotMatch(css, /rgba?\(|hsla?\(|oklch\(/, 'no colours outside hex ramp stops');
  });

  it('only the three scene components and their CSS live in components/scenes', () => {
    assert.deepEqual(readdirSync(dir).sort(), ['AuroraScene.astro', 'EmberScene.astro', 'SunriseScene.astro', 'scene.css']);
  });
});

describe('the ramps match the generated tokens, once they carry them', () => {
  const candidates = ['../styles/tokens.css', '../../tokens.css'].map((p) => new URL(p, import.meta.url));
  const found: Record<string, string> = {};
  for (const file of candidates) {
    if (!existsSync(file)) continue;
    for (const m of readFileSync(file, 'utf8').matchAll(/--[\w-]*?(sky|sun)-(\d+)\s*:\s*(#[0-9a-fA-F]{6})/g)) {
      found[`${m[1]}-${m[2]}`] = m[3]!.toLowerCase();
    }
  }
  it('every --*-sky-N and --*-sun-N custom property equals the scene ramp', (t) => {
    if (Object.keys(found).length === 0) {
      t.skip('tokens.css has no sky or sun ramp properties yet');
      return;
    }
    for (const [key, hex] of Object.entries(found)) {
      const [ramp, index] = key.split('-') as ['sky' | 'sun', string];
      const expected = (ramp === 'sky' ? SKY : SUN)[Number(index)];
      assert.equal(hex, expected, `--${key}`);
    }
  });
});
