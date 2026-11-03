// The scene harness serves the scene pages and type-stripped modules, and is never deployed.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import { SCENES, startHarness } from './server.mjs';

const siteRoot = fileURLToPath(new URL('../../', import.meta.url));

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

describe('scene harness', () => {
  let harness;
  before(async () => {
    process.removeAllListeners('warning');
    harness = await startHarness();
  });
  after(() => harness.close());

  it('lists every scene and serves a page for each', async () => {
    const index = await (await fetch(`${harness.url}/`)).text();
    for (const name of Object.keys(SCENES)) {
      assert.ok(index.includes(`name=${name}`), name);
      const res = await fetch(`${harness.url}/scene?name=${name}`);
      assert.equal(res.status, 200);
      const html = await res.text();
      assert.ok(html.includes(`data-scene="${name}"`));
      assert.ok(html.includes('.jw-scene > .jw-scene-pixels'), 'the component CSS is inlined');
    }
  });

  it('serves scene modules as JavaScript with the types stripped', async () => {
    const res = await fetch(`${harness.url}/scenes/runtime.ts`);
    assert.equal(res.status, 200);
    assert.match(res.headers.get('content-type'), /^text\/javascript/);
    const js = await res.text();
    assert.ok(js.includes('export function mountScene'));
    assert.ok(!/\binterface\s+SceneEnv\b/.test(js), 'no TypeScript left');
  });

  it('refuses anything that is not a scene module', async () => {
    for (const path of [
      '/scenes/runtime.test.ts',
      '/scenes/..%2F..%2Fpackage.json',
      '/scenes/../../package.json',
      '/scene?name=__proto__',
      '/scene?name=sunrise&variant=nope',
      '/package.json',
    ]) {
      const res = await fetch(`${harness.url}${path}`);
      assert.equal(res.status, 404, path);
    }
  });
});

describe('never deployed', () => {
  it('nothing Astro builds refers to the harness', () => {
    for (const dir of ['src', 'public']) {
      for (const file of walk(join(siteRoot, dir))) {
        if (/\.test\.(ts|mjs)$/.test(file)) continue;
        assert.ok(!readFileSync(file, 'utf8').includes('scene-harness'), file);
      }
    }
  });
});
