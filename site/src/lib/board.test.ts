// Run with: npm test (node --test, using Node's built-in TypeScript type stripping).
import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import {
  BOARD_KV_KEY,
  type BoardJson,
  type BoardKV,
  emptyBoard,
  greenPercent,
  hasRun,
  loadBoard,
  parseBoard,
} from './board.ts';

const EMPTY_EXPECTED = {
  schema: 1,
  generated_at: null,
  game_versions: ['1.21.1', '1.21.11'],
  totals: { mods: 0, green: 0, yellow: 0, red: 0 },
  rows: [],
};

function kvWith(value: string | null): BoardKV & { keys: string[] } {
  const keys: string[] = [];
  return {
    keys,
    async get(key: string) {
      keys.push(key);
      return value;
    },
  };
}

function validBoard(): BoardJson {
  return {
    schema: 1,
    generated_at: '2026-11-04T02:41:00Z',
    game_versions: ['1.21.1', '1.21.11'],
    totals: { mods: 2, green: 1, yellow: 0, red: 1 },
    rows: [
      {
        mod_id: 'AANobbMI',
        name: 'Example mod',
        page_url: 'https://modrinth.com/mod/example',
        downloads: 1234,
        tier: 'a',
        result: 'green',
        reason: null,
        last_run: '2026-11-04T02:40:00Z',
        report_url: null,
      },
      {
        mod_id: 'P7dR8mSH',
        name: 'Another mod',
        page_url: 'https://modrinth.com/mod/another',
        downloads: 99,
        tier: 'b',
        result: 'red',
        reason: 'mixin failure',
        last_run: '2026-11-04T02:40:00Z',
        report_url: 'https://jarrow.pages.dev/board/another/',
      },
    ],
  };
}

describe('emptyBoard', () => {
  it('matches the documented empty structure exactly', () => {
    assert.deepEqual(emptyBoard(), EMPTY_EXPECTED);
  });

  it('returns a fresh object each time', () => {
    const a = emptyBoard();
    a.rows.push(validBoard().rows[0]!);
    assert.equal(emptyBoard().rows.length, 0);
  });
});

describe('loadBoard', () => {
  it('returns the empty board when the binding is absent', async () => {
    for (const kv of [undefined, null, {} as BoardKV]) {
      const load = await loadBoard(kv);
      assert.equal(load.status, 'unbound');
      assert.deepEqual(load.board, EMPTY_EXPECTED);
      assert.equal(hasRun(load), false);
    }
  });

  it('returns the empty board when the key is missing', async () => {
    const kv = kvWith(null);
    const load = await loadBoard(kv);
    assert.equal(load.status, 'missing');
    assert.deepEqual(load.board, EMPTY_EXPECTED);
    assert.deepEqual(kv.keys, [BOARD_KV_KEY]);
  });

  it('returns a valid board and its raw text unchanged', async () => {
    const raw = JSON.stringify(validBoard());
    const load = await loadBoard(kvWith(raw));
    assert.equal(load.status, 'ok');
    if (load.status !== 'ok') return;
    assert.equal(load.raw, raw);
    assert.equal(load.board.rows.length, 2);
    assert.equal(hasRun(load), true);
  });

  it('reports invalid JSON without throwing', async () => {
    const load = await loadBoard(kvWith('{not json'));
    assert.equal(load.status, 'invalid');
    assert.deepEqual(load.board, EMPTY_EXPECTED);
  });

  it('reports a failed KV read without throwing', async () => {
    const kv: BoardKV = {
      async get() {
        throw new Error('network down');
      },
    };
    const load = await loadBoard(kv);
    assert.equal(load.status, 'error');
    if (load.status === 'error') assert.match(load.problem, /network down/);
  });
});

describe('parseBoard', () => {
  it('accepts the empty structure', () => {
    assert.equal(parseBoard(JSON.stringify(EMPTY_EXPECTED)).ok, true);
  });

  it('accepts extra fields for forward compatibility', () => {
    const board = { ...validBoard(), top_reasons: [] };
    assert.equal(parseBoard(JSON.stringify(board)).ok, true);
  });

  const cases: Array<[string, (b: Record<string, any>) => void]> = [
    ['wrong schema', (b) => (b.schema = 2)],
    ['bad generated_at', (b) => (b.generated_at = 'yesterday')],
    ['non-string game version', (b) => (b.game_versions = [1])],
    ['negative total', (b) => (b.totals.red = -1)],
    ['fractional total', (b) => (b.totals.mods = 1.5)],
    ['results exceed mods', (b) => (b.totals.green = 5)],
    ['rows not an array', (b) => (b.rows = {})],
    ['unknown tier', (b) => (b.rows[0].tier = 'A')],
    ['unknown result', (b) => (b.rows[0].result = 'blue')],
    ['page_url not https', (b) => (b.rows[0].page_url = 'http://modrinth.com/mod/x')],
    ['page_url not a URL', (b) => (b.rows[0].page_url = 'modrinth')],
    ['report_url javascript', (b) => (b.rows[1].report_url = 'javascript:alert(1)')],
    ['missing name', (b) => delete b.rows[0].name],
    ['bad last_run', (b) => (b.rows[0].last_run = 'never')],
  ];
  for (const [name, mutate] of cases) {
    it(`rejects ${name}`, () => {
      const board = validBoard() as unknown as Record<string, any>;
      mutate(board);
      const result = parseBoard(JSON.stringify(board));
      assert.equal(result.ok, false, name);
    });
  }

  it('rejects non-objects', () => {
    for (const text of ['null', '[]', '"board"', '1']) {
      assert.equal(parseBoard(text).ok, false, text);
    }
  });
});

describe('greenPercent', () => {
  it('is null without mods and never rounds up', () => {
    assert.equal(greenPercent({ mods: 0, green: 0, yellow: 0, red: 0 }), null);
    assert.equal(greenPercent({ mods: 1000, green: 599, yellow: 0, red: 0 }), 59);
    assert.equal(greenPercent({ mods: 1000, green: 600, yellow: 0, red: 0 }), 60);
  });
});
