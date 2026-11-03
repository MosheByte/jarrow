// GET /api/badge/parity.json: the shields.io endpoint behind the README "parity" badge, the
// share of board mods that are green, from the same board.json as /api/board.json.
import type { APIRoute } from 'astro';

import { parityBadge } from '../../../lib/board.ts';
import { readBoard } from '../../../lib/runtime.ts';

export const prerender = false;

export const GET: APIRoute = async () => {
  const badge = parityBadge(await readBoard());
  return new Response(JSON.stringify(badge), {
    status: 200,
    headers: {
      'Content-Type': 'application/json; charset=utf-8',
      'Access-Control-Allow-Origin': '*',
      'X-Content-Type-Options': 'nosniff',
      'Cache-Control': badge.isError ? 'no-store' : 'public, max-age=300',
    },
  });
};
