/**
 * The public board: the shape of `board.json` and the one loader every page and the
 * `/api/board.json` route use to read it. The contract is docs/board.md; change both together.
 *
 * `board.json` lives in the KV namespace bound as `BOARD` under the key `board.json`. The
 * nightly board pipeline (crates/board, milestone M1) writes it. Until the first nightly runs,
 * the key is missing and the site shows an empty board: every direction listed, none with a
 * column, zero everywhere, no rows. Nothing here invents a row or a number.
 *
 * The board has one column per direction (D-029): 12 directions between Fabric, NeoForge,
 * Forge and Quilt, listed in build order in DIRECTIONS. A row is one mod in one direction
 * column, and its `results` hold one entry per game version.
 *
 * Tier and outcome ids match `jarrow_report::Tier` and `jarrow_report::Outcome`
 * (crates/report/src/outcome.rs, D-024): tiers are "a", "b", "unsupported"; outcomes are
 * "green", "yellow", "red", "not-run". A board row's outcome is never "not-run", and nothing
 * is green without a harness score of 1.0 or 0.75.
 *
 * This module has no runtime dependency on Astro or Cloudflare, so its tests run with
 * `node --test` (see board.test.ts).
 */

import { GAME_VERSIONS } from './site.ts';

/** KV key the nightly writes and the site reads. */
export const BOARD_KV_KEY = 'board.json';

/** Version of the board.json schema this site understands (docs/board.md). */
export const BOARD_SCHEMA = 2;

/** Loader ids, as in loaders/<id>.toml. */
export const LOADERS = ['fabric', 'neoforge', 'forge', 'quilt'] as const;
export type Loader = (typeof LOADERS)[number];

export const LOADER_LABEL: Record<Loader, string> = {
  fabric: 'Fabric',
  neoforge: 'NeoForge',
  forge: 'Forge',
  quilt: 'Quilt',
};

/**
 * The 12 directions in the fixed build order (docs/directions/README.md). `order` is the build
 * step; `bar_percent` is the share of the direction's column that must be green for v1.
 */
export const DIRECTIONS = [
  { id: 'fabric-neoforge', from: 'fabric', to: 'neoforge', order: 1, bar_percent: 60 },
  { id: 'neoforge-fabric', from: 'neoforge', to: 'fabric', order: 2, bar_percent: 50 },
  { id: 'fabric-quilt', from: 'fabric', to: 'quilt', order: 3, bar_percent: 50 },
  { id: 'quilt-fabric', from: 'quilt', to: 'fabric', order: 3, bar_percent: 50 },
  { id: 'forge-neoforge', from: 'forge', to: 'neoforge', order: 4, bar_percent: 50 },
  { id: 'neoforge-forge', from: 'neoforge', to: 'forge', order: 4, bar_percent: 50 },
  { id: 'fabric-forge', from: 'fabric', to: 'forge', order: 5, bar_percent: 50 },
  { id: 'forge-fabric', from: 'forge', to: 'fabric', order: 5, bar_percent: 50 },
  { id: 'quilt-forge', from: 'quilt', to: 'forge', order: 6, bar_percent: 50 },
  { id: 'forge-quilt', from: 'forge', to: 'quilt', order: 6, bar_percent: 50 },
  { id: 'quilt-neoforge', from: 'quilt', to: 'neoforge', order: 6, bar_percent: 50 },
  { id: 'neoforge-quilt', from: 'neoforge', to: 'quilt', order: 6, bar_percent: 50 },
] as const;
export type DirectionId = (typeof DIRECTIONS)[number]['id'];

const DIRECTION_IDS: readonly string[] = DIRECTIONS.map((d) => d.id);

/** "Fabric → NeoForge" for "fabric-neoforge". */
export function directionLabel(id: DirectionId): string {
  const spec = DIRECTIONS.find((d) => d.id === id)!;
  return `${LOADER_LABEL[spec.from]} → ${LOADER_LABEL[spec.to]}`;
}

export const TIERS = ['a', 'b', 'unsupported'] as const;
export type Tier = (typeof TIERS)[number];

/** Every outcome a result can have. */
export const OUTCOMES = ['green', 'yellow', 'red', 'not-run'] as const;
export type Outcome = (typeof OUTCOMES)[number];

/** Outcomes a board row can have: a mod that was not measured counts as red or yellow (D-024). */
export const BOARD_OUTCOMES = ['green', 'yellow', 'red'] as const;
export type BoardOutcome = (typeof BOARD_OUTCOMES)[number];

