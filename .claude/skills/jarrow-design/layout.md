# Layout

Measurements: `docs/design-research/horizon.md`, section 1 (inventory and the shared layouts) and section 2.3 (spacing, container, radii, borders, shadows). Tokens: the "Space", "Layout", "Radius" and "Card" groups in `site/tokens.css`.

## The grid

- The unit is 4 px (`--jw-space`). Every margin, padding, gap, height and column is a multiple of it (rule 10). Use the scale: `--jw-space-1` (4 px), `-2` (8), `-3` (12), `-4` (16), `-5` (20), `-6` (24), `-8` (32), `-10` (40), `-12` (48), `-16` (64), `-18` (72), `-24` (96). In Tailwind, `p-6`, `gap-x-16`, `pt-12` are the same steps (`--spacing: 0.25rem`).
- Exceptions the reference itself makes, and only these: the 10 px icon-to-label gap in buttons (`--jw-button-gap`), radii, and type sizes.
- The scene's pixel grid is its own: `px = clamp(round(width / 340), 3, 6)` (`motion.md`).

## The container

- 1152 px (`--jw-container`), centred. Its gutters are inside it: 24 px (`--jw-gutter`) below 1024 px, 40 px (`--jw-gutter-wide`) from 1024 px (`64rem`). At 1440 the content starts at x = 184; at 390 at x = 24 (`home-1440-first.png`, `home-390-first.png`).
- The `.wrap` class in `site/src/styles/site.css` is this container.

## The header

72 px tall (`--jw-header-height`), fixed, transparent at scroll 0, then `--jw-page-75` under a 12 px backdrop blur (`--jw-header-blur`) after scrolling, with a 300 ms transition (`--jw-header-duration`) on the background and the blur. No border, no shadow. Sections carry `scroll-margin-top` of at least the header height so anchors clear it. Details: `components/header.md`.

## Hero

- At 1440 the hero is the first screen (`min-height: 100svh`, 900 px): copy at left, the hero card at right, the scene in the bottom 230 px (rule 17). Nothing overlaps the sun or the ridges.
- At 390 the copy, buttons and card stack; the scene is a 253 px strip under the content (`clamp(0.3 × innerHeight, 180, 320)`), and the sun is at 50 % width.
- The hero's bottom edge is a straight cut at the scene's bottom; the scene's last sky stop is the page colour, so there is no gradient bridge into the next section (rule 19).

## Docs

Three columns at 1440: a 208 px rail (`--jw-docs-rail`) with search and grouped links, a 600 px article (`--jw-docs-article`), and a 176 px "On this page" rail (`--jw-docs-toc`). At 390: a search field, a "Docs menu" disclosure, the breadcrumb, the article and an "On this page" disclosure. Details: `components/docs-layout.md`.

## Radii

| Token | Value | Where (report 2.3) |
| --- | --- | --- |
| `--jw-radius-2` | 2 px | stat bars, slider thumb |
| `--jw-radius-3` | 3 px | legend and score squares |
| `--jw-radius-5` | 5 px | inline code |
| `--jw-radius-6` | 6 px | code copy button, pill toggle |
| `--jw-radius-8` | 8 px | header and 40 px buttons, hamburger, search field |
| `--jw-radius-10` | 10 px | 48 px hero buttons |
| `--jw-radius-12` | 12 px | code blocks, tables, dropdowns, blockquotes |
| `--jw-radius-16` | 16 px | cards, figures (`clip-path: inset(0 round 1rem)`), panels |
| `--jw-radius-24`, `--jw-radius-28` | 24, 28 px | large panels; the one light panel |
| `--jw-radius-full` | 9999 px | pills, avatars |

## Surfaces, borders, shadows

- A surface is white at 3 to 8 % over the page (`colors.md`); separation is by alpha, never an edge (rule 12).
- No borders on cards, buttons, the header or panels (`--jw-card-border: 0`). The only rules: `1px solid var(--jw-white-8)` on top of the footer and between rows, and a 2 px `--jw-white-15` left rule on blockquotes.
- No shadows (`--jw-card-shadow: none`). The one glow is the pill's and a live dot's: `0 0 12px -2px var(--jw-accent-45)`.
- Backdrop blur: the hero card 24 px (`--jw-card-blur`), the scrolled header 12 px. Nowhere else.

## Responsive

- Breakpoints are Tailwind's: 40rem, 48rem, 64rem, 80rem. The gutter switches at 64rem; the hero stacks below 64rem.
- No horizontal page scroll at 390. Wide tables scroll inside their own region.
- Touch: under `(hover: none)`, controls that reveal on hover are always visible (the code copy button) and hit targets are at least 40 px.
