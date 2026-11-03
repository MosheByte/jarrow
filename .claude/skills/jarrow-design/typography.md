# Type

Measurements: `docs/design-research/horizon.md`, section 2.1 (roles), section 9 (fonts), section 6 (copy). Tokens: the "Type" group in `site/tokens.css`. Fonts and fallbacks: `site/src/styles/fonts.css`.

## Families

| Token | Family | Role | Weight |
| --- | --- | --- | --- |
| `--jw-font-display` | Bricolage Grotesque (variable, 200 to 800) | h1, h2, display numbers | 600 (`--jw-weight-display`) |
| `--jw-font-body` | Geist (variable) | everything else: body, h3, labels, nav, buttons | 400; 500 for h3, labels, eyebrows, buttons (`--jw-weight-medium`) |
| `--jw-font-mono` | Geist Mono (variable) | `code`, `pre`, docs inline code | 400 |

All three are self-hosted through Fontsource; no font comes from another host. Every face is `font-display: swap`. The Latin files of Bricolage Grotesque and Geist are preloaded on every page (`src/layouts/Base.astro`); the other subsets load only for their code points. There is no pixel font anywhere: the pixel language stays in the scenes and the grid (report section 4).

### Fallbacks (CLS 0)

Each family falls back to a metric-matched local Arial, so the swap to the web font moves nothing. The overrides are computed from the woff2 files Fontsource ships (Capsize's method: `size-adjust` is the ratio of average character widths; each override is the web font's metric divided by unitsPerEm × size-adjust):

| Fallback | size-adjust | ascent | descent | line gap |
| --- | --- | --- | --- | --- |
| Bricolage Grotesque Fallback | 105.43 % | 88.21 % | 25.61 % | 0 % |
| Geist Fallback | 104.76 % | 95.94 % | 28.16 % | 0 % |
| Geist Mono Fallback | 134.59 % | 74.67 % | 21.92 % | 0 % |

Geist's and Geist Mono's values equal the report's; Bricolage keeps the report's 105.43 % and 88.21 %. A new family needs its own computed fallback before it ships.

## The scale

The root is 16 px; every size below is in rem and assumes it. Never change the root size.

| Role | Size token | Value | Line height | Tracking | Family, weight | Measured (report 2.1) |
| --- | --- | --- | --- | --- | --- | --- |
| Page h1 | `--jw-h1-size` | `clamp(2rem, 10.4cqi, 4.75rem)` | `--jw-h1-leading` 1.02 | `--jw-h1-tracking` −0.035 em | display 600 | 59.9 px at 1440, 35.6 px at 390 |
| Section h2 | `--jw-h2-size` | `clamp(2rem, 3.6vw, 3rem)` | 1.08 | −0.03 em | display 600 | 48 px, 32 px at 390 |
| Card h2 | `--jw-h2-card-size` | 2rem | 1.1 | −0.02 em | display 600 | 32 px |
| Docs article h1 | `--jw-docs-h1-size` | `clamp(2rem, 4.5vw, 3rem)` | 1.08 | −0.03 em | display 600 | 48 px, 32 px at 390 |
| Docs h2 | `--jw-docs-h2-size` | 1.625rem | 1.2 | −0.02 em | display 600 | 26 px |
| Docs h3 | `--jw-docs-h3-size` | 1.1875rem | 1.3 | −0.01 em | body 600 | 19 px |
| Feature h3 | `--jw-h3-size` | 1.125rem | 1.75rem (28 px) | normal | body 500 | 18 px |
| Card number | `--jw-number-size` | 2.5rem | 1 | −0.02 em | display 600 | 40 px |
| Section number | `--jw-number-xl-size` | `clamp(6rem, 18vw, 11rem)` | 1 | −0.02 em | display 600 | 144 px |
| Hero lede | `--jw-lede-size` | 1.125rem | 1.625 | normal | body 400, white 70 % | 18 px |
| Card and feature body | `--jw-body-size` | 0.9375rem | 1.625 | normal | body 400, white 60 % | 15 px |
| Docs body | `--jw-docs-body-size` | 1rem | 1.7 | normal | body 400, white 72 % | 16 px |
| Label, eyebrow, nav, footer | `--jw-label-size` | 0.875rem | 1.25rem (20 px); footer paragraphs 1.625 | normal | body 500 (labels), 400 (nav, footer text) | 14 px |
| Docs rail group label | `--jw-rail-label-size` | 0.8125rem | 1.5 | normal | body 500, white 50 % | 13 px |
| Hint | `--jw-small-size` | 0.75rem | 1.625 | normal | body 400, white 50 % | 12 px |
| Pill count | `--jw-pill-size` | 0.625rem | 1 | normal | body 600, page colour on accent | 10 px |
| Inline code | `--jw-code-size` | 0.84em, padding `--jw-code-padding` | inherits | normal | mono, `--jw-fg` on white 7 %, radius 5 px | 13.44 px in 16 px text |
| Code block | `--jw-pre-size` | 0.84375rem, padding `--jw-pre-padding` | 1.65 | normal | mono, `--jw-fg` on white 4 %, radius 12 px | 13.5 px |

The page h1 uses `cqi` units: its parent is an inline-size container (`container-type: inline-size`), the hero's text column (576 px wide at 1440). Without the container, `cqi` falls back to the viewport and the h1 sets at its 4.75rem maximum. Check rule 3 with a screenshot at both widths.

## Rules

1. **Two lines, one accent.** A page h1 is two lines; the last clause sits on its own line in a `<span>` with the accent gradient (rule 24): the first line names the subject, the second is the promise (report section 6: eight of eight page heroes in the reference). The accent never marks a content h2, a docs heading, a footer heading or a question.
2. **No uppercase.** `text-transform: uppercase` appears on no element, and no copy is typed in capitals for emphasis (rule 6). Eyebrows are sentence case, 14 px, weight 500, white at 60 %.
3. **Numbers are display type.** Big counts are Bricolage 600 at 40 or 144 px with the unit beside them in 14 px at white 50 % ("1,000 mods", "4.1 MiB"). Every number that changes or aligns has `font-variant-numeric: tabular-nums` (rule 14). Index numbers ("01" to "14") are 12 px at white 40 %, decorative.
4. **Wrapping.** Headings `text-wrap: balance`; paragraphs `text-wrap: pretty`.
5. **The floor.** Text at white 50 % is never under 12 px; nothing under 50 % carries information (rules 7, 29).
6. **Weights.** 600 for display and docs h3, 500 for h3, labels, buttons and footer headings, 400 for everything else. No 700 or 800 anywhere.
7. **Mono is for code.** Never for labels, eyebrows, pills or numbers.

## Copy casing and figures

- Sentence case for every heading, nav item, button, label, tab and table header.
- Full stop at the end of a heading that is a sentence ("Every release, as it ships."); none on a label ("Board").
- Figures, not words, for numbers: "3 loaders", "1,000 mods", "60 Hz", with thousands separators and the unit after a space. Versions as written by their projects ("1.21.1", "NeoForge 21.1.77").
