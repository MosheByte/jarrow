import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

import { SECURITY_HEADERS, applySecurityHeaders } from './headers.ts';

describe('security headers', () => {
  it('public/_headers sets the same headers for static files', () => {
    const file = readFileSync(new URL('../../public/_headers', import.meta.url), 'utf8');
    const block = file.split(/\r?\n/);
    const start = block.findIndex((line) => line.trim() === '/*');
    assert.notEqual(start, -1, 'public/_headers has a /* rule');
    const declared: Record<string, string> = {};
    for (const line of block.slice(start + 1)) {
      if (!/^\s+\S/.test(line)) break;
      const [name, ...rest] = line.trim().split(':');
      declared[name!.trim()] = rest.join(':').trim();
    }
    assert.deepEqual(declared, SECURITY_HEADERS);
  });

  it('adds missing headers, keeps existing ones and fixes a bare text/html', () => {
    const headers = new Headers({ 'Content-Type': 'text/html', 'X-Frame-Options': 'SAMEORIGIN' });
    applySecurityHeaders(headers);
    assert.equal(headers.get('Content-Type'), 'text/html; charset=utf-8');
    assert.equal(headers.get('X-Frame-Options'), 'SAMEORIGIN');
    assert.equal(headers.get('X-Content-Type-Options'), 'nosniff');
  });

  it('leaves other content types alone', () => {
    const headers = new Headers({ 'Content-Type': 'application/json; charset=utf-8' });
    applySecurityHeaders(headers);
    assert.equal(headers.get('Content-Type'), 'application/json; charset=utf-8');
  });
});
