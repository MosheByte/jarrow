---
name: jarrow-design
description: Jarrow's design system for everything under site/ (pages, components, CSS, canvas scenes, brand marks, site copy). The measured playhorizon.dev system with Jarrow's four substitutions - night-blue page, amber accent, a sunrise, amber sky and sun ramps. Use it before writing, changing or reviewing anything a visitor sees. Every rule names a token in site/tokens.css or a measurement in docs/design-research/horizon.md.
---

# Jarrow design system

Jarrow's site is built on one measured reference: playhorizon.dev, analysed in `docs/design-research/horizon.md` (tokens read from its stylesheet, its canvas generators read line by line, Lighthouse, contrast, 176 screenshots in `docs/design-research/horizon/`, measurement dumps in `docs/design-research/horizon/data/`). The reference draws a violet sunset over the sea; Jarrow draws an amber sunrise behind a mountain ridge, with the same type, the same grid, the same restraint and the same way of meeting the interface. Jarrow follows the report's thirty rules (section 11) exactly, with four substitutions.

This file is the entry point. The values live in `site/tokens.css`; the rules that use them live in the files listed under "Files".

## Read first, in this order

1. `docs/design-research/horizon.md`, in full. Sections 2 (tokens), 3 (illustration), 6 (copy), 7 (logos), 8 (motion), 9 (performance), 10 (accessibility) and 11 (the thirty rules) are the ones every change touches.
2. If your working copy has the private build specification (`BUILD-BOOK.md` at the repository root; it is never committed), read its section 8, the site's design direction. Never quote it in a tracked file. Its four substitutions are restated, as values, in `colors.md`.
3. `site/tokens.css`. Generated from `crates/report/src/tokens.rs`; never edit it by hand.
4. The file in this skill for what you are building (see "Files").

## How to use this skill

