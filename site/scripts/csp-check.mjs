// Checks the built site against its Content-Security-Policy (run by `npm run build`, after
// pages-assemble.mjs).
//
// The policy (src/lib/headers.ts, copied into public/_headers) allows inline scripts only by
// SHA-256 hash. An inline script that changes, or a new one, would be blocked in the browser
// without any build error, so this script reads every built page in dist/ and fails when:
//
//   - an inline <script> has a hash that script-src does not list,
//   - an element has an inline event handler (onclick= and the like), which no hash allows,
//   - a URL attribute holds a javascript: URL,
//   - a <script src> points to another origin.
//
// A hash the policy lists but no page uses is reported as a warning: the server-rendered pages
// (/ and /board) are not in dist/, and they use the same layout as the prerendered pages.
//
//   node scripts/csp-check.mjs           check dist/ (exit 1 on a problem)
//   node scripts/csp-check.mjs --print   print the hashes the built pages need, for headers.ts

import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = fileURLToPath(new URL('..', import.meta.url));

/** The value of `name` in the `/*` rule of a Pages _headers file, or null. */
export function headerFromHeadersFile(text, name) {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === '/*');
  if (start === -1) return null;
  for (const line of lines.slice(start + 1)) {
    if (!/^\s+\S/.test(line)) break;
    const colon = line.indexOf(':');
    if (colon !== -1 && line.slice(0, colon).trim().toLowerCase() === name.toLowerCase()) {
      return line.slice(colon + 1).trim();
    }
  }
  return null;
}

/** The source list of one directive of a policy, as an array of tokens. */
export function directive(policy, name) {
  for (const part of policy.split(';')) {
    const [key, ...values] = part.trim().split(/\s+/);
    if (key && key.toLowerCase() === name) return values;
  }
  return [];
}

/** 'sha256-<base64>' of a script's text, exactly as the browser hashes it (UTF-8 bytes). */
export function sha256(text) {
  return `sha256-${createHash('sha256').update(text, 'utf8').digest('base64')}`;
}

const SCRIPT = /<script\b([^>]*)>([\s\S]*?)<\/script\s*>/gi;
const SRC = /\bsrc\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/i;
const HANDLER = /<[a-zA-Z][^>]*?\son[a-z]+\s*=/g;
const JS_URL = /\b(?:href|src|action|formaction|data)\s*=\s*["']?\s*javascript:/gi;

/** Inline scripts of a page: [{ hash, preview }]; scripts with src= are not inline. */
export function inlineScripts(html) {
  const found = [];
  for (const match of html.matchAll(SCRIPT)) {
    const [, attrs, body] = match;
    if (SRC.test(attrs) || body.trim() === '') continue;
    found.push({ hash: sha256(body), preview: body.trim().slice(0, 60).replace(/\s+/g, ' ') });
  }
  return found;
}

/** External script URLs of a page (anything that is not same-origin). */
export function externalScripts(html) {
  const urls = [];
  for (const match of html.matchAll(SCRIPT)) {
    const src = SRC.exec(match[1]);
    const url = src && (src[1] ?? src[2] ?? src[3]);
    if (url && /^(?:[a-z][a-z0-9+.-]*:)?\/\//i.test(url)) urls.push(url);
  }
  return urls;
}

/**
 * Problems of the built pages against a policy: { errors: string[], warnings: string[],
 * needed: string[] } where needed is every inline-script hash the pages use, sorted.
 */
export function checkPages(pages, policy) {
  const allowed = new Set(directive(policy, 'script-src').map((token) => token.replace(/^'|'$/g, '')));
  const errors = [];
  const needed = new Map();
  for (const [name, html] of pages) {
    for (const { hash, preview } of inlineScripts(html)) {
      if (!needed.has(hash)) needed.set(hash, { page: name, preview });
      if (!allowed.has(hash)) errors.push(`${name}: inline script ${hash} is not in script-src ("${preview}...")`);
    }
    for (const handler of html.match(HANDLER) ?? []) {
      errors.push(`${name}: inline event handler (${handler.slice(0, 60)}...); the policy blocks it`);
    }
    if (JS_URL.test(html)) errors.push(`${name}: a javascript: URL; the policy blocks it`);
    JS_URL.lastIndex = 0;
    for (const url of externalScripts(html)) errors.push(`${name}: script from another origin (${url})`);
  }
  const warnings = [...allowed]
    .filter((token) => token.startsWith('sha256-') && !needed.has(token))
    .map((hash) => `script-src lists ${hash}, which no prerendered page uses (a server-rendered page may)`);
  return { errors: [...new Set(errors)], warnings, needed: [...needed.keys()].sort() };
}

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

function main() {
  const dist = join(siteRoot, 'dist');
  const headersFile = join(dist, '_headers');
  if (!existsSync(headersFile)) {
    console.error('csp-check: dist/_headers not found; run `npm run build` first');
    process.exit(1);
  }
  const policy = headerFromHeadersFile(readFileSync(headersFile, 'utf8'), 'Content-Security-Policy');
  if (!policy) {
    console.error('csp-check: dist/_headers has no Content-Security-Policy in its /* rule');
    process.exit(1);
  }
  const pages = walk(dist)
    .filter((file) => file.endsWith('.html') && !file.includes(`${sep}_worker.js${sep}`))
    .map((file) => [relative(dist, file).split(sep).join('/'), readFileSync(file, 'utf8')]);
  const { errors, warnings, needed } = checkPages(pages, policy);
  if (process.argv.includes('--print')) {
    for (const hash of needed) console.log(`  '${hash}',`);
    return;
  }
  for (const warning of warnings) console.warn(`csp-check: warning: ${warning}`);
  if (errors.length > 0) {
    for (const error of errors) console.error(`csp-check: ${error}`);
    console.error(
      'csp-check: update SCRIPT_HASHES in src/lib/headers.ts and the Content-Security-Policy line in ' +
        'public/_headers (the hashes the pages need: node scripts/csp-check.mjs --print)',
    );
    process.exit(1);
  }
  console.log(`csp-check: ${pages.length} pages, ${needed.length} inline scripts, all allowed by script-src`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) main();
