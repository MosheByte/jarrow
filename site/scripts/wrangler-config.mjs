// Derives the Workers-shaped wrangler config that @astrojs/cloudflare needs from the Pages
// config in wrangler.toml, so the bindings are written down once.
//
// Why two shapes: the site deploys to a Cloudflare Pages project (D-004), and wrangler.toml
// is that project's config (`pages_build_output_dir`). The current adapter builds through
// the Cloudflare Vite plugin, which only accepts a Workers config and refuses one with
// `pages_build_output_dir`. astro.config.mjs calls writeAdapterConfig() on every Astro
// command; the result is gitignored and only drives `astro dev` and `astro build`.

import { readFileSync, writeFileSync } from 'node:fs';
import { parse } from 'smol-toml';

/** Keys of wrangler.toml that only exist for Pages and are dropped for the adapter. */
const PAGES_ONLY_KEYS = new Set(['pages_build_output_dir']);

/** Name of the generated file, relative to site/. */
export const ADAPTER_CONFIG = 'wrangler.adapter.jsonc';

/**
 * @param {URL} siteRoot URL of the site/ directory, with a trailing slash.
 * @returns {string} Path of the generated config, relative to siteRoot.
 */
export function writeAdapterConfig(siteRoot) {
  const source = parse(readFileSync(new URL('wrangler.toml', siteRoot), 'utf8'));
  if (typeof source.pages_build_output_dir !== 'string') {
    throw new Error('site/wrangler.toml must set pages_build_output_dir: it is the Pages project config.');
  }

  const config = Object.fromEntries(Object.entries(source).filter(([key]) => !PAGES_ONLY_KEYS.has(key)));
  const body =
    '// Generated from wrangler.toml by scripts/wrangler-config.mjs on every Astro command.\n' +
    '// Do not edit: change wrangler.toml instead. This file is gitignored.\n' +
    `${JSON.stringify(config, null, 2)}\n`;

  const targetUrl = new URL(ADAPTER_CONFIG, siteRoot);
  let current = '';
  try {
    current = readFileSync(targetUrl, 'utf8');
  } catch {
    // First run: the file does not exist yet.
  }
  // Rewrite only on change, so the dev server does not restart for nothing.
  if (current !== body) writeFileSync(targetUrl, body);
  return ADAPTER_CONFIG;
}
