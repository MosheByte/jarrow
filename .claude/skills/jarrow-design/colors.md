# Colour

Source of truth: `site/tokens.css`, generated from `crates/report/src/tokens.rs`. The reference's measurements are in `docs/design-research/horizon.md`, section 2.2 (colour) and section 10 (contrast). Contrast below is WCAG 2.x, measured as the report does: the alpha colour composited over the page (`#0b0f1f`) or over the hero card (white at 4 % over the page, `#151928`) and read back as an opaque colour.

## The palette

### Page

| Token | Value | Use | Reference |
| --- | --- | --- | --- |
| `--jw-page` | `#0b0f1f` | page background, `<meta name="theme-color">`, text on light pills | `#100a1f` |
| `--jw-page-75` | page at 75 % | the header after scrolling (under a 12 px backdrop blur), dialog backdrops | `#100a1f` at 75 % |

### Text

| Token | Composited on page | On page | On card | Use |
| --- | --- | --- | --- | --- |
| `--jw-white` | `#ffffff` | 19.1:1 | 17.5:1 | headings, primary labels, display numbers |
| `--jw-fg` | `#f4f3ff` | 17.3:1 | 15.9:1 | body text default, code text |
| `--jw-white-90` | `#e7e7e9` | 15.4:1 | 14.3:1 | white button hover |
| `--jw-white-80` | `#cecfd2` | 12.2:1 | 11.4:1 | footer headings, disclaimer, hamburger icon, tier labels |
| `--jw-white-72` | `#bbbcc0` | 10.0:1 | 9.5:1 | docs body |
| `--jw-white-70` | `#b6b7bc` | 9.5:1 | 9.0:1 | hero lede, footer links |
| `--jw-white-65` | `#aaabb1` | 8.3:1 | 8.0:1 | nav links |
| `--jw-white-60` | `#9d9fa5` | 7.2:1 | 6.9:1 | card and feature body, eyebrows, labels |
| `--jw-white-50` | `#85878f` | 5.3:1 | 5.2:1 | hints, footer paragraphs, units, docs rail labels: the floor for information |
| `--jw-white-45` | `#797b84` | 4.5:1 | 4.5:1 | decorative only |
| `--jw-white-40` | `#6d6f79` | 3.8:1 | 3.8:1 | decorative only (an index number beside its label) |

Below 50 % nothing carries information (rule 7). The reference sets its docs rail group labels at 40 %; Jarrow sets them at 50 %, because 3.8:1 fails rule 29.

### Surfaces

White over the page, never a new grey: `--jw-white-3`, `-4`, `-5`, `-6`, `-7`, `-8`, `-9`, `-10`, `-12`, `-14`, `-15`, `-20`.

| Step | Use (report section 2.2 and 2.3) |
| --- | --- |
| 3 % | rows, the quietest surface; terminal and tab bars of code blocks |
| 4 % | cards (`--jw-card-surface`), the hero card, code blocks, asides |
| 6 % | the hamburger button, the docs search field |
| 7 % | inline code |
| 8 % | secondary buttons, the footer rule, rules between rows, the current docs page in the rail |
| 10 % | the header button; hamburger hover |
| 12 % | secondary button hover |
| 15 % | header button hover, blockquote rule, note and tip aside rule |
| 20 % | the underline of footer links; code selection |

A rule between sections or rows is `1px solid var(--jw-white-8)`. There is no other border colour (rule 19).

### Actions

| Token | Value | Use |
| --- | --- | --- |
| `--jw-primary` | `#ffb454` | primary button background, `::selection` |
| `--jw-primary-hover` | `#ffc26b` | primary button hover, link hover |
| `--jw-on-primary` | `#1a1200` | text on the primary button and on accent fills: 10.5:1, 11.7:1 on hover |
| `--jw-accent` | `#ffb454` | focus rings, pills, bar fills, links, the h1's last clause: 10.8:1 on the page |
| `--jw-accent-45` | accent at 45 % | the glow of a pill or a live dot (`0 0 12px -2px`) and nothing else |
| `--jw-accent-light`, `--jw-accent-deep` | `#ffe3b0`, `#ff8a3d` | the first and last stops of the gradient only |
| `--jw-accent-gradient` | `linear-gradient(90deg, #ffe3b0, #ffb454 55%, #ff8a3d)` | `background-clip: text` on one span per heading |