- Claude Code loads it from `.claude/skills/jarrow-design/` by itself. Other tools (Codex, Cursor, Copilot, any agent that takes rules files): add this directory as context or rules. It is plain Markdown, laid out the way TypeUI design skills are (https://www.typeui.sh/design-skills): foundations (colour, type, spacing), components with their states, accessibility, principles and anti-patterns, tone. Its content is Jarrow's own.
- Before you write: open the component file, use only the tokens it names, and keep its states (default, hover, focus-visible, active, disabled, loading, error, empty).
- Values come from tokens. Write `var(--jw-white-60)`, not `rgb(255 255 255 / 0.6)` and never a new hex. A value the tokens lack goes into `crates/report/src/tokens.rs` first, with the report section it comes from, then `JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current`.
- After you write: `cd site && npm test && npm run check && npm run build`. The build ends with `scripts/check-colors.mjs`, which fails on any colour in `dist/` that is not a token or a declared alpha step.
- Every pull request that touches `site/` carries a 1440×900 and a 390×844 screenshot of each changed page and the checklist at the end of this file, with only the rules the change verifiably meets ticked. A reviewer that has not seen the pull request compares the screenshots with `docs/design-research/horizon/<page>-<width>-first.png` and lists every rule broken. Nothing merges with an open rule violation.

## The four substitutions

Everything not in this table is the report's value, verbatim (section 2).

| What | Reference (measured) | Jarrow | Token |
| --- | --- | --- | --- |
| Page | `#100a1f`, dark only | `#0b0f1f` night blue, dark only | `--jw-page` |
| Accent and primary button | `#a98fff`; button `#6b4bff`, hover `#7a5dff`, white text | `#ffb454`; button `#ffb454`, hover `#ffc26b`, text `#1a1200` | `--jw-accent`, `--jw-primary`, `--jw-primary-hover`, `--jw-on-primary` |
| Scene | a sunset into the sea | a sunrise behind a mountain ridge | `components/` and `motion.md` |
| Sky and sun ramps | violet to orange | night blue to amber, 15 and 7 stops | `--jw-sky-0` to `--jw-sky-14`, `--jw-sun-0` to `--jw-sun-6` |

The body text (`--jw-fg`, `#f4f3ff`) and the heading gradient (`--jw-accent-gradient`) follow the accent. The tier colours (`--jw-tier-green`, `--jw-tier-amber`, `--jw-tier-red`) are the report's own and are reserved for result tiers and warnings.

## Principles

1. **Tokens, not values.** Every colour, size, length, radius, duration and easing is a `--jw-*` custom property. `check-colors.mjs` enforces it for colour; review enforces the rest.
2. **Dark only.** `color-scheme: dark`, no light token set, no `prefers-color-scheme` query, no theme switch (rule 1).
3. **Surfaces by alpha, never by edge.** A card is white at 3 to 8 % over the page (`--jw-white-4` for the hero card), radius 16 px, padding 24 or 32 px; no border, no shadow (rule 12).
4. **One accent, used for five things.** Focus rings, pills, bar fills, links, and the last clause of a page h1 (rules 8, 9, 24). Never an ordinary heading, never body text, never a decoration.
5. **Green, amber and red mean a result or a warning.** Nothing is green without a harness result behind it. Tier A, Tier B and unsupported are a classification, so their labels are neutral text (`--jw-white-80`).
6. **The pictures are code.** Every illustration is a seeded canvas generator on a pixel grid; no raster decoration, no SVG wave dividers (rules 15 to 19).
7. **Measured, not guessed.** A number in the interface is a figure with its unit and thousands separator, in `tabular-nums` (rule 14). A claim on the page is a fact with a source.

## Anti-patterns (each is a rule failure)

From the report's section 5 ("Tells of generated design") and the four lapses of section 7:

- A three-column feature grid with icons in tinted circles; icons on every list item.
- Gradient text on more than one span per heading, or on body text, labels or buttons.
- Uppercase anywhere (`text-transform: uppercase`), tracked eyebrows, Title Case nav or buttons.
- Blurred colour orbs, mesh gradients, noise textures; any gradient other than `--jw-accent-gradient` and the scene's own sky.
- Cards with 1 px borders and drop shadows; hover that lifts or shadows a card.
- Buttons with gradient fills or glows.
- "Trusted by" logo rows, testimonials, invented metrics ("10k+ users").
- "Learn more" links; links that do not say where they go.
- A pricing table, a cookie banner, a newsletter modal, a chat widget, an auto-playing video.
- "Oops! Page not found"; "No data"; error toasts in red boxes.
- A placeholder icon, a generic glyph, a drawn lookalike or an emoji standing in for a brand; a brand mark tinted to a colour its guideline forbids (the reference tinted the Microsoft and Mojang marks to 40 % white, drew its own Windows squares, and used a dollar glyph for GitHub Sponsors: never repeat those).
- Any colour that is not a token. Any length off the 4 px grid.

## Accessibility

Testable, and it wins: if a rule here would lower contrast, hide focus or block the keyboard, the accessibility requirement holds and the conflict is named in the pull request.

- Every text and background pair measures at least 4.9:1, composited the way the report measures it (rule 29; `MIN_TEXT_CONTRAST` in `tokens.rs`, tested for every text step on the page and on the card). Text at 50 % white or darker is never under 12 px.
- Focus is visible: `outline: 2px solid var(--jw-accent); outline-offset: 2px` (report section 10).
- Canvases are `aria-hidden="true"`; game screenshots carry full-sentence alt text; brand marks are `aria-hidden` beside the brand's name as text.
- Every text link that leaves the site carries a visually hidden "(opens in a new tab)" (rule 23).
- `prefers-reduced-motion: reduce` turns off every keyframe and every canvas loop (`motion.md`).
- Every page renders its full text, navigation and footer without JavaScript (rule 30).
- Lighthouse runs in CI on `/`, `/board` and one docs page: Performance at least 95 on mobile and 98 on desktop, Accessibility 100, CLS 0. A score below blocks the merge.

## Writing tone

Report section 6 is the style guide; `brand.md` has the details.

- Sentence case everywhere, including the nav and buttons. A heading that is a sentence ends with a full stop; a label does not.
- Figures with units and thousands separators: "1,000 mods", "4.1 MiB", "1.21.1".
- None of the marketing adjectives the report counts zero of (its "Register" paragraph lists them), and not the outcome-promise word `AGENTS.md` bans. Write "verified by the harness" with a link instead.
- Negative facts as plainly as positive ones: "Not an official Minecraft product." "Not built yet (milestone M3)."
- Empty and error states say what is missing and since when.

## Files

| File | What it decides |
| --- | --- |
| `colors.md` | Every colour token, where it may appear, contrast, how the colour check works |
| `typography.md` | Families, the type scale, numbers, fallbacks, copy casing |
| `layout.md` | The 4 px grid, the container, the header, the docs rails, radii, surfaces |
| `motion.md` | The one entrance keyframe, hover, the scene canvases, reduced motion |
| `brand.md` | Brand marks and `site/BRAND-SOURCES.md`, the wordmark, voice |
| `components/button.md` | Hero, page, header, white and pill buttons |
| `components/card.md` | Cards, the hero card, panels |
| `components/range-input.md` | The board card's range input and its bars |
| `components/header.md` | The 72 px header and the nav |
| `components/footer.md` | The three-paragraph footer |
| `components/docs-layout.md` | The 208 / 600 / 176 px docs layout |

## The thirty rules, with Jarrow's substitutions

Copy this list into every pull request that touches `site/` and tick only what the change verifiably meets. Numbers match the report's section 11.

- [ ] 1. The page background is `#0b0f1f` (`--jw-page`) and `color-scheme` is dark; there is no light mode.
- [ ] 2. Headings and display numbers are Bricolage Grotesque at weight 600; everything else is Geist; code is Geist Mono.
- [ ] 3. The page h1 is 60 px at 1440 and 36 px at 390, line height 1.02 (at most 1.05), letter spacing −0.035 em.
- [ ] 4. Section h2s are 48 px at 1440 and 32 px at 390, line height 1.08, letter spacing −0.03 em.
- [ ] 5. Body text is 15 to 18 px Geist at line height 1.625; docs body is 16 px at 1.7.
- [ ] 6. No text uses `text-transform: uppercase`; eyebrows are 14 px, weight 500, sentence case, white at 60 %.
- [ ] 7. Text steps are white at 100, 80, 72 (docs body), 70, 65, 60 and 50 %; nothing below 50 % carries information.
- [ ] 8. The accent `#ffb454` appears on focus rings, pills, bar fills, links and the last words of a page h1; the primary button is `#ffb454`, hover `#ffc26b`, text `#1a1200`.
- [ ] 9. Gradient text (`#ffe3b0` → `#ffb454` 55 % → `#ff8a3d`) is on at most one span per heading, never on body text, labels or buttons.
- [ ] 10. Every length is a multiple of 4 px; the container is 1152 px with 40 px gutters from 1024 px and 24 px below.
- [ ] 11. The header is 72 px, fixed, transparent at scroll 0, and `#0b0f1f` at 75 % with a 12 px backdrop blur after scrolling.
- [ ] 12. Cards have no border and no shadow; their surface is white at 3 to 8 % over the page, radius 16 px, padding 24 to 32 px.
- [ ] 13. Buttons are 48 px (hero), 40 px (page), 36 px (header) tall, radius 10 or 8 px, 15 or 14 px at weight 500, with 17 to 18 px icons and a 10 px gap.
- [ ] 14. Numbers that change or align use `tabular-nums`; units follow the number in 14 px at 50 % white.
- [ ] 15. Every illustration is drawn into a canvas at runtime on a grid of `clamp(round(width / 340), 3, 6)` px, scaled with `image-rendering: pixelated`.
- [ ] 16. Scene canvases are `aria-hidden`, fade in over 700 ms, run at 32 fps or less only while on screen, and draw one static frame under reduced motion.
- [ ] 17. The sunrise sits in the bottom 230 px of the hero at 1440 and in a 253 px strip under the content at 390; no copy overlaps the sun or the ridges.
- [ ] 18. No raster is used for decoration; every `<img>` is a game screenshot with a full-sentence alt, a credited photo, an avatar or a partner's wordmark.
- [ ] 19. Section separators are the dithered edge of a scene canvas or a 1 px rule at 8 % white; no SVG wave masks, no gradient bridges.
- [ ] 20. Entrance motion is one keyframe (`translateY(14px)` → 0, opacity 0 → 1) over 0.9 s with `cubic-bezier(0.2, 0.7, 0.2, 1)`, staggered by `--delay`, bound to the view timeline below the fold.
- [ ] 21. Hover changes colour alpha over 150 ms; the only hover transforms are a 3 % image scale and a 4 px caption rise, both under `motion-safe`.
- [ ] 22. Every brand mark is the brand's official inline SVG with `data-brand-source`, `aria-hidden`, 14 to 20 px, beside the brand's name as text, in a colour its guideline permits, recorded in `site/BRAND-SOURCES.md`; a brand without a usable mark is its name in text.
- [ ] 23. Every text link that leaves the site carries a visually hidden "(opens in a new tab)"; in body copy also a 14 to 16 px arrow.
- [ ] 24. Each page h1 is two lines with the last clause on its own line in the accent gradient; content h2s and docs headings are white.
- [ ] 25. Hero copy states a version number and a count; the lede is at most 25 words with at least two figures.
- [ ] 26. Social proof is a counted list of real people with links; no testimonials, no logo rows.
- [ ] 27. The footer is three paragraphs in sentences (who makes it, licences with links, privacy with the hostnames the page calls) plus the Mojang and Microsoft disclaimer.
- [ ] 28. Empty, loading and error states are sentences that say what is missing and since when.
- [ ] 29. Text at 50 % white or darker is never under 12 px, and every text and background pair measures at least 4.9:1.
- [ ] 30. Pages render their full text, navigation and footer without JavaScript; the home page's mobile transfer stays under 450 KiB with CLS 0.
