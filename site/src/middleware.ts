import { defineMiddleware } from 'astro:middleware';

import { applySecurityHeaders } from './lib/headers.ts';

// Server-rendered responses do not get public/_headers on Pages, so they get the same
// security headers here. Prerendered pages are served as static files with _headers.
export const onRequest = defineMiddleware(async (context, next) => {
  const response = await next();
  if (context.isPrerendered) return response;
  try {
    applySecurityHeaders(response.headers);
    return response;
  } catch {
    // Some responses have immutable headers; copy them into a new response.
    const copy = new Response(response.body, response);
    applySecurityHeaders(copy.headers);
    return copy;
  }
});
