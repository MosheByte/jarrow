import assert from 'node:assert/strict';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';

import { routesFor } from './pages-assemble.mjs';

const pages = fileURLToPath(new URL('../src/pages/', import.meta.url));
const file = (rel) => join(pages, ...rel.split('/'));

describe('routesFor', () => {
  it('maps the index page to the root only', () => {
    assert.deepEqual(routesFor(file('index.astro')), ['/']);
  });

  it('maps pages to both slash forms', () => {
    assert.deepEqual(routesFor(file('board.astro')), ['/board', '/board/']);
    assert.deepEqual(routesFor(file('board/index.astro')), ['/board', '/board/']);
  });

  it('keeps endpoint paths exact', () => {
    assert.deepEqual(routesFor(file('api/board.json.ts')), ['/api/board.json']);
  });

  it('turns dynamic segments into a wildcard', () => {
    assert.deepEqual(routesFor(file('board/[mod].astro')), ['/board/*']);
    assert.deepEqual(routesFor(file('badge/[mod].svg.ts')), ['/badge/*']);
  });

  it('ignores files that are not routes', () => {
    assert.deepEqual(routesFor(file('notes.txt')), []);
  });
});
