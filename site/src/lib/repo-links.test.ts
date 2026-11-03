import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { rewriteRepoLinks } from './repo-links.ts';

const BLOB = 'https://github.com/MosheByte/jarrow/blob/main';

describe('rewriteRepoLinks', () => {
  it('points relative file links at GitHub', () => {
    assert.equal(
      rewriteRepoLinks('<a href="BUILD-BOOK.md">book</a>'),
      `<a href="${BLOB}/BUILD-BOOK.md">book</a>`,
    );
  });

  it('keeps fragments on relative links', () => {
    assert.equal(
      rewriteRepoLinks('<a href="docs/legal.md#opt-out">x</a>'),
      `<a href="${BLOB}/docs/legal.md#opt-out">x</a>`,
    );
  });

  it('resolves against the source directory', () => {
    assert.equal(rewriteRepoLinks('<a href="../LICENSE">x</a>', 'docs'), `<a href="${BLOB}/LICENSE">x</a>`);
  });

  it('uses /tree/ for directories', () => {
    assert.equal(
      rewriteRepoLinks('<a href="crates/">x</a>'),
      '<a href="https://github.com/MosheByte/jarrow/tree/main/crates/">x</a>',
    );
  });

  it('leaves absolute, site-absolute, fragment and mailto links alone', () => {
    const html = [
      '<a href="https://github.com/users/MosheByte/projects/1">p</a>',
      '<a href="/docs/">d</a>',
      '<a href="#milestones">m</a>',
      '<a href="mailto:someone@example.com">e</a>',
      '<a href="//example.com/x">e</a>',
    ].join('');
    assert.equal(rewriteRepoLinks(html), html);
  });

  it('handles other attributes before href and single quotes', () => {
    assert.equal(
      rewriteRepoLinks(`<a class="x" href='ROADMAP.md'>r</a>`),
      `<a class="x" href='${BLOB}/ROADMAP.md'>r</a>`,
    );
  });
});
