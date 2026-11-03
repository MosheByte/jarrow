// Static preview harness for the scenes in src/scenes. Used only for screenshots and tests:
// it lives outside src/pages and public/, so Astro never builds or deploys it.
//
//   node scripts/scene-harness/server.mjs [--port 4790]
//
// It serves one page per scene with the same markup and CSS as the scene components, and the
// scene modules themselves with their TypeScript types stripped by Node (no bundler needed).

import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const SCENES_DIR = new URL('../../src/scenes/', import.meta.url);
const SCENE_CSS = new URL('../../src/components/scenes/scene.css', import.meta.url);

/** The scenes the harness can show, with the height of the section each one lives in. */
export const SCENES = {
  sunrise: { module: 'sunrise.ts', height: '100svh', variants: ['home', 'releases'] },
  ember: { module: 'ember.ts', height: 'min(640px, 72svh)', variants: [] },
  aurora: { module: 'aurora.ts', height: 'min(540px, 64svh)', variants: [] },
};

const MODULE_NAME = /^[a-z][a-z0-9-]*\.ts$/;

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function indexPage() {
  const links = Object.entries(SCENES)
    .flatMap(([name, scene]) =>
      (scene.variants.length ? scene.variants : [null]).map((variant) => {
        const query = variant ? `?name=${name}&variant=${variant}` : `?name=${name}`;
        return `<li><a href="/scene${query}">${escapeHtml(variant ? `${name} (${variant})` : name)}</a></li>`;
      }),
    )
    .join('\n');
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Scene harness</title></head>
<body><h1>Scene harness</h1><p>Not deployed. One page per scene.</p><ul>
${links}
</ul></body></html>`;
}

function scenePage(name, variant, css) {
  const scene = SCENES[name];
  const variantAttr = variant ? ` data-variant="${escapeHtml(variant)}"` : '';
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Scene harness: ${escapeHtml(name)}</title>
<style>
html, body { margin: 0; background: #0b0f1f; }
.hero { position: relative; overflow: hidden; isolation: isolate; height: ${scene.height}; }
${css}
</style>
</head>
<body>
<section class="hero">
  <div class="jw-scene jw-scene-${escapeHtml(name)}" data-scene="${escapeHtml(name)}"${variantAttr} aria-hidden="true">
    <canvas class="jw-scene-smooth" data-layer="smooth" aria-hidden="true"></canvas>
    <canvas class="jw-scene-pixels" data-layer="pixels" aria-hidden="true"></canvas>
  </div>
</section>
<script type="module">
  import { mountAll } from '/scenes/boot.ts';
  import { ${name} } from '/scenes/${scene.module}';
  window.__scenes = mountAll('${name}', ${name});
</script>
</body>
</html>`;
}

function send(res, status, type, body) {
  res.writeHead(status, { 'Content-Type': type, 'Cache-Control': 'no-store' });
  res.end(body);
}

async function handle(req, res) {
  const url = new URL(req.url ?? '/', 'http://harness.invalid');
  if (req.method !== 'GET') return send(res, 405, 'text/plain; charset=utf-8', 'GET only\n');
  if (url.pathname === '/') return send(res, 200, 'text/html; charset=utf-8', indexPage());
  if (url.pathname === '/scene') {
    const name = url.searchParams.get('name') ?? '';
    if (!Object.hasOwn(SCENES, name)) return send(res, 404, 'text/plain; charset=utf-8', 'unknown scene\n');
    const variant = url.searchParams.get('variant');
    if (variant && !SCENES[name].variants.includes(variant)) {
      return send(res, 404, 'text/plain; charset=utf-8', 'unknown variant\n');
    }
    const css = await readFile(SCENE_CSS, 'utf8');
    return send(res, 200, 'text/html; charset=utf-8', scenePage(name, variant, css));
  }
  if (url.pathname.startsWith('/scenes/')) {
    const file = url.pathname.slice('/scenes/'.length);
    if (!MODULE_NAME.test(file) || file.endsWith('.test.ts')) {
      return send(res, 404, 'text/plain; charset=utf-8', 'not a scene module\n');
    }
    let source;
    try {
      source = await readFile(new URL(file, SCENES_DIR), 'utf8');
    } catch {
      return send(res, 404, 'text/plain; charset=utf-8', 'not found\n');
    }
    return send(res, 200, 'text/javascript; charset=utf-8', stripTypeScriptTypes(source, { mode: 'strip' }));
  }
  return send(res, 404, 'text/plain; charset=utf-8', 'not found\n');
}

/** Starts the harness on 127.0.0.1; port 0 picks a free one. Resolves to its URL and a close(). */
export function startHarness({ port = 0 } = {}) {
  const server = createServer((req, res) => {
    handle(req, res).catch((error) => send(res, 500, 'text/plain; charset=utf-8', `${error}\n`));
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => {
      const address = server.address();
      resolve({
        url: `http://127.0.0.1:${address.port}`,
        close: () => new Promise((done) => server.close(() => done())),
      });
    });
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const flag = process.argv.indexOf('--port');
  const port = flag > 0 ? Number(process.argv[flag + 1]) : 4790;
  const { url } = await startHarness({ port });
  console.log(`scene harness on ${url} (from ${fileURLToPath(SCENES_DIR)})`);
}
