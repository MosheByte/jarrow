import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { formatList } from './format.ts';
import { GAME_VERSIONS } from './site.ts';

describe('formatList', () => {
  it('joins the pinned game versions as plain English', () => {
    assert.equal(formatList(GAME_VERSIONS), '1.20.1, 1.21.1 and 1.21.11');
  });

  it('handles short lists', () => {
    assert.equal(formatList([]), '');
    assert.equal(formatList(['1.21.1']), '1.21.1');
    assert.equal(formatList(['1.21.1', '1.21.11']), '1.21.1 and 1.21.11');
  });
});
