// GET /api/board.json: the board as the nightly stored it in KV, or the empty board before
// the first nightly. Public data, so any origin may read it (shields.io endpoint badges).
import type { APIRoute } from 'astro';

import { emptyBoard } from '../../lib/board.ts';
import { readBoard } from '../../lib/runtime.ts';

export const prerender = false;

const BASE_HEADERS = {
  'Content-Type': 'application/json; charset=utf-8',
  'Access-Control-Allow-Origin': '*',
  'X-Content-Type-Options': 'nosniff',
};

export const GET: APIRoute = async () => {
  const load = await readBoard();

  if (load.status === 'invalid' || load.status === 'error') {
    // Never serve a board we could not validate, and never pretend it is empty.
    return new Response(JSON.stringify({ error: 'board.json is unavailable right now' }), {
      status: 503,
      headers: { ...BASE_HEADERS, 'Cache-Control': 'no-store', 'Retry-After': '300' },
    });
  }

  const body = load.status === 'ok' ? load.raw : JSON.stringify(emptyBoard());
  return new Response(body, {
    status: 200,
    headers: { ...BASE_HEADERS, 'Cache-Control': 'public, max-age=300' },
  });
};
