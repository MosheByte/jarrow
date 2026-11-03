// Run with: npm test (node --test, using Node's built-in TypeScript type stripping).
import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import {
  BOARD_KV_KEY,
  type BoardKV,
  emptyBoard,
  greenPercent,
  hasRun,
  isDirection,
  loadBoard,
  parseBoard,
  TIER_LABEL,
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

/** A row as `jarrow-board merge` writes it (docs/board.md), trimmed to one result. */
function storedRow(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    direction: 'fabric-neoforge',
    project_id: 'AANobbMI',
    slug: 'example',
    title: 'Example mod',
    downloads: 1234,
    page_url: 'https://modrinth.com/mod/example',
    icon_url: null,
    license: 'MIT',
    primary_game_version: '1.21.11',
    tier: 'unknown',
    outcome: 'red',
    reason_kind: 'no_converter',
    reason: 'No converter yet (milestone M2)',
    last_run: '2026-11-04T02:40:00Z',
    neoforge_build_exists: false,
    target_build_exists: false,
    results: [
      {
        game_version: '1.21.11',
        version_id: 'abcdEFGH',
        version_number: '1.0.0',
        file_sha512: 'a'.repeat(128),
        mod_id: null,
        mod_version: '1.0.0',
        tier: 'unknown',
        tier_reasons: ['classifier not built yet (M1 wave 2)'],
        outcome: 'red',
        score: null,
        reason_kind: 'no_converter',
        reason: 'No converter yet (milestone M2)',
        report_key: 'report:fabric-neoforge:AANobbMI:1.21.11',
        report_hash: `sha256:${'b'.repeat(64)}`,
        duration_ms: 412,
      },
    ],
    ...overrides,
  };
}

function validBoard(): Record<string, any> {
  return {
    schema: 1,
    generated_at: '2026-11-04T02:41:00Z',
    started_at: '2026-11-04T02:20:00Z',
    run_id: 'nightly-20261104',
    engine: { version: '0.1.0', git_sha: '0123456789abcdef0123456789abcdef01234567' },
    rules_version: 'none',
    game_versions: ['1.21.1', '1.21.11'],
    totals: { mods: 2, green: 1, yellow: 0, red: 1 },
    top_reasons: [{ kind: 'no_converter', label: 'No converter yet (milestone M2)', count: 1 }],
    directions: [],
    stats: {},
    rows: [
      storedRow({ tier: 'a', outcome: 'green', reason_kind: null, reason: null }),
      storedRow({ project_id: 'P7dR8mSH', title: 'Another mod', downloads: 99, page_url: 'https://modrinth.com/mod/another' }),
    ],
  };
}

describe('emptyBoard', () => {
  it('matches the documented empty structure exactly', () => {
    assert.deepEqual(emptyBoard(), EMPTY_EXPECTED);
  });

  it('returns a fresh object each time', () => {
    const a = emptyBoard();
    a.game_versions.push('1.0');
    assert.deepEqual(emptyBoard().game_versions, ['1.21.1', '1.21.11']);
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

  it('returns a valid board, mapped for the pages, and its raw text unchanged', async () => {
    const raw = JSON.stringify(validBoard());
    const load = await loadBoard(kvWith(raw));
    assert.equal(load.status, 'ok');
    if (load.status !== 'ok') return;
    assert.equal(load.raw, raw);
    assert.equal(hasRun(load), true);
    assert.deepEqual(load.board.rows[1], {
      mod_id: 'P7dR8mSH',
      name: 'Another mod',
      direction: 'fabric-neoforge',
      page_url: 'https://modrinth.com/mod/another',
      downloads: 99,
      tier: 'unknown',
      result: 'red',
      reason: 'No converter yet (milestone M2)',
      last_run: '2026-11-04T02:40:00Z',
      report_url: null,
    });
    assert.equal(load.board.rows[0]?.result, 'green');
    assert.deepEqual(load.board.totals, { mods: 2, green: 1, yellow: 0, red: 1 });
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
    const board: Record<string, any> = { ...validBoard(), something_new: [] };
    board.rows[0].also_new = true;
    assert.equal(parseBoard(JSON.stringify(board)).ok, true);
  });

  it('accepts every tier, including unknown while the classifier is not built', () => {
    for (const tier of ['a', 'b', 'unsupported', 'unknown']) {
      const board = validBoard();
      board.rows[1].tier = tier;
      assert.equal(parseBoard(JSON.stringify(board)).ok, true, tier);
    }
    assert.equal(TIER_LABEL.unknown, 'Not classified yet');
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
    ['unknown outcome', (b) => (b.rows[0].outcome = 'blue')],
    ['the view name for the outcome', (b) => (delete b.rows[0].outcome, (b.rows[0].result = 'red'))],
    ['page_url not https', (b) => (b.rows[0].page_url = 'http://modrinth.com/mod/x')],
    ['page_url not a URL', (b) => (b.rows[0].page_url = 'modrinth')],
    ['missing title', (b) => delete b.rows[0].title],
    ['missing project_id', (b) => delete b.rows[0].project_id],
    ['missing direction', (b) => delete b.rows[0].direction],
    ['same-loader direction', (b) => (b.rows[0].direction = 'fabric-fabric')],
    ['bad last_run', (b) => (b.rows[0].last_run = 'never')],
  ];
  for (const [name, mutate] of cases) {
    it(`rejects ${name}`, () => {
      const board = validBoard();
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

describe('isDirection', () => {
  it('accepts two different known loaders', () => {
    assert.equal(isDirection('fabric-neoforge'), true);
    assert.equal(isDirection('quilt-forge'), true);
    for (const bad of ['fabric', 'fabric-fabric', 'fabric-rift', 'fabric-neoforge-forge', 1, null]) {
      assert.equal(isDirection(bad), false, String(bad));
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
