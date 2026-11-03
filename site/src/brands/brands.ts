/**
 * Every third-party brand the site names, and how the site shows it.
 *
 * This list mirrors the "Brands" table in site/BRAND-SOURCES.md, which is the record of each
 * brand's official page, guideline, licence and constraints. scripts/brand-check.test.mjs fails
 * when the two disagree, and scripts/brand-check.mjs fails the build on any mark that is not
 * an official SVG recorded there.
 *
 * A brand is `logo` only when its artwork licence and its published trademark terms both allow
 * a non-affiliated site to show the mark; everything else is `text`, set in the site's own type.
 * Render brands with BrandMark.astro, never by hand.
 */

export type BrandId =
  | 'minecraft'
  | 'mojang'
  | 'microsoft'
  | 'windows'
  | 'winget'
  | 'fabric'
  | 'neoforge'
  | 'forge'
  | 'quilt'
  | 'modrinth'
  | 'curseforge'
  | 'github'
  | 'github-sponsors'
  | 'github-actions'
  | 'codeql'
  | 'dependabot'
  | 'gitlab'
  | 'rust'
  | 'cargo'
  | 'crates-io'
  | 'webassembly'
  | 'npm'
  | 'homebrew'
  | 'scoop'
  | 'aur'
  | 'linux'
  | 'macos'
  | 'gradle'
  | 'sigstore'
  | 'openssf'
  | 'cloudflare'
  | 'anthropic'
  | 'claude'
  | 'codecov'
  | 'sentry'
  | 'posthog'
  | 'better-stack'
  | 'uptimerobot'
  | 'sinytra-connector'
  | 'forgified-fabric-api'
  | 'architectury';

/** A credit line the licence of a mark asks for, shown by BrandCredits.astro. */
export interface BrandCredit {
  /** The sentence, without links. */
  text: string;
  /** Where the artwork comes from: link text and URL. */
  sourceLabel: string;
  sourceUrl: string;
  /** The licence: link text and URL. */
  licenceLabel: string;
  licenceUrl: string;
}

export interface BrandMarkInfo {
  /** The component in src/brands/ that renders the official inline SVG. */
  component: string;
  /** The official kit page, the same URL as the SVG's data-brand-source and BRAND-SOURCES.md. */
  source: string;
  credit?: BrandCredit;
}

export interface Brand {
  /** The brand's own spelling, used as the adjacent text. */
  name: string;
  shownAs: 'logo' | 'text';
  /** Present only for `logo` brands. */
  mark?: BrandMarkInfo;
}

export const BRANDS: Record<BrandId, Brand> = {
  minecraft: { name: 'Minecraft', shownAs: 'text' },
  mojang: { name: 'Mojang Studios', shownAs: 'text' },
  microsoft: { name: 'Microsoft', shownAs: 'text' },
  windows: { name: 'Windows', shownAs: 'text' },
  winget: { name: 'winget', shownAs: 'text' },
  fabric: { name: 'Fabric', shownAs: 'text' },
  neoforge: { name: 'NeoForge', shownAs: 'text' },
  forge: { name: 'Forge', shownAs: 'text' },
  quilt: { name: 'Quilt', shownAs: 'text' },
  modrinth: { name: 'Modrinth', shownAs: 'text' },
  curseforge: { name: 'CurseForge', shownAs: 'text' },
  github: {
    name: 'GitHub',
    shownAs: 'logo',
    mark: {
      component: 'GitHub',
      source: 'https://primer.style/octicons/icon/mark-github-16/',
      credit: {
        text: "The GitHub mark is GitHub's Octicon mark-github, copyright GitHub, Inc.",
        sourceLabel: 'primer/octicons',
        sourceUrl: 'https://github.com/primer/octicons',
        licenceLabel: 'MIT licence',
        licenceUrl: 'https://github.com/primer/octicons/blob/main/LICENSE',
      },
    },
  },
  'github-sponsors': { name: 'GitHub Sponsors', shownAs: 'text' },
  'github-actions': { name: 'GitHub Actions', shownAs: 'text' },
  codeql: { name: 'CodeQL', shownAs: 'text' },
  dependabot: { name: 'Dependabot', shownAs: 'text' },
  gitlab: { name: 'GitLab', shownAs: 'text' },
  rust: {
    name: 'Rust',
    shownAs: 'logo',
    mark: {
      component: 'Rust',
      source: 'https://github.com/rust-lang/rust-artwork/tree/main/logo',
      credit: {
        text: 'The Rust logo is owned by the Rust Foundation; it is shown in the text colour instead of black.',
        sourceLabel: 'rust-lang/rust-artwork',
        sourceUrl: 'https://github.com/rust-lang/rust-artwork/tree/main/logo',
        licenceLabel: 'CC BY 4.0',
        licenceUrl: 'https://creativecommons.org/licenses/by/4.0/',
      },
    },
  },
  cargo: { name: 'Cargo', shownAs: 'text' },
  'crates-io': { name: 'crates.io', shownAs: 'text' },
  webassembly: {
    name: 'WebAssembly',
    shownAs: 'logo',
    mark: {
      component: 'WebAssembly',
      source: 'https://github.com/carlosbaraza/web-assembly-logo',
      credit: {
        text: 'The WebAssembly logo is by Carlos Baraza.',
        sourceLabel: 'carlosbaraza/web-assembly-logo',
        sourceUrl: 'https://github.com/carlosbaraza/web-assembly-logo',
        licenceLabel: 'CC0 1.0',
        licenceUrl: 'https://creativecommons.org/publicdomain/zero/1.0/',
      },
    },
  },
  npm: { name: 'npm', shownAs: 'text' },
  homebrew: { name: 'Homebrew', shownAs: 'text' },
  scoop: { name: 'Scoop', shownAs: 'text' },
  aur: { name: 'AUR', shownAs: 'text' },
  linux: { name: 'Linux', shownAs: 'text' },
  macos: { name: 'macOS', shownAs: 'text' },
  gradle: { name: 'Gradle', shownAs: 'text' },
  sigstore: { name: 'Sigstore', shownAs: 'text' },
  openssf: { name: 'OpenSSF', shownAs: 'text' },
  cloudflare: { name: 'Cloudflare', shownAs: 'text' },
  anthropic: { name: 'Anthropic', shownAs: 'text' },
  claude: { name: 'Claude', shownAs: 'text' },
  codecov: { name: 'Codecov', shownAs: 'text' },
  sentry: { name: 'Sentry', shownAs: 'text' },
  posthog: { name: 'PostHog', shownAs: 'text' },
  'better-stack': { name: 'Better Stack', shownAs: 'text' },
  uptimerobot: { name: 'UptimeRobot', shownAs: 'text' },
  'sinytra-connector': { name: 'Sinytra Connector', shownAs: 'text' },
  'forgified-fabric-api': { name: 'Forgified Fabric API', shownAs: 'text' },
  architectury: { name: 'Architectury', shownAs: 'text' },
};

/** The brand for an id; throws on an unknown id so a typo fails the build. */
export function brand(id: BrandId): Brand {
  const entry = BRANDS[id];
  if (!entry) throw new Error(`Unknown brand id "${id}". Add it to src/brands/brands.ts and site/BRAND-SOURCES.md first.`);
  return entry;
}
