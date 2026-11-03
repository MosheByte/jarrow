// @ts-check
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import cloudflare from '@astrojs/cloudflare';
import sitemap from '@astrojs/sitemap';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';

import { writeAdapterConfig } from './scripts/wrangler-config.mjs';
import { DESCRIPTION, PRODUCT_NAME, REPO_URL, SITE_URL } from './src/lib/site.ts';

const siteRoot = new URL('./', import.meta.url);
const repoRoot = new URL('../', import.meta.url);

// /roadmap renders the repository's ROADMAP.md at build time. Without it, fail loudly:
// the site never invents a roadmap.
const roadmap = new URL('ROADMAP.md', repoRoot);
if (!existsSync(roadmap)) {
  throw new Error(
    `ROADMAP.md not found at ${fileURLToPath(roadmap)}. The /roadmap page renders it at build time; restore the file before building the site.`,
  );
}

export default defineConfig({
  site: SITE_URL,
  output: 'static',
  trailingSlash: 'ignore',
  // Lossless whitespace compression. Astro 7's default ('jsx') drops the space between a line
  // of text and an inline element on the next line ("on the<a>roadmap</a>").
  compressHTML: true,
  // No sessions: the site has no accounts before milestone M4.
  session: false,
  adapter: cloudflare({
    // Bindings are declared once, in wrangler.toml (the Pages config); see scripts/wrangler-config.mjs.
    configPath: writeAdapterConfig(siteRoot),
    // The site uses no images, and Pages has no Images binding.
    imageService: 'passthrough',
  }),
  integrations: [
    starlight({
      title: PRODUCT_NAME,
      description: DESCRIPTION,
      favicon: '/favicon.svg',
      customCss: [
        './src/styles/global.css',
        '@fontsource-variable/atkinson-hyperlegible-next',
        '@fontsource-variable/bricolage-grotesque',
        '@fontsource-variable/jetbrains-mono',
        './src/styles/starlight.css',
      ],
      editLink: { baseUrl: `${REPO_URL}/edit/main/site/` },
      // The site's own 404 page (src/pages/404.astro) covers the docs too.
      disable404Route: true,
      components: {
        Footer: './src/components/starlight/Footer.astro',
        SocialIcons: './src/components/starlight/SocialIcons.astro',
      },
      head: [{ tag: 'meta', attrs: { name: 'theme-color', content: '#0b0d12' } }],
      sidebar: [
        { label: 'Start here', items: ['docs', 'docs/status', 'docs/directions', 'docs/tiers', 'docs/install'] },
        { label: 'Policy', items: ['docs/legal', 'docs/telemetry'] },
        { label: 'Help', items: ['docs/faq', 'docs/contributing'] },
      ],
    }),
    // Server-rendered pages are not discovered at build time; list them so they are in the sitemap.
    sitemap({ customPages: [`${SITE_URL}/`, `${SITE_URL}/board/`] }),
  ],
  vite: {
    plugins: [tailwindcss()],
    // ROADMAP.md sits outside site/, one level up.
    server: { fs: { allow: [fileURLToPath(repoRoot)] } },
  },
});
