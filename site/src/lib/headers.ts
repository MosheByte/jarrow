/**
 * Security headers for every response. Static files get them from public/_headers (Pages
 * applies that file to assets only); server-rendered routes get them from src/middleware.ts.
 * headers.test.ts keeps the two in step.
 */
export const SECURITY_HEADERS: Readonly<Record<string, string>> = {
  'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
  'Permissions-Policy': 'camera=(), microphone=(), geolocation=()',
  'X-Frame-Options': 'DENY',
};

/** Adds the security headers that are not already set, and a charset to bare text/html. */
export function applySecurityHeaders(headers: Headers): void {
  for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
    if (!headers.has(name)) headers.set(name, value);
  }
  if (headers.get('Content-Type') === 'text/html') {
    headers.set('Content-Type', 'text/html; charset=utf-8');
  }
}
