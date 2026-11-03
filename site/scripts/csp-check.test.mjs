import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

import { checkPages, directive, externalScripts, headerFromHeadersFile, inlineScripts, sha256 } from './csp-check.mjs';

const HEADERS = readFileSync(new URL('../public/_headers', import.meta.url), 'utf8');

describe('csp-check', () => {
  it('hashes inline scripts the way browsers do', () => {
    // The CSP spec example: sha256 of the UTF-8 bytes of the script's text.
    assert.equal(sha256("alert('Hello, world.');"), 'sha256-qznLcsROx4GACP2dm0UCKCzCG+HiZ1guq6ZZDob/Tng=');
    const page = '<script>a()</script><script type="module">b()</script><script src="/x.js"></script><script> </script>';
    assert.deepEqual(
      inlineScripts(page).map((s) => s.hash),
      [sha256('a()'), sha256('b()')],
    );
  });

  it('reads the policy from public/_headers', () => {
    const policy = headerFromHeadersFile(HEADERS, 'Content-Security-Policy');
    assert.ok(policy);
    assert.deepEqual(directive(policy, 'default-src'), ["'none'"]);
    assert.deepEqual(directive(policy, 'frame-ancestors'), ["'none'"]);
    const scripts = directive(policy, 'script-src');
    assert.ok(!scripts.includes("'unsafe-inline'") && !scripts.includes("'unsafe-eval'"));
    assert.ok(scripts.filter((token) => token.startsWith("'sha256-")).length > 0);
  });

  it('fails on a script the policy does not allow, handlers and javascript: URLs', () => {
    const policy = "default-src 'none'; script-src 'self' 'sha256-AAAA'";
    const pages = [
      ['ok.html', '<p>no scripts</p>'],
      ['bad.html', '<script>evil()</script><button onclick="x()">x</button><a href="javascript:void(0)">y</a>'],
      ['ext.html', '<script src="https://cdn.example/x.js"></script><script src="//cdn.example/y.js"></script>'],
    ];
    const { errors, warnings, needed } = checkPages(pages, policy);
    assert.equal(errors.length, 5, errors.join('\n'));
    assert.ok(errors.some((e) => e.includes(sha256('evil()'))));
    assert.ok(errors.some((e) => e.includes('inline event handler')));
    assert.ok(errors.some((e) => e.includes('javascript: URL')));
    assert.deepEqual(externalScripts(pages[2][1]), ['https://cdn.example/x.js', '//cdn.example/y.js']);
    assert.deepEqual(needed, [sha256('evil()')]);
    assert.deepEqual(warnings.length, 1);
  });

  it('passes when every inline script is listed', () => {
    const policy = `script-src 'self' '${sha256('a()')}'`;
    const { errors } = checkPages([['p.html', '<script>a()</script><script src="/app.js"></script>']], policy);
    assert.deepEqual(errors, []);
  });
});