/** Harness scores (docs/harness.md). Only 1.0 and 0.75 are green. */
export const SCORES = [0, 0.25, 0.5, 0.75, 1] as const;
export type Score = (typeof SCORES)[number];
const GREEN_SCORES: readonly number[] = [0.75, 1];

/** The closed reason vocabulary (docs/board.md). */
export const REASON_KINDS = [
  'no_converter',
  'unsupported_native',
  'unsupported_transformer',
  'unsupported_language_adapter',
  'unsupported_obfuscated',
  'license_personal_use',
  'tier_b_required',
  'harness_not_run_eula',
  'harness_load_failure',
  'class_not_found',
  'mixin_failure',
  'registry_conflict',
  'crash',
  'entries_differ',
  'client_only_not_measured',
  'download_failed',
  'opt_out',
] as const;
export type ReasonKind = (typeof REASON_KINDS)[number];

export interface BoardTotals {
  mods: number;
  green: number;
  yellow: number;
  red: number;
}

export interface ReasonCount {
  kind: ReasonKind;
  count: number;
}

/** One direction and its board column. */
export interface BoardDirection {
  id: DirectionId;
  from: Loader;
  to: Loader;
  order: number;
  bar_percent: number;
  /** True once the nightly measured this direction. */
  column: boolean;
  /** Content hash of rules/<from>-<to>/ used in the run; null without a column. */
  rules_version: string | null;
  totals: BoardTotals;
  top_reasons: ReasonCount[];
}

/** One mod, one direction, one game version. */
export interface BoardResult {
  direction: DirectionId;
  game_version: string;
  version_id: string | null;
  file_sha512: string | null;
  /** The mod id from the source loader's manifest, when readable. */
  mod_id: string | null;
  mod_version: string | null;
  tier: Tier | null;
  tier_reasons: string[];
  outcome: Outcome;
  score: Score | null;
  reason_kind: ReasonKind | null;
  reason: string | null;
  report_key: string | null;
  report_hash: string | null;
  duration_ms: number | null;
}

/** One mod in one direction column. */
export interface BoardRow {
  direction: DirectionId;
  /** Modrinth project id. */
  project_id: string;
  slug: string;
  /** Display name, as Modrinth lists it. */
  title: string;
  /** Modrinth download count when the source list was fetched. */
  downloads: number;
  /** The mod's own Modrinth page. The board links here and never to a converted JAR. */
  page_url: string;
  icon_url: string | null;
  license: string | null;
  /** The newest pinned version the mod ships a source-loader file for. */
  primary_game_version: string;
  /** Null only when the classifier could not run (for example download_failed). */
  tier: Tier | null;
  outcome: BoardOutcome;
  /** Classified reason; always present for yellow and red. */
  reason_kind: ReasonKind | null;
  reason: string | null;
  /** ISO 8601 time of the last harness run for this mod in this column. */
  last_run: string | null;
  /** Link to the full report, when one exists. */
  report_url: string | null;
  /** Informational: whether Modrinth lists the target loader's tag for the project. */
  target_build_exists: boolean;
  results: BoardResult[];
}

/** The whole board as stored in KV and served at /api/board.json. */
export interface BoardJson {
  schema: typeof BOARD_SCHEMA;
  /** ISO 8601 time the nightly wrote this file; null before the first nightly. */
  generated_at: string | null;
  run_id: string | null;
  engine: { version: string; git_sha: string } | null;
  game_versions: string[];
  /** The sum of every direction's totals: one entry per mod per column. */
  totals: BoardTotals;
  /** All 12 directions, in build order. */
  directions: BoardDirection[];
  rows: BoardRow[];
}

/** The minimal part of a Workers KV namespace this module needs. */
export interface BoardKV {
  get(key: string, type: 'text'): Promise<string | null>;
}

function zeroTotals(): BoardTotals {
  return { mods: 0, green: 0, yellow: 0, red: 0 };
}

/** The board before the first nightly: every direction listed, no column, zero everywhere, no rows. */
export function emptyBoard(): BoardJson {
  return {
    schema: BOARD_SCHEMA,
    generated_at: null,
    run_id: null,
    engine: null,
    game_versions: [...GAME_VERSIONS],
    totals: zeroTotals(),
    directions: DIRECTIONS.map((d) => ({
      id: d.id,
      from: d.from,
      to: d.to,
      order: d.order,
      bar_percent: d.bar_percent,
      column: false,
      rules_version: null,
      totals: zeroTotals(),
      top_reasons: [],
    })),
    rows: [],
  };
}