Where the accent may appear: focus rings, pills, bar fills, links, and the last clause of a page h1 or the one number of a section h2 (rules 8, 9, 24). Nowhere else: not on an ordinary heading, a docs heading, a footer heading, a label, body text or a button label.

The gradient is text only: `background: var(--jw-accent-gradient); background-clip: text; color: transparent`, on a `<span>` holding the last clause. Never a background fill, never a border.

### Result tiers and warnings

| Token | Value | On page | Use |
| --- | --- | --- | --- |
| `--jw-tier-green` | `#8be0b4` | 12.2:1 | a green result (the board's green count, a green badge), success text |
| `--jw-tier-amber` | `#f2c46b` | 11.7:1 | an amber result, warnings, caution asides |
| `--jw-tier-red` | `#ff8a8a` | 8.4:1 | a red result, errors, danger asides |

Reserved: never decorative, never a heading accent, never a brand colour. The brand amber (`#ffb454`) and the tier amber (`#f2c46b`) are different colours and are never swapped. A mod with no harness result yet is neutral (`--jw-white-50`), never a colour. Jarrow's tiers (A, B, unsupported) are a classification, not a result, so a tier label is neutral text (`--jw-white-80`) and the words say which tier it is. Badges (`badge_color` in `tokens.rs`) are the tier colours with `--jw-page` text, or the page colour with `--jw-fg` text when nothing has run.

### Scenes only

`--jw-sky-0` (`#0b0f1f`, the top of the sky) to `--jw-sky-14` (`#ffd27a`, the horizon), and `--jw-sun-0` (`#fff8dc`, the top of the disc) to `--jw-sun-6` (`#ff7a3d`). `--jw-sky-fallback` is the CSS sky behind the canvas before its first frame and without JavaScript: stops 0, 2, 5 and 9 at 0, 38, 62 and 80 %, then the page colour at 86 %, the same stops the reference takes from its ramp. These colours are for the canvas generators and that fallback; no interface element uses them.

## How the colour check works

`site/scripts/check-colors.mjs` is the last step of `npm run build`, so CI runs it on every pull request. It reads every CSS and HTML file in `dist/`, and the compiled markup of the server-rendered routes, and finds every colour: hex, `rgb()`, `hsl()`, `color-mix()`, other colour functions and named colours. Each one must be a token value or an alpha step `tokens.css` declares (as a literal, or as `color-mix(in oklab, var(--jw-white) 60%, transparent)`, which is what Tailwind's `text-white/60` compiles to). `transparent`, `currentColor` and `inherit` are not colours of their own. The browser theme colour must be `--jw-page` itself.

Two exceptions, both proven, never assumed: vendor defaults (Starlight, Pagefind) that `site/src/styles/global.css` replaces with an unlayered token mapping on `:root`; and Pagefind's default-UI stylesheets, which no page loads. A `PENDING` entry in the script names a colour that comes from a file the change may not edit yet; the check fails once the entry matches nothing, so it is removed with the fix.

If the check fails, the fix is never a new literal: use the token; if no token fits, the design does not have that colour.

## Tailwind

`global.css` removes Tailwind's palette (`--color-*: initial`) and maps the tokens: `bg-page`, `text-fg`, `text-white`, `text-white/60` (compiles to a declared alpha step), `bg-primary`, `hover:bg-primary-hover`, `text-on-primary`, `text-accent`, `text-tier-green`. Only the alpha steps listed above pass the check: `bg-white/4` passes, `bg-white/37` fails the build.
