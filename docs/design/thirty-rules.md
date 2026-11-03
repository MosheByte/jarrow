# The thirty rules

The design rules of jarrow.pages.dev. They are the thirty rules of the measured reference, [`docs/design-research/horizon.md`](../design-research/horizon.md) section 11, with Jarrow's four substitutions applied: the night-blue page `#0b0f1f`, the amber accent and primary button `#ffb454`, the sunrise in place of the sunset, and the amber sky and sun ramps. Everything else is the reference's measured value. Where Jarrow's bar is stricter than the reference's, the stricter value is written (rule 22: the brand's own colours instead of `currentColor`, which the report lists as a lapse; rule 30: 350 KiB instead of 450 KiB).

Each rule ends with how it is checked. **automated** names the check in [`site/tests/rules/`](../../site/tests/rules/) that fails the `rules` or `lighthouse` job of [`site-quality.yml`](../../.github/workflows/site-quality.yml) (`rendered rule-NN` is in `rendered.test.mjs`, `static css-*` and `static token-pairs-*` in `static.test.mjs`). **review** names what only the design reviewer judges, from the screenshots. How the checks run, and the markup they rely on, is in [`docs/site-quality.md`](../site-quality.md).

1. The page background is `#0b0f1f` and `color-scheme` is dark; there is no light mode. — automated: `rendered rule-01`, `static css-no-light-mode`
2. Headings and display numbers are set in Bricolage Grotesque at weight 600; everything else is Geist; code is Geist Mono. — automated: `rendered rule-02`
3. The page h1 is 60 px at 1440 and 36 px at 390, line height 1.05 or less, letter spacing −0.035 em. — automated: `rendered rule-03`
4. Section h2s are 48 px at 1440 and 32 px at 390, line height 1.08, letter spacing −0.03 em. — automated: `rendered rule-04`
5. Body text is 15 to 18 px Geist at line height 1.625; docs body is 16 px at 1.7. — automated: `rendered rule-05`
6. No text element uses `text-transform: uppercase`; eyebrows are 14 px, weight 500, sentence case, white at 60 %. — automated: `rendered rule-06`, `static css-no-uppercase`; review: eyebrow size, weight and colour
7. Text colour steps are white at 100, 80, 70, 65, 60 and 50 % alpha; nothing below 50 % carries information. — automated: `rendered rule-07`
8. The accent `#ffb454` appears on focus rings, pills, bar fills, links and the last words of a page h1; the primary button is `#ffb454` with `#1a1200` text and turns `#ffc26b` on hover; the tier amber `#f2c46b` never stands in for it. — automated: `rendered rule-08`; review: where the accent appears (focus rings, pills, bars, links)
9. Gradient text (`#ffe3b0 → #ffb454 55 % → #ff8a3d`) is applied to at most one span per heading, and never to body text, labels or buttons. — automated: `rendered rule-09`
10. Every length is a multiple of 4 px; the content column is 1152 px wide with 40 px gutters at 1024 px and wider, and 24 px below. — automated: `rendered rule-10`
11. The header is 72 px, fixed, transparent at scroll 0, and `#0b0f1f` at 75 % with a 12 px backdrop blur after scrolling. — automated: `rendered rule-11`
12. Cards have no border and no drop shadow; their surface is white at 3 to 8 % over the page colour, radius 16 px, padding 24 to 32 px. — automated: `rendered rule-12`
13. Buttons are 48 px (hero), 40 px (page) or 36 px (header) tall, radius 10 or 8 px, 15 or 14 px type at weight 500, with 17 to 18 px icons and a 10 px gap. — automated: `rendered rule-13`
14. Numbers that change or align use `tabular-nums`; units follow the number in 14 px at 50 % white. — automated: `rendered rule-14`
15. Every illustration is drawn into a canvas at runtime on a pixel grid of `clamp(round(viewport width / 340), 3, 6)` px and scaled with `image-rendering: pixelated`. — automated: `rendered rule-15`
16. Scene canvases are `aria-hidden`, fade in over 700 ms, run at 32 fps or less only while on screen, and draw a single static frame under `prefers-reduced-motion: reduce`. — automated: `rendered rule-16`
17. The sunrise sits in the bottom 230 px of the hero at 1440 and in a 253 px strip under the content at 390; no copy overlaps the sun or the ridges. — automated: `rendered rule-17`; review: no copy over the sun or the ridges
18. No raster is used for decoration; every `<img>` is a game screenshot with a full-sentence alt, a credited product photo, an avatar or a partner's wordmark. — automated: `rendered rule-18`, `static css-no-raster-decoration`; review: what each image is and its credit
19. Section separators are the dithered edges of a scene canvas or a 1 px rule at 8 % white; there are no SVG wave masks and no gradient bridges between sections. — automated: `rendered rule-19`; review: gradient bridges between sections
20. Entrance motion is one keyframe (`translateY(14px) → 0`, opacity 0 → 1) over 0.9 s with `cubic-bezier(0.2, 0.7, 0.2, 1)`, staggered by `--delay`, and bound to the view timeline below the fold. — automated: `rendered rule-20`, `static css-one-entrance-keyframe`
21. Hover changes colour alpha over 150 ms; the only hover transforms are a 3 % image scale and a 4 px caption rise, both under `motion-safe`. — automated: `rendered rule-21`, `static css-hover-colour-only`
22. Every brand mark is the brand's official inline SVG with `data-brand-source`, `aria-hidden`, with the brand's name as adjacent text, in the colours and at the size its guideline permits, and recorded in `site/BRAND-SOURCES.md`. — automated: `rendered rule-22`; review: that the mark is the official one, at a permitted size and colour
23. Every text link that leaves the site carries a visually hidden "(opens in a new tab)"; in body copy it also carries a 14 to 16 px arrow glyph. — automated: `rendered rule-23`
24. Each page h1 is two lines with the last clause on its own line in the accent gradient; content h2s and docs headings are white. — automated: `rendered rule-24`
25. Hero copy states a version number and a count; the lede is 25 words or fewer and contains at least two figures. — automated: `rendered rule-25`; review: that the figures are real and current
26. Social proof is a counted list of real people with links; there are no testimonials and no logo rows. — review
27. The footer is three paragraphs in sentences (who makes it, licences with links, privacy with the hostnames the page calls) plus the Mojang and Microsoft disclaimer. — automated: `rendered rule-27`; review: what the paragraphs say
28. Empty, loading and error states are sentences that say what is missing and since when ("No samples in this range yet: the history began on 31 October 2026."). — review
29. Body text at 50 % white or darker is never smaller than 12 px, and every text/background pair measures 4.9:1 or more. — automated: `rendered rule-29`, `static token-pairs-contrast`
30. Pages render their full text, navigation and footer with JavaScript disabled, and the mobile transfer of the home page stays at 350 KiB or less with CLS 0. — automated: `rendered rule-30`, `lighthouse`

## Checklist for a pull request that touches `site/`

Copy this into the pull request and tick only what the change verifiably meets; the `rules` job and the design review check the same list.

```md
- [ ] 1 Night-blue page, dark only
- [ ] 2 Bricolage 600 headings, Geist body, Geist Mono code
- [ ] 3 h1 60 / 36 px, line height 1.05 or less, -0.035 em
- [ ] 4 h2 48 / 32 px, 1.08, -0.03 em
- [ ] 5 Body 15 to 18 px at 1.625; docs 16 px at 1.7
- [ ] 6 No uppercase; eyebrows 14 px, 500, white 60 %
- [ ] 7 White text steps 100 to 50 %, nothing informative below 50 %
- [ ] 8 Accent and primary #ffb454, hover #ffc26b, text #1a1200
- [ ] 9 One gradient span per heading at most
- [ ] 10 4 px lengths, 1152 px column, 40 / 24 px gutters
- [ ] 11 Header 72 px, fixed, transparent, then 75 % with 12 px blur
- [ ] 12 Cards: no border, no shadow, white 3 to 8 %, 16 px radius, 24 to 32 px padding
- [ ] 13 Buttons 48 / 40 / 36 px, radius 10 / 8, 15 / 14 px at 500, 17 to 18 px icons, 10 px gap
- [ ] 14 tabular-nums; units 14 px at 50 %
- [ ] 15 Canvas scenes on the clamp(round(w / 340), 3, 6) px grid, pixelated
- [ ] 16 Scenes aria-hidden, 700 ms fade, 32 fps or less on screen only, static under reduced motion
- [ ] 17 Sunrise in the bottom 230 px at 1440, 253 px strip at 390, no copy over it
- [ ] 18 No decorative raster; every img accounted for
- [ ] 19 Edges are the scene's falloff or a 1 px 8 % rule
- [ ] 20 One entrance keyframe, 0.9 s, view timeline below the fold
- [ ] 21 Hover changes colour over 150 ms only
- [ ] 22 Official brand marks with data-brand-source, recorded in BRAND-SOURCES.md
- [ ] 23 "(opens in a new tab)" on outbound text links
- [ ] 24 Two-line h1 with the accent last clause; other headings white
- [ ] 25 Hero: version number and count; lede 25 words or fewer, two figures
- [ ] 26 Social proof is real people, counted and linked
- [ ] 27 Footer in three paragraphs plus the disclaimer
- [ ] 28 Empty and error states say what is missing and since when
- [ ] 29 Nothing under 12 px at 50 % or darker; every pair 4.9:1 or more
- [ ] 30 Works without JavaScript; / is 350 KiB or less on mobile, CLS 0
```
