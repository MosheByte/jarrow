// Run with: npm test (node --test, using Node's built-in TypeScript type stripping).
import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import {
  BOARD_KV_KEY,
  type BoardJson,
  type BoardKV,
  DIRECTIONS,
  LOADERS,
  columns,
  directionLabel,
  emptyBoard,
  greenPercent,
  hasRun,
  loadBoard,
  parseBoard,
} from './board.ts';

const ZERO = { mods: 0, green: 0, yellow: 0, red: 0 };

/** The documented empty structure (docs/board.md), written out in full. */
const EMPTY_EXPECTED = {
  schema: 2,
  generated_at: null,
  run_id: null,
  engine: null,
  game_versions: ['1.20.1', '1.21.1', '1.21.11'],
  totals: ZERO,
  directions: [
    ['fabric-neoforge', 'fabric', 'neoforge', 1, 60],
    ['neoforge-fabric', 'neoforge', 'fabric', 2, 50],
    ['fabric-quilt', 'fabric', 'quilt', 3, 50],
    ['quilt-fabric', 'quilt', 'fabric', 3, 50],
    ['forge-neoforge', 'forge', 'neoforge', 4, 50],
    ['neoforge-forge', 'neoforge', 'forge', 4, 50],
    ['fabric-forge', 'fabric', 'forge', 5, 50],
    ['forge-fabric', 'forge', 'fabric', 5, 50],
    ['quilt-forge', 'quilt', 'forge', 6, 50],
    ['forge-quilt', 'forge', 'quilt', 6, 50],
    ['quilt-neoforge', 'quilt', 'neoforge', 6, 50],
    ['neoforge-quilt', 'neoforge', 'quilt', 6, 50],
  ].map(([id, from, to, order, bar_percent]) => ({
    id,
    from,
    to,
    order,
    bar_percent,
    column: false,
    rules_version: null,
    totals: ZERO,
    top_reasons: [],
  })),
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

function result(overrides: Record<string, unknown> = {}) {
  return {
    direction: 'fabric-neoforge',
    game_version: '1.21.11',
    version_id: 'abc',
    file_sha512: 'f'.repeat(128),
    mod_id: 'example',
    mod_version: '1.0.0',
    tier: 'a',
    tier_reasons: [],
    outcome: 'green',
    score: 1,
    reason_kind: null,
    reason: null,
    report_key: 'reports/nightly-20261104/fabric-neoforge/AANobbMI/1.21.11.json',
    report_hash: 'sha256:00',
    duration_ms: 412,
    ...overrides,
  };
}

/** A board with one column (fabric-neoforge) holding two rows: one green, one red. */
function validBoard(): BoardJson {
  const board = emptyBoard();
  board.generated_at = '2026-11-04T02:41:00Z';
  board.run_id = 'nightly-20261104';
  board.engine = { version: '0.1.0', git_sha: '0123abcd' };
  board.totals = { mods: 2, green: 1, yellow: 0, red: 1 };
  const column = board.directions[0]!;
  column.column = true;
  column.rules_version = 'sha256:rules';
  column.totals = { mods: 2, green: 1, yellow: 0, red: 1 };
  column.top_reasons = [{ kind: 'mixin_failure', count: 1 }];
  board.rows = [
    {
      direction: 'fabric-neoforge',
      project_id: 'AANobbMI',
      slug: 'example',
      title: 'Example mod',
      downloads: 1234,
      page_url: 'https://modrinth.com/mod/example',
      icon_url: null,
      license: 'MIT',
      primary_game_version: '1.21.11',
      tier: 'a',
      outcome: 'green',
      reason_kind: null,
      reason: null,
      last_run: '2026-11-04T02:40:00Z',
      report_url: null,
      target_build_exists: false,
      results: [result(), result({ game_version: '1.21.1', outcome: 'not-run', score: null, reason_kind: 'harness_not_run_eula' })],
    },
    {
      direction: 'fabric-neoforge',
      project_id: 'P7dR8mSH',
      slug: 'another',
      title: 'Another mod',
      downloads: 99,
      page_url: 'https://modrinth.com/mod/another',
      icon_url: 'https://cdn.modrinth.com/data/P7dR8mSH/icon.png',
      license: null,
      primary_game_version: '1.21.1',
      tier: 'b',
      outcome: 'red',
      reason_kind: 'mixin_failure',
      reason: 'mixin failure',
      last_run: '2026-11-04T02:40:00Z',
      report_url: 'https://jarrow.pages.dev/board/another/',
      target_build_exists: true,
      results: [result({ game_version: '1.21.1', tier: 'b', outcome: 'red', score: 0, reason_kind: 'mixin_failure' })],
    },
  ] as BoardJson['rows'];
  return board;
}

describe('DIRECTIONS', () => {
  it('lists every ordered pair of the four loaders exactly once', () => {
    const ids = DIRECTIONS.map((d) => d.id);
    assert.equal(new Set(ids).size, 12);
    for (const from of LOADERS) {
      for (const to of LOADERS) {
        if (from !== to) assert.ok(ids.includes(`${from}-${to}` as (typeof ids)[number]), `${from}-${to}`);
      }
    }
    for (const d of DIRECTIONS) assert.equal(d.id, `${d.from}-${d.to}`);
  });

  it('keeps the fixed build order and the v1 bars', () => {
    assert.deepEqual(
      DIRECTIONS.map((d) => d.order),
      [1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6],
    );
    for (const d of DIRECTIONS) assert.equal(d.bar_percent, d.id === 'fabric-neoforge' ? 60 : 50, d.id);
  });

  it('builds each pair and its inverse in the same step, except the first pair', () => {
    for (const d of DIRECTIONS) {
      const inverse = DIRECTIONS.find((x) => x.from === d.to && x.to === d.from)!;
      if (d.from === 'fabric' && d.to === 'neoforge') assert.equal(inverse.order, 2);
      else if (d.from === 'neoforge' && d.to === 'fabric') assert.equal(inverse.order, 1);
      else assert.equal(inverse.order, d.order, d.id);
    }
  });

  it('labels directions with loader names', () => {
    assert.equal(directionLabel('fabric-neoforge'), 'Fabric → NeoForge');
    assert.equal(directionLabel('quilt-forge'), 'Quilt → Forge');
  });
});

describe('emptyBoard', () => {
  it('matches the documented empty structure exactly', () => {
    assert.deepEqual(emptyBoard(), EMPTY_EXPECTED);
  });

  it('reports zero, and no column, for every direction', () => {
    const board = emptyBoard();
    assert.equal(board.directions.length, 12);
    for (const d of board.directions) {
      assert.equal(d.column, false, d.id);
      assert.deepEqual(d.totals, ZERO, d.id);
    }
    assert.deepEqual(board.totals, ZERO);
    assert.deepEqual(columns(board), []);
  });

  it('is itself a valid board', () => {
    assert.equal(parseBoard(JSON.stringify(emptyBoard())).ok, true);
  });

  it('returns a fresh object each time', () => {
    const a = emptyBoard();
    a.rows.push(validBoard().rows[0]!);
    a.directions[0]!.totals.mods = 5;
    assert.equal(emptyBoard().rows.length, 0);
    assert.equal(emptyBoard().directions[0]!.totals.mods, 0);
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
    assert.equal(load.status, 'ok', load.status === 'invalid' ? load.problem : '');
    if (load.status !== 'ok') return;
    assert.equal(load.raw, raw);
    assert.equal(load.board.rows.length, 2);
    assert.deepEqual(
      columns(load.board).map((d) => d.id),
      ['fabric-neoforge'],
    );
    assert.equal(hasRun(load), true);
  });

  it('reports invalid JSON without throwing', async () => {
    const load = await loadBoard(kvWith('{not json'));
    assert.equal(load.status, 'invalid');
    assert.deepEqual(load.board, EMPTY_EXPECTED);
  });

  it('reports a board in the old schema as invalid', async () => {
    const old = { schema: 1, generated_at: null, game_versions: ['1.21.1', '1.21.11'], totals: ZERO, rows: [] };
    const load = await loadBoard(kvWith(JSON.stringify(old)));
    assert.equal(load.status, 'invalid');
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

  it('accepts the two-row example', () => {
    const parsed = parseBoard(JSON.stringify(validBoard()));
    assert.equal(parsed.ok, true, parsed.ok ? '' : parsed.problem);
  });

  it('accepts extra fields for forward compatibility', () => {
    const board = { ...validBoard(), note: 'extra' };
    assert.equal(parseBoard(JSON.stringify(board)).ok, true);
  });

  it('accepts a red row whose file could not be classified', () => {
    const board = validBoard() as unknown as Record<string, any>;
    board.rows[1].tier = null;
    board.rows[1].reason_kind = 'download_failed';
    board.rows[1].results[0].tier = null;
    board.rows[1].results[0].outcome = 'not-run';
    board.rows[1].results[0].score = null;
    board.directions[0].top_reasons = [{ kind: 'download_failed', count: 1 }];
    const parsed = parseBoard(JSON.stringify(board));
    assert.equal(parsed.ok, true, parsed.ok ? '' : parsed.problem);
  });

  const cases: Array<[string, (b: Record<string, any>) => void]> = [
    ['wrong schema', (b) => (b.schema = 1)],
    ['bad generated_at', (b) => (b.generated_at = 'yesterday')],
    ['empty run_id', (b) => (b.run_id = '')],
    ['engine without git_sha', (b) => (b.engine = { version: '0.1.0' })],
    ['non-string game version', (b) => (b.game_versions = [1])],
    ['repeated game version', (b) => (b.game_versions = ['1.21.1', '1.21.1'])],
    ['negative total', (b) => (b.totals.red = -1)],
    ['fractional total', (b) => (b.totals.mods = 1.5)],
    ['totals that do not add up', (b) => (b.totals.mods = 3)],
    ['totals that differ from the directions', (b) => (b.totals = { mods: 3, green: 1, yellow: 0, red: 2 })],
    ['a missing direction', (b) => b.directions.pop()],
    ['directions out of build order', (b) => b.directions.reverse()],
    ['a changed bar', (b) => (b.directions[0].bar_percent = 50)],
    ['a direction with the wrong loaders', (b) => (b.directions[1].from = 'forge')],
    ['column that is not a boolean', (b) => (b.directions[0].column = 'yes')],
    ['direction totals that do not add up', (b) => (b.directions[0].totals.green = 2)],
    ['an unknown top reason', (b) => (b.directions[0].top_reasons = [{ kind: 'bad luck', count: 1 }])],
    [
      'numbers in a direction without a column',
      (b) => {
        b.directions[1].totals = { mods: 1, green: 0, yellow: 0, red: 1 };
        b.totals = { mods: 3, green: 1, yellow: 0, red: 2 };
      },
    ],
    ['rows in a direction without a column', (b) => {
      b.rows[1].direction = 'neoforge-fabric';
      b.rows[1].results[0].direction = 'neoforge-fabric';
    }],
    ['rows that do not match the totals', (b) => b.rows.pop()],
    ['a repeated row', (b) => (b.rows[1] = { ...b.rows[0] })],
    ['rows not an array', (b) => (b.rows = {})],
    ['an unknown direction', (b) => (b.rows[0].direction = 'fabric-bukkit')],
    ['an unknown tier', (b) => (b.rows[0].tier = 'A')],
    ['an unknown outcome', (b) => (b.rows[0].outcome = 'blue')],
    ['a board row that was not run', (b) => (b.rows[1].outcome = 'not-run')],
    ['green without a harness score', (b) => (b.rows[0].results[0].score = null)],
    ['green at score 0.5', (b) => (b.rows[0].results[0].score = 0.5)],
    ['a green row over a red result', (b) => {
      b.rows[0].results[0].outcome = 'red';
      b.rows[0].results[0].score = 0;
    }],
    ['red without a reason', (b) => (b.rows[1].reason_kind = null)],
    ['an unknown reason kind', (b) => (b.rows[1].reason_kind = 'bad luck')],
    ['a score off the scale', (b) => (b.rows[1].results[0].score = 0.3)],
    ['a result that was not run but has a score', (b) => (b.rows[0].results[1].score = 0)],
    ['a result in another direction', (b) => (b.rows[0].results[1].direction = 'fabric-forge')],
    ['a game version that is not pinned', (b) => (b.rows[0].results[1].game_version = '1.19.2')],
    ['a game version twice', (b) => (b.rows[0].results[1].game_version = '1.21.11')],
    ['no result for the primary version', (b) => (b.rows[1].primary_game_version = '1.21.11')],
    ['no results', (b) => (b.rows[1].results = [])],
    ['page_url not https', (b) => (b.rows[0].page_url = 'http://modrinth.com/mod/x')],
    ['page_url not a URL', (b) => (b.rows[0].page_url = 'modrinth')],
    ['report_url javascript', (b) => (b.rows[1].report_url = 'javascript:alert(1)')],
    ['icon_url javascript', (b) => (b.rows[1].icon_url = 'javascript:alert(1)')],
    ['missing title', (b) => delete b.rows[0].title],
    ['bad last_run', (b) => (b.rows[0].last_run = 'never')],
    ['target_build_exists missing', (b) => delete b.rows[0].target_build_exists],
  ];
  for (const [name, mutate] of cases) {
    it(`rejects ${name}`, () => {
      const board = validBoard() as unknown as Record<string, any>;
      mutate(board);
      const parsed = parseBoard(JSON.stringify(board));
      assert.equal(parsed.ok, false, name);
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
    assert.equal(greenPercent({ mods: 1000, green: 599, yellow: 0, red: 401 }), 59);
    assert.equal(greenPercent({ mods: 1000, green: 600, yellow: 0, red: 400 }), 60);
  });
});
