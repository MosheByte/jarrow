// robots.txt, built from SITE_URL so the sitemap link follows the domain.
import type { APIRoute } from 'astro';

import { absolute } from '../lib/site.ts';

export const GET: APIRoute = () =>
  new Response(`User-agent: *\nAllow: /\n\nSitemap: ${absolute('/sitemap-index.xml')}\n`, {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
