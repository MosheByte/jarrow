// The measured values the thirty rules check, in one place. They are Horizon's values from
// docs/design-research/horizon.md with Jarrow's four substitutions applied: the night-blue page,
// the amber accent and primary button, the sunrise scene, and the amber sky and sun ramps.
// docs/design/thirty-rules.md states the same numbers in words; change both together.

export const SPEC = {
  colours: {
    page: '#0b0f1f',
    foreground: '#f4f3ff',
    white: '#ffffff',
    primary: '#ffb454',
    primaryHover: '#ffc26b',
    primaryText: '#1a1200',
    accent: '#ffb454',
    gradient: ['#ffe3b0', '#ffb454', '#ff8a3d'],
    tierGreen: '#8be0b4',
    tierAmber: '#f2c46b',
    tierRed: '#ff8a8a',
  },
  /** White text steps, as alpha (rule 7). */
  textAlphas: [1, 0.8, 0.7, 0.65, 0.6, 0.5],
  fonts: {
    display: 'Bricolage Grotesque',
    body: 'Geist',
    mono: 'Geist Mono',
    displayWeight: 600,
  },
  h1: { 1440: 60, 390: 36, tolerancePx: 1, maxLineHeight: 1.05, letterSpacingEm: -0.035 },
  h2: { 1440: 48, 390: 32, tolerancePx: 1, lineHeight: 1.08, letterSpacingEm: -0.03 },
  body: { minPx: 15, maxPx: 18, lineHeight: 1.625 },
  docsBody: { px: 16, lineHeight: 1.7 },
  eyebrow: { px: 14, weight: 500, alpha: 0.6 },
  unit: { px: 14, alpha: 0.5 },
  spacingUnitPx: 4,
  /** The one spacing exception the rules name: the 10 px icon gap inside buttons (rule 13). */
  iconGapPx: 10,
  container: { width: 1152, gutterWide: 40, gutterNarrow: 24, wideFrom: 1024 },
  header: { height: 72, scrolledAlpha: 0.75, blurPx: 12 },
  card: { radius: 16, paddingMin: 24, paddingMax: 32, alphaMin: 0.03, alphaMax: 0.08 },
  buttons: {
    heights: [48, 40, 36],
    radii: [10, 8],
    fontPx: [15, 14],
    weight: 500,
    iconMinPx: 17,
    iconMaxPx: 18,
    gapPx: 10,
  },
  scene: {
    fadeMs: 700,
    maxFps: 32,
    /** Height of the scene strip under the content at 390 x 844: clamp(0.3 * 844, 180, 320). */
    strip390: 253,
    /** The hills occupy the bottom of the hero at 1440. */
    band1440: 230,
  },
  entrance: { durationS: 0.9, easing: 'cubic-bezier(0.2, 0.7, 0.2, 1)', translatePx: 14 },
  hoverMs: 150,
  contrastMin: 4.9,
  minTextPx: 12,
  ledeMaxWords: 25,
  /** Lighthouse bars (docs/site-quality.md). */
  lighthouse: { mobilePerformance: 0.95, desktopPerformance: 0.98, accessibility: 1, cls: 0, homeTransferKiB: 350 },
};

/** The pixel size of a scene's low-resolution layer at a viewport width (rule 15). */
export function pixelSize(viewportWidth) {
  return Math.min(6, Math.max(3, Math.round(viewportWidth / 340)));
}

export const VIEWPORTS = {
  1440: { width: 1440, height: 900, label: '1440 x 900' },
  390: { width: 390, height: 844, label: '390 x 844' },
};

/** Android Chrome, as the Horizon captures at 390 used (touch enabled). */
export const MOBILE_UA =
  'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36';
