# Footer

Measured: `docs/design-research/horizon.md`, section 1 (the shared footer), section 2.1 (footer type), section 5 (tells 11 and 12), section 7 (the Mojang and Microsoft marks, and why Jarrow does not copy their treatment). Rule 27. Screenshot: `home-1440-full.png`, the bottom 670 px.

## Anatomy

Above it, one rule: `border-top: 1px solid var(--jw-white-8)`. Inside the container:

1. The wordmark, and the disclaimer line in `--jw-white-80`, 15 px, weight 500: "Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft." with Mojang's trademark notice.
2. Three paragraphs in sentences, each under a 14 px weight 500 heading in `--jw-white-80`:
   - **Who makes it:** who makes Jarrow and how to reach them, with links (the owner's personal name appears only in `LICENSE`; the footer names the project and its maintainers' public handles).
   - **Licences:** the code's MIT licence and the rule tables' CC0 licence with links; the fonts (Bricolage Grotesque, Geist, Geist Mono, SIL Open Font License); "every third-party mark on this site is listed in BRAND-SOURCES.md" with the link.
   - **Privacy:** what the page loads and from where, naming every hostname the page calls (none on `/`), and a link to `/privacy`.
3. A last line: the repository path as a link, and the Rust mark beside "Written in Rust" if its row exists in `site/BRAND-SOURCES.md`.

## Tokens

| Part | Value |
| --- | --- |
| Paragraphs | 14 px (`--jw-label-size`), line height 1.625, `--jw-white-50` (5.3:1) |
| Headings | 14 px, weight 500, line height 20 px, `--jw-white-80` |
| Links | `--jw-white-70`, underline `--jw-white-20` offset 4 px; hover `--jw-white` with the underline at full white, over 150 ms |
| Spacing | on the 4 px grid; compare with the bottom of `home-1440-full.png` and `home-390-full.png` |
| Layout | four columns at 1440 (wordmark and disclaimer, then the three paragraphs); one column at 390 |

## Rules

- Sentences, not link lists: no "Product / Company / Resources / Legal" columns (report section 5, tell 11).
- No "© 2026 Jarrow. All rights reserved." (tell 12).
- Outbound links carry the visually hidden "(opens in a new tab)".
- Third-party marks follow `brand.md`: the official mark in a colour its guideline permits, or the name in text. Never the reference's 40 %-white Microsoft and Mojang marks.
