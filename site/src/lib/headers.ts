/**
 * Security headers for every response. Static files get them from public/_headers (Pages
 * applies that file to assets only); server-rendered routes get them from src/middleware.ts.
 * headers.test.ts keeps the two in step, and scripts/csp-check.mjs (run by `npm run build`)
 * fails the build when a built page has an inline script whose hash the policy lacks.
 */

/**
 * SHA-256 hashes of the inline scripts the build emits: the theme script in Base.astro, its
 * hoisted ThemeToggle module, and Starlight's theme, sidebar, table-of-contents and search
 * scripts. A changed inline script changes its hash; `node scripts/csp-check.mjs` prints the
 * list to paste here and into public/_headers.
 */
export const SCRIPT_HASHES: readonly string[] = [
  'sha256-4kvtJp6QbPAHTxFa7WbvD+NIIzEmHaaJW51SO2bUloY=',
  'sha256-7eCV4jtsr4t4knb3c4FCRPeu7GGZeOUGE3XvWix0XOQ=',
  'sha256-GkZBRnvSuhtx/cvzvukVkX2JJZW+DdPlVr7BX8Tefqo=',
  'sha256-VWo5Wp4aqSj6nSgMpeAp9cKieaoIfwFUAunAVugI5gA=',
  'sha256-f/zAUE74ucc3JYp4r4QQvkJofoQdkOIhHYK+jeZ6eko=',
  'sha256-wX2yOADeV+NMngflD5uYi3vl50SHC4sfM1EmylVjlX4=',
  'sha256-1Yn9jif1l+hrdSyUfQQkLcGW0Db99ewrLVypfxnCTso=',
  'sha256-VgrgcKBvIM3XpiLJ6fEv1Vy6YHBwlKLkYtFdfBMouTo=',
  'sha256-hP48sUywTAXyg22qvpdOC/cYIA8MfPdKEbPC/mvGk4M=',
  'sha256-nVO98WGdJA5yup1/MAdukGqUlS6Psuf7JJFF5ikWqbQ=',
  'sha256-ucPfdYc9FPIvbmnn9ECv3eLKYLt8/heMPFQ+hNTn6jY=',
];

/**
 * The Content-Security-Policy, one directive per entry:
 * - scripts only from this origin or with a listed hash; 'wasm-unsafe-eval' lets the docs
 *   search (Pagefind) compile its WebAssembly, and nothing else (no eval, no inline handlers);
 * - styles from this origin plus inline styles, because Starlight writes style attributes
 *   (`style="--depth: 0"`) and Astro inlines small component stylesheets;
 * - images from this origin, data: URIs (icons in the CSS), GitHub avatars (/about,
 *   /contributors) and github.com (the CI badge on the home page);
 * - fonts from this origin and data: URIs (one bundled font face is inlined);
 * - fetches and the search worker from this origin only; no frames, no plugins, no
 *   cross-origin form posts, and no page may frame this site.
 */
export const CONTENT_SECURITY_POLICY = [
  "default-src 'none'",
  `script-src 'self' 'wasm-unsafe-eval' ${SCRIPT_HASHES.map((hash) => `'${hash}'`).join(' ')}`,
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: https://avatars.githubusercontent.com https://github.com",
  "font-src 'self' data:",
  "connect-src 'self'",
  "worker-src 'self'",
  "manifest-src 'self'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
].join('; ');

export const SECURITY_HEADERS: Readonly<Record<string, string>> = {
  'Strict-Transport-Security': 'max-age=31536000; includeSubDomains',
  'Content-Security-Policy': CONTENT_SECURITY_POLICY,
  'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
  'Permissions-Policy':
    'camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(), hid=(), browsing-topics=()',
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
