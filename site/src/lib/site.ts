/**
 * Site-wide constants. Every absolute link the site builds starts from here.
 *
 * SITE_URL is the one place the site's origin is written down: by design, all links are
 * built from this one constant. When the project owner attaches a custom domain, change it
 * here and in wrangler.toml [vars], nowhere else.
 */

export const PRODUCT_NAME = 'Jarrow';

/** Canonical origin of the site, without a trailing slash. */
export const SITE_URL = 'https://jarrow.pages.dev';

/** The one-sentence mission. */
export const MISSION =
  'Drop a Minecraft mod JAR on a web page; get the same mod back for another loader, with a proof that it behaves the same.';

/** Shorter description used in meta tags. */
export const DESCRIPTION =
  'Jarrow will convert a Minecraft mod JAR from any loader to any loader and prove each result with a behaviour harness. Nothing converts yet: the project is in milestone M0, and the nightly board measures only Fabric → NeoForge, where every mod is red until a converter exists.';

export const REPO_SLUG = 'MosheByte/jarrow';
export const REPO_URL = `https://github.com/${REPO_SLUG}`;
export const ROADMAP_PROJECT_URL = 'https://github.com/users/MosheByte/projects/1';
export const API_DOCS_URL = 'https://moshebyte.github.io/jarrow/';
export const NEW_ISSUE_URL = `${REPO_URL}/issues/new/choose`;
export const DISCUSSIONS_URL = `${REPO_URL}/discussions`;

/** GitHub Actions status badge for ci.yml on main. It is the repository's own status. */
export const CI_WORKFLOW_URL = `${REPO_URL}/actions/workflows/ci.yml`;
export const CI_BADGE_URL = `${CI_WORKFLOW_URL}/badge.svg?branch=main`;

/**
 * Minecraft versions v1 targets: 1.21.1 and 1.21.11 (D-002, docs/research/game-versions.json),
 * plus 1.20.1 (D-029). Oldest first.
 */
export const GAME_VERSIONS = ['1.20.1', '1.21.1', '1.21.11'] as const;

/** The milestone the project is in. Update when a milestone closes. */
export const CURRENT_MILESTONE = 'M0';

/** Link to a file on the default branch of the repository. */
export function repoFile(path: string): string {
  return `${REPO_URL}/blob/main/${path.replace(/^\/+/, '')}`;
}

/** Absolute URL on this site for a path such as "/board/". */
export function absolute(path: string): string {
  return new URL(path, `${SITE_URL}/`).toString();
}
