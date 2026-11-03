# Docs layout

Measured: `docs/design-research/horizon.md`, section 1 ("Docs pages share one layout"), section 2.1 (docs h1, h2, h3, body, inline code, code blocks), section 9 (fonts on docs pages). Screenshots: `docs-1440-first.png`, `docs-390-first.png`, `docs-install-1440-full.png`; data: `docs-1440.json`, `docs-390.json`.

The docs are Starlight. Its variables are mapped to the tokens in `site/src/styles/global.css` (`--sl-color-*`, `--sl-font*`, `--sl-text-*`, `--sl-nav-height`, `--sl-content-width`), and its own selectors are set in `site/src/styles/starlight.css`. Code blocks are Expressive Code, themed from the tokens in `site/ec.config.mjs`.

## 1440: three columns in the container

| Column | Width | Content |
| --- | --- | --- |
| Left rail | `--jw-docs-rail` 208 px | a search field (40 px, radius 8 px, `--jw-white-6`, 14 px text, a "/" shortcut hint; the reference's placeholder is white 40 %, Jarrow's is `--jw-white-50`), then the groups: a 13 px weight 500 label (`--jw-rail-label-size`) in `--jw-white-50` (the reference uses 40 %, which fails rule 29) and its 14 px links; the current page `--jw-white` on `--jw-white-8`, radius 8 px |
| Article | `--jw-docs-article` 600 px | breadcrumb, h1, lede (18 px, `--jw-white-70`), body |
| Right rail | `--jw-docs-toc` 176 px | "On this page" and the h2 links, 14 px; compare with `docs-1440-first.png` |

## 390

A search field, a "Docs menu" disclosure holding the rail, the breadcrumb, the h1, the lede, an "On this page" disclosure, then the article. No horizontal scroll; code blocks and tables scroll inside themselves.

## The article

| Element | Tokens |
| --- | --- |
| h1 | `--jw-docs-h1-size` (48 px, 32 px at 390), display 600, line height `--jw-docs-h1-leading`, tracking `--jw-docs-h1-tracking`, `--jw-white`; never the accent gradient |
| h2 | `--jw-docs-h2-size` 26 px, display 600, 1.2, −0.02 em, white; a `#` anchor shows on hover and on `:focus-visible` (150 ms opacity) |
| h3 | `--jw-docs-h3-size` 19 px, Geist 600, 1.3, −0.01 em, white |
| Body | `--jw-docs-body-size` 16 px, line height 1.7, `--jw-white-72` |
| Links | `--jw-accent`, underlined, hover `--jw-primary-hover` |
| Inline code | Geist Mono 0.84 em, `--jw-fg` on `--jw-white-7`, radius 5 px, padding 0.12 em 0.38 em |
| Code block | Geist Mono 13.5 px, 1.65, `--jw-fg` on `--jw-white-4`, radius 12 px, padding 1em 1.15em, no border, no syntax colours; the copy button always visible under `(hover: none)` |
| Blockquote | 2 px `--jw-white-15` left rule, radius 12 px |
| Tables | 14 px, rules `--jw-white-8` between rows, header `--jw-white-80` 500, radius 12 px on the scroll region |
| Asides | `--jw-white-4` surface with a 4 px left rule: neutral (`--jw-white-15`) for notes and tips, `--jw-tier-amber` for caution, `--jw-tier-red` for danger |

## Rules

- Docs headings are white. The accent marks links, focus and the current item of "On this page" only.
- Every docs page is static and renders fully without JavaScript; search is the only thing that needs it.
- The header is the site's 72 px header (`--sl-nav-height`), dark only: no theme switch.