/**
 * What the loader found.
 *
 * - `ok`: a valid board.json was read from KV; `raw` is the stored text, served unchanged.
 * - `missing`: the BOARD binding exists but the key is absent (no nightly has run yet).
 * - `unbound`: there is no BOARD binding (local development without KV); empty board.
 * - `invalid`: the stored value is not a board.json this site understands.
 * - `error`: reading KV failed.
 *
 * In every case `board` is safe to render; for anything but `ok` it is the empty board.
 */
export type BoardLoad =
  | { status: 'ok'; board: BoardJson; raw: string }
  | { status: 'missing' | 'unbound'; board: BoardJson }
  | { status: 'invalid' | 'error'; board: BoardJson; problem: string };

export type ParseResult = { ok: true; board: BoardJson } | { ok: false; problem: string };

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value !== '';
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string';
}

function isIsoTime(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && !Number.isNaN(Date.parse(value));
}

function isHttpsUrl(value: unknown): value is string {
  if (typeof value !== 'string') return false;
  try {
    return new URL(value).protocol === 'https:';
  } catch {
    return false;
  }
}

function oneOf<T extends string | number>(values: readonly T[], value: unknown): value is T {
  return (values as readonly unknown[]).includes(value);
}

function checkTotals(value: unknown, at: string): string | null {
  if (!isObject(value)) return `${at} is missing`;
  for (const key of ['mods', 'green', 'yellow', 'red'] as const) {
    if (!isCount(value[key])) return `${at}.${key} must be a non-negative integer`;
  }
  const t = value as unknown as BoardTotals;
  if (t.green + t.yellow + t.red !== t.mods) return `${at}: green + yellow + red must equal mods`;
  return null;
}

function checkDirection(value: unknown, index: number): string | null {
  const spec = DIRECTIONS[index]!;
  const at = `directions[${index}]`;
  if (!isObject(value)) return `${at} is not an object`;
  for (const key of ['id', 'from', 'to', 'order', 'bar_percent'] as const) {
    if (value[key] !== spec[key]) {
      return `${at}.${key} must be ${JSON.stringify(spec[key])}: directions are listed in build order`;
    }
  }
  if (typeof value.column !== 'boolean') return `${at}.column must be true or false`;
  if (!isNullableString(value.rules_version)) return `${at}.rules_version must be a string or null`;
  const totals = checkTotals(value.totals, `${at}.totals`);
  if (totals) return totals;
  if (!Array.isArray(value.top_reasons)) return `${at}.top_reasons must be an array`;
  for (const [i, reason] of value.top_reasons.entries()) {
    if (!isObject(reason) || !oneOf(REASON_KINDS, reason.kind) || !isCount(reason.count)) {
      return `${at}.top_reasons[${i}] must be a known reason kind with a count`;
    }
  }
  if (!value.column) {
    const t = value.totals as BoardTotals;
    if (t.mods !== 0 || value.top_reasons.length !== 0 || value.rules_version !== null) {
      return `${at} has no column, so its totals must be zero, with no top reasons and no rules_version`;
    }
  }
  return null;
}

function checkResult(value: unknown, at: string, gameVersions: readonly string[]): string | null {
  if (!isObject(value)) return `${at} is not an object`;
  if (!oneOf(DIRECTION_IDS, value.direction)) return `${at}.direction is not a known direction`;
  if (typeof value.game_version !== 'string' || !gameVersions.includes(value.game_version)) {
    return `${at}.game_version must be one of game_versions`;
  }
  for (const key of ['version_id', 'file_sha512', 'mod_id', 'mod_version', 'reason', 'report_key', 'report_hash'] as const) {
    if (!isNullableString(value[key])) return `${at}.${key} must be a string or null`;
  }
  if (value.tier !== null && !oneOf(TIERS, value.tier)) return `${at}.tier must be one of ${TIERS.join(', ')} or null`;
  if (!Array.isArray(value.tier_reasons) || !value.tier_reasons.every((r) => typeof r === 'string')) {
    return `${at}.tier_reasons must be an array of strings`;
  }
  if (!oneOf(OUTCOMES, value.outcome)) return `${at}.outcome must be one of ${OUTCOMES.join(', ')}`;
  if (value.score !== null && !oneOf(SCORES, value.score)) return `${at}.score must be a harness score or null`;
  if (value.outcome === 'green' && !oneOf(GREEN_SCORES, value.score)) {
    return `${at} is green without a harness score of 1.0 or 0.75`;
  }
  if (value.outcome === 'not-run' && value.score !== null) return `${at} was not run, so its score must be null`;
  if (value.reason_kind !== null && !oneOf(REASON_KINDS, value.reason_kind)) {
    return `${at}.reason_kind is not a known reason kind`;
  }
  if (value.duration_ms !== null && !isCount(value.duration_ms)) {
    return `${at}.duration_ms must be a non-negative integer or null`;
  }
  return null;
}

