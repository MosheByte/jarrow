# Header and nav

Measured: `docs/design-research/horizon.md`, section 1 ("every page shares the same 72 px fixed header"), section 2.3 (header, buttons), section 2.1 (nav link type), section 8 (the header transition), section 10 (focus). Rule 11. Screenshots: `home-1440-first.png`, `home-390-first.png`; data: `home-measure.json` (`headerScrolled`).

## Anatomy, 1440

`[wordmark]` at the container's left edge, then the nav links, then at the right `[GitHub mark + "GitHub"]` as a header text link and one header button. All inside the 1152 px container, vertically centred in 72 px.

## Tokens

| Part | Value |
| --- | --- |
| Height | `--jw-header-height` 72 px; `position: fixed`, full width, above the page |
| Background at scroll 0 | transparent |
| Background after scrolling | `--jw-page-75` with `backdrop-filter: blur(var(--jw-header-blur))` (12 px), transition on background-color and backdrop-filter over `--jw-header-duration` (300 ms) |
| Edge | none: no border, no shadow |
| Wordmark | "Jarrow", Bricolage 600, `--jw-white` (`brand.md`) |
| Nav links | 14 px (`--jw-label-size`), weight 400, line height 20 px, `--jw-white-65`; hover and current page `--jw-white` over 150 ms; spacing on the 4 px grid, compared with `home-1440-first.png` |
| Count pill | 16 px tall, min 16 px wide, radius full, 10 px weight 600 (`--jw-pill-size`), `--jw-page` on `--jw-accent`, glow `0 0 12px -2px var(--jw-accent-45)`; a visually hidden text says what it counts |
| Header button | `components/button.md`, 36 px |

## 390

The wordmark at left; the header button and the 36 × 36 px hamburger (`--jw-white-6`, icon `--jw-white-80`, radius 8 px) at right. The menu opens a full-width panel on `--jw-page` with the same links at 16 px. Without JavaScript the links are reachable another way (the footer repeats them), so the page still has full navigation (rule 30).

## States

- **Current page:** `aria-current="page"` and `--jw-white`; no underline bar, no accent.
- **Focus-visible:** the 2 px accent ring, offset 2 px.
- **Scrolled:** a scroll listener (or a sentinel `IntersectionObserver`) toggles one attribute; the colour change is CSS. Under reduced motion the change is instant.

## Rules

- Nav items are sentence case nouns that name the destination ("Board", "Docs", "Roadmap"). No uppercase, no icons on nav items other than an official brand mark beside its brand name.
- Every section the nav links to has `scroll-margin-top` of at least 72 px so anchors clear the header.
- No theme toggle: the site is dark only.
