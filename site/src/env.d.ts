// Types for the Cloudflare bindings in wrangler.toml, as the site uses them.
// Only the methods the site calls are declared; add to them as routes need more.

declare module 'cloudflare:workers' {
  interface JarrowKV {
    get(key: string, type: 'text'): Promise<string | null>;
  }

  interface JarrowEnv {
    /** KV namespace "jarrow-board": cached board.json written by the nightly. */
    BOARD?: JarrowKV;
    /** D1 database "jarrow". Not read by any route yet. */
    DB?: unknown;
    /** R2 bucket "jarrow-reports". Not read by any route yet. */
    REPORTS?: unknown;
    /** Canonical origin, mirrors src/lib/site.ts. */
    SITE_URL?: string;
  }

  export const env: JarrowEnv;
}