function checkRow(row: unknown, index: number, gameVersions: readonly string[]): string | null {
  const at = `rows[${index}]`;
  if (!isObject(row)) return `${at} is not an object`;
  if (!oneOf(DIRECTION_IDS, row.direction)) return `${at}.direction is not a known direction`;
  for (const key of ['project_id', 'slug', 'title'] as const) {
    if (!isNonEmptyString(row[key])) return `${at}.${key} must be a non-empty string`;
  }
  if (!isCount(row.downloads)) return `${at}.downloads must be a non-negative integer`;
  if (!isHttpsUrl(row.page_url)) return `${at}.page_url must be an https URL`;
  if (row.icon_url !== null && !isHttpsUrl(row.icon_url)) return `${at}.icon_url must be an https URL or null`;
  if (!isNullableString(row.license)) return `${at}.license must be a string or null`;
  if (typeof row.primary_game_version !== 'string' || !gameVersions.includes(row.primary_game_version)) {
    return `${at}.primary_game_version must be one of game_versions`;
  }
  if (row.tier !== null && !oneOf(TIERS, row.tier)) return `${at}.tier must be one of ${TIERS.join(', ')} or null`;
  if (!oneOf(BOARD_OUTCOMES, row.outcome)) return `${at}.outcome must be one of ${BOARD_OUTCOMES.join(', ')}`;
  if (row.reason_kind !== null && !oneOf(REASON_KINDS, row.reason_kind)) {
    return `${at}.reason_kind is not a known reason kind`;
  }
  if (row.outcome !== 'green' && row.reason_kind === null) return `${at} is ${row.outcome} without a reason_kind`;
  if (!isNullableString(row.reason)) return `${at}.reason must be a string or null`;
  if (row.last_run !== null && !isIsoTime(row.last_run)) return `${at}.last_run must be an ISO 8601 time or null`;
  if (row.report_url !== null && !isHttpsUrl(row.report_url)) return `${at}.report_url must be an https URL or null`;
  if (typeof row.target_build_exists !== 'boolean') return `${at}.target_build_exists must be true or false`;
  if (!Array.isArray(row.results) || row.results.length === 0) return `${at}.results must be a non-empty array`;
  const seen = new Set<string>();
  for (const [i, result] of row.results.entries()) {
    const problem = checkResult(result, `${at}.results[${i}]`, gameVersions);
    if (problem) return problem;
    const r = result as BoardResult;
    if (r.direction !== row.direction) return `${at}.results[${i}].direction differs from the row's direction`;
    if (seen.has(r.game_version)) return `${at}.results has ${r.game_version} twice`;
    seen.add(r.game_version);
  }
  const primary = (row.results as BoardResult[]).find((r) => r.game_version === row.primary_game_version);
  if (!primary) return `${at}.results has no entry for the primary game version`;
  if (row.outcome === 'green' && (primary.outcome !== 'green' || !oneOf(GREEN_SCORES, primary.score))) {
    return `${at} is green without a harness score of 1.0 or 0.75 on its primary game version`;
  }
  return null;
}

