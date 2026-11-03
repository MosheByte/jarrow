/**
 * The one place server-rendered routes touch Cloudflare bindings. Kept apart from board.ts
 * so the board logic stays testable without a Workers runtime.
 */

import { env } from 'cloudflare:workers';

import { type BoardLoad, loadBoard } from './board.ts';

/** Reads board.json from the BOARD binding; empty board when the binding is absent. */
export async function readBoard(): Promise<BoardLoad> {
  const load = await loadBoard(env?.BOARD);
  if (load.status === 'invalid' || load.status === 'error') {
    // Workers logs are the error sink until Sentry is wired (D-009).
    console.error(`board.json: ${load.status}: ${load.problem}`);
  }
  return load;
}
