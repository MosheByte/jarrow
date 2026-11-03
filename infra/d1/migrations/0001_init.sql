-- Jarrow D1 schema, migration 0001 (M0).
-- Board results, author flags, and the error/event fallbacks for Sentry and PostHog.
-- Queue and user tables arrive with Tier B in M4.

CREATE TABLE board_runs (
  id             TEXT PRIMARY KEY,          -- e.g. "nightly-20261104"
  started_at     TEXT NOT NULL,             -- RFC 3339 UTC
  finished_at    TEXT,
  engine_version TEXT NOT NULL,
  rules_version  TEXT NOT NULL,
  game_versions  TEXT NOT NULL,             -- JSON array of pinned versions
  totals         TEXT NOT NULL              -- JSON {"mods":n,"green":n,"yellow":n,"red":n}
);

CREATE TABLE board_rows (
  run_id        TEXT NOT NULL REFERENCES board_runs(id) ON DELETE CASCADE,
  project_id    TEXT NOT NULL,              -- Modrinth project id
  slug          TEXT NOT NULL,
  mod_id        TEXT,                       -- from fabric.mod.json, when readable
  game_version  TEXT NOT NULL,
  downloads     INTEGER NOT NULL,
  tier          TEXT NOT NULL CHECK (tier IN ('A', 'B', 'unsupported', 'unknown')),
  outcome       TEXT NOT NULL CHECK (outcome IN ('green', 'yellow', 'red', 'not_run')),
  score         REAL,                       -- harness score 0..1, NULL when not run
  reason_kind   TEXT,                       -- classified red/yellow reason
  reason        TEXT,
  report_key    TEXT,                       -- R2 key of the full report
  report_hash   TEXT,
  PRIMARY KEY (run_id, project_id, game_version)
);
CREATE INDEX board_rows_slug ON board_rows (slug);
CREATE INDEX board_rows_outcome ON board_rows (run_id, outcome);

-- Verified author choices: honoured by the converter and the board (docs/legal.md).
CREATE TABLE author_flags (
  project_id        TEXT PRIMARY KEY,
  flag              TEXT NOT NULL CHECK (flag IN ('do_not_port', 'official_port')),
  official_port_url TEXT,
  verified_by       TEXT NOT NULL,          -- GitHub login or Modrinth user id
  verified_via      TEXT NOT NULL CHECK (verified_via IN ('github', 'modrinth', 'issue')),
  verified_at       TEXT NOT NULL
);

-- Fallback error sink while Sentry is not wired (D-009).
CREATE TABLE errors (
  id      INTEGER PRIMARY KEY AUTOINCREMENT,
  at      TEXT NOT NULL,
  source  TEXT NOT NULL,                    -- "site", "worker", "tierb", "board"
  message TEXT NOT NULL,
  context TEXT                              -- JSON, scrubbed of user class names
);

-- Fallback event sink while PostHog is not wired. Anonymous: no IPs, no user ids.
CREATE TABLE events (
  id    INTEGER PRIMARY KEY AUTOINCREMENT,
  at    TEXT NOT NULL,
  name  TEXT NOT NULL,
  props TEXT NOT NULL                       -- JSON, fields listed in docs/telemetry.md
);