/** Validates a parsed value against the board.json schema. Extra fields are allowed. */
export function validateBoard(value: unknown): ParseResult {
  if (!isObject(value)) return { ok: false, problem: 'board.json is not a JSON object' };
  if (value.schema !== BOARD_SCHEMA) {
    return { ok: false, problem: `unsupported schema ${JSON.stringify(value.schema)}; this site reads schema ${BOARD_SCHEMA}` };
  }
  if (value.generated_at !== null && !isIsoTime(value.generated_at)) {
    return { ok: false, problem: 'generated_at must be an ISO 8601 time or null' };
  }
  if (value.run_id !== null && !isNonEmptyString(value.run_id)) {
    return { ok: false, problem: 'run_id must be a non-empty string or null' };
  }
  const engine = value.engine;
  if (engine !== null && !(isObject(engine) && isNonEmptyString(engine.version) && isNonEmptyString(engine.git_sha))) {
    return { ok: false, problem: 'engine must be {version, git_sha} or null' };
  }
  const versions = value.game_versions;
  if (!Array.isArray(versions) || !versions.every(isNonEmptyString) || new Set(versions).size !== versions.length) {
    return { ok: false, problem: 'game_versions must be an array of distinct strings' };
  }
  const totals = checkTotals(value.totals, 'totals');
  if (totals) return { ok: false, problem: totals };

  if (!Array.isArray(value.directions) || value.directions.length !== DIRECTIONS.length) {
    return { ok: false, problem: `directions must list all ${DIRECTIONS.length} directions` };
  }
  for (let i = 0; i < DIRECTIONS.length; i++) {
    const problem = checkDirection(value.directions[i], i);
    if (problem) return { ok: false, problem };
  }
  const directions = value.directions as BoardDirection[];
  const sum = zeroTotals();
  for (const d of directions) {
    for (const key of ['mods', 'green', 'yellow', 'red'] as const) sum[key] += d.totals[key];
  }
  const top = value.totals as BoardTotals;
  if ((['mods', 'green', 'yellow', 'red'] as const).some((key) => sum[key] !== top[key])) {
    return { ok: false, problem: "totals must equal the sum of the directions' totals" };
  }

  if (!Array.isArray(value.rows)) return { ok: false, problem: 'rows must be an array' };
  const counted = new Map<string, BoardTotals>(directions.map((d) => [d.id, zeroTotals()]));
  const keys = new Set<string>();
  for (let i = 0; i < value.rows.length; i++) {
    const problem = checkRow(value.rows[i], i, versions);
    if (problem) return { ok: false, problem };
    const row = value.rows[i] as BoardRow;
    const key = `${row.direction}/${row.project_id}`;
    if (keys.has(key)) return { ok: false, problem: `rows[${i}] repeats ${row.project_id} in ${row.direction}` };
    keys.add(key);
    const count = counted.get(row.direction)!;
    count.mods += 1;
    count[row.outcome] += 1;
  }
  for (const d of directions) {
    const count = counted.get(d.id)!;
    if (!d.column && count.mods > 0) {
      return { ok: false, problem: `rows name ${d.id}, which has no column` };
    }
    if ((['mods', 'green', 'yellow', 'red'] as const).some((key) => count[key] !== d.totals[key])) {
      return { ok: false, problem: `the rows of ${d.id} do not match its totals` };
    }
  }
  return { ok: true, board: value as unknown as BoardJson };
}

/** Parses and validates the text stored under the board.json key. */
export function parseBoard(text: string): ParseResult {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return { ok: false, problem: 'board.json is not valid JSON' };
  }
  return validateBoard(value);
}

/**
 * Reads board.json from KV. Never throws: a missing binding (local development) or a
 * missing key gives the empty board, and a bad value or a failed read is reported as such.
 */
export async function loadBoard(kv: BoardKV | null | undefined): Promise<BoardLoad> {
  if (!kv || typeof kv.get !== 'function') {
    return { status: 'unbound', board: emptyBoard() };
  }
  let raw: string | null;
  try {
    raw = await kv.get(BOARD_KV_KEY, 'text');
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { status: 'error', board: emptyBoard(), problem: `reading KV failed: ${message}` };
  }
  if (raw === null) return { status: 'missing', board: emptyBoard() };
  const parsed = parseBoard(raw);
  if (!parsed.ok) return { status: 'invalid', board: emptyBoard(), problem: parsed.problem };
  return { status: 'ok', board: parsed.board, raw };
}

/** Whether a nightly has produced this board. */
export function hasRun(load: BoardLoad): boolean {
  return load.status === 'ok' && load.board.generated_at !== null;
}

/** The directions that have a board column, in build order. Only these may be offered on the site. */
export function columns(board: BoardJson): BoardDirection[] {
  return board.directions.filter((d) => d.column);
}

/** Share of mods that are green, as a whole percentage, or null when there are no mods. */
export function greenPercent(totals: BoardTotals): number | null {
  if (totals.mods === 0) return null;
  return Math.floor((totals.green / totals.mods) * 100);
}

export const TIER_LABEL: Record<Tier, string> = {
  a: 'Tier A',
  b: 'Tier B',
  unsupported: 'Unsupported',
};

/** Labels for outcomes, as `jarrow_report::Outcome::label` writes them. */
export const RESULT_LABEL: Record<Outcome, string> = {
  green: 'green',
  yellow: 'yellow',
  red: 'red',
  'not-run': 'not run',
};
