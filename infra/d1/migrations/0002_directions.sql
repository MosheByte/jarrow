-- Jarrow D1 schema, migration 0002 (M0, D-029): one board column per direction.
-- The board is any loader to any loader, so every board row now names its direction, and
-- the tier and outcome strings follow D-024 (crates/report): tiers 'a', 'b', 'unsupported';
-- outcomes 'green', 'yellow', 'red', 'not-run'. The contract is docs/board.md.
--
-- No nightly has run, so board_runs and board_rows hold no rows. They are dropped and
-- recreated instead of altered. author_flags, errors and events are left as they are.

DROP TABLE IF EXISTS board_rows;            -- first: it references board_runs
DROP TABLE IF EXISTS board_runs;

CREATE TABLE board_runs (
  id             TEXT PRIMARY KEY,          -- e.g. "nightly-20261104"
  started_at     TEXT NOT NULL,             -- RFC 3339 UTC
  finished_at    TEXT,
  engine_version TEXT NOT NULL,
  game_versions  TEXT NOT NULL,             -- JSON array of pinned versions
  directions     TEXT NOT NULL,             -- JSON array of the direction ids with a column in this run
  totals         TEXT NOT NULL              -- JSON object per direction id:
                                            --   {"fabric-neoforge": {"mods":n,"green":n,"yellow":n,"red":n,
                                            --    "rules_version":"sha256:..."}, ...}
);

-- One row per mod, direction and game version: the "results" entries of board.json.
CREATE TABLE board_rows (
  run_id        TEXT NOT NULL REFERENCES board_runs(id) ON DELETE CASCADE,
  direction     TEXT NOT NULL CHECK (direction IN (
                  'fabric-neoforge', 'neoforge-fabric',
                  'fabric-quilt', 'quilt-fabric',
                  'forge-neoforge', 'neoforge-forge',
                  'fabric-forge', 'forge-fabric',
                  'quilt-forge', 'forge-quilt', 'quilt-neoforge', 'neoforge-quilt')),
  project_id    TEXT NOT NULL,              -- Modrinth project id
  slug          TEXT NOT NULL,
  game_version  TEXT NOT NULL,
  downloads     INTEGER NOT NULL CHECK (downloads >= 0),
  version_id    TEXT,                       -- Modrinth version id of the source file
  file_sha512   TEXT,
  mod_id        TEXT,                       -- from the source loader's manifest, when readable
  mod_version   TEXT,
  tier          TEXT CHECK (tier IS NULL OR tier IN ('a', 'b', 'unsupported')),
                                            -- NULL only when the classifier could not run
  outcome       TEXT NOT NULL CHECK (outcome IN ('green', 'yellow', 'red', 'not-run')),
  score         REAL CHECK (score IS NULL OR score IN (0, 0.25, 0.5, 0.75, 1)),
                                            -- harness score, NULL when not run
  reason_kind   TEXT,                       -- classified reason (docs/board.md vocabulary)
  reason        TEXT,
  report_key    TEXT,                       -- R2 key of the full report
  report_hash   TEXT,
  duration_ms   INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
  is_primary    INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
                                            -- 1 for the mod's primary game version in this column
  board_outcome TEXT CHECK (board_outcome IS NULL OR board_outcome IN ('green', 'yellow', 'red')),
                                            -- the row's outcome on the board; set on the primary entry only
  PRIMARY KEY (run_id, direction, project_id, game_version),
  CHECK ((is_primary = 1) = (board_outcome IS NOT NULL)),
  -- Nothing is green without a harness score of 1.0 or 0.75 (D-024). NULL must not slip
  -- through, so the score is tested with IS NOT NULL first.
  CHECK (outcome <> 'green' OR (score IS NOT NULL AND score IN (0.75, 1))),
  CHECK (board_outcome IS NOT 'green' OR (outcome = 'green' AND score IS NOT NULL AND score IN (0.75, 1)))
);
CREATE INDEX board_rows_slug ON board_rows (slug);
CREATE INDEX board_rows_outcome ON board_rows (run_id, direction, board_outcome);
CREATE UNIQUE INDEX board_rows_primary ON board_rows (run_id, direction, project_id) WHERE is_primary = 1;
