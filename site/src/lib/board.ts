/**
 * The public board: the shape of `board.json` and the one loader every page and the
 * `/api/board.json` route use to read it.
 *
 * `board.json` lives in the KV namespace bound as `BOARD` under the key `board.json`. The
 * nightly board pipeline (crates/board, milestone M1) writes it. Until the first nightly runs,
 * the key is missing and the site shows an empty board: zero mods, no rows. Nothing here
 * invents a row or a number.
 *
 * Tier and outcome ids match `jarrow_report::Tier` and `jarrow_report::Outcome`
 * (crates/report/src/outcome.rs): tiers are "a", "b", "unsupported"; results are "green",
 * "yellow", "red", "not-run".
 *
 * This module has no runtime dependency on Astro or Cloudflare, so its tests run with
 * `node --test` (see board.test.ts).
 */

import { GAME_VERSIONS } from './site.ts';

/** KV key the nightly writes and the site reads. */
export const BOARD_KV_KEY = 'board.json';

/** Version of the board.json schema this site understands. */
export const BOARD_SCHEMA = 1;

export const TIERS = ['a', 'b', 'unsupported'] as const;
export type Tier = (typeof TIERS)[number];

export const RESULTS = ['green', 'yellow', 'red', 'not-run'] as const;
export type Result = (typeof RESULTS)[number];

/** One mod on the board. */
export interface BoardRow {
  /** Modrinth project id. */
  mod_id: string;
  /** Display name, as Modrinth lists it. */
  name: string;
  /** The mod's own Modrinth page. The board links here and never to a converted JAR. */
  page_url: string;
  /** Modrinth download count when the nightly ran. */
  downloads: number;
  tier: Tier;
  result: Result;
  /** Classified reason for a yellow or red result, or why the tier was chosen. */
  reason: string | null;
  /** ISO 8601 time of the last harness run for this mod. */
  last_run: string | null;
  /** Link to the full report, when one exists. */
  report_url: string | null;
}

export interface BoardTotals {
  mods: number;
  green: number;
  yellow: number;
  red: number;
}

/** The whole board as stored in KV and served at /api/board.json. */
export interface BoardJson {
  schema: typeof BOARD_SCHEMA;
  /** ISO 8601 time the nightly wrote this file; null before the first nightly. */
  generated_at: string | null;
  game_versions: string[];
  totals: BoardTotals;
  rows: BoardRow[];
}

/** The minimal part of a Workers KV namespace this module needs. */
export interface BoardKV {
  get(key: string, type: 'text'): Promise<string | null>;
}

/** The board before the first nightly: zero mods, no rows. */
export function emptyBoard(): BoardJson {
  return {
    schema: BOARD_SCHEMA,
    generated_at: null,
    game_versions: [...GAME_VERSIONS],
    totals: { mods: 0, green: 0, yellow: 0, red: 0 },
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

function checkRow(row: unknown, index: number): string | null {
  const at = `rows[${index}]`;
  if (!isObject(row)) return `${at} is not an object`;
  if (typeof row.mod_id !== 'string' || row.mod_id === '') return `${at}.mod_id must be a non-empty string`;
  if (typeof row.name !== 'string' || row.name === '') return `${at}.name must be a non-empty string`;
  if (!isHttpsUrl(row.page_url)) return `${at}.page_url must be an https URL`;
  if (!isCount(row.downloads)) return `${at}.downloads must be a non-negative integer`;
  if (!TIERS.includes(row.tier as Tier)) return `${at}.tier must be one of ${TIERS.join(', ')}`;
  if (!RESULTS.includes(row.result as Result)) return `${at}.result must be one of ${RESULTS.join(', ')}`;
  if (!isNullableString(row.reason)) return `${at}.reason must be a string or null`;
  if (row.last_run !== null && !isIsoTime(row.last_run)) return `${at}.last_run must be an ISO 8601 time or null`;
  if (row.report_url !== null && !isHttpsUrl(row.report_url)) return `${at}.report_url must be an https URL or null`;
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
  if (!Array.isArray(value.game_versions) || !value.game_versions.every((v) => typeof v === 'string')) {
    return { ok: false, problem: 'game_versions must be an array of strings' };
  }
  const totals = value.totals;
  if (!isObject(totals)) return { ok: false, problem: 'totals is missing' };
  for (const key of ['mods', 'green', 'yellow', 'red'] as const) {
    if (!isCount(totals[key])) return { ok: false, problem: `totals.${key} must be a non-negative integer` };
  }
  const t = totals as unknown as BoardTotals;
  if (t.green + t.yellow + t.red > t.mods) {
    return { ok: false, problem: 'totals.green + totals.yellow + totals.red exceeds totals.mods' };
  }
  if (!Array.isArray(value.rows)) return { ok: false, problem: 'rows must be an array' };
  for (let i = 0; i < value.rows.length; i++) {
    const problem = checkRow(value.rows[i], i);
    if (problem) return { ok: false, problem };
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

export const RESULT_LABEL: Record<Result, string> = {
  green: 'green',
  yellow: 'yellow',
  red: 'red',
  'not-run': 'not run',
};
