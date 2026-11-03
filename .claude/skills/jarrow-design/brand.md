# Brand: marks, the wordmark, voice

Measurements and sources: `docs/design-research/horizon.md`, section 7 (every third-party mark on the reference, how it is drawn, and the starter table of official brand pages), section 4 (why the reference reads as a Minecraft site without Minecraft's assets) and section 6 (voice). The register of every mark Jarrow shows: `site/BRAND-SOURCES.md`.

## Third-party marks

1. **Official or nothing.** A product, platform or technology named on the landing page, in the docs or in a link row is shown with its official logo from its own brand page or press kit, or as its name in text. Never a placeholder icon, a generic glyph, a drawn lookalike or an emoji in its place. A generic icon set's glyph for a brand counts as a lookalike.
2. **Recorded first.** Before a mark reaches a page, its row exists in `site/BRAND-SOURCES.md`: the brand, the source URL of the asset, the guideline URL, the licence or permission, the size and colour allowed. A brand whose guideline forbids logo use is listed with that fact, and the site shows its name as text in the site's own type.
3. **Inline SVG, marked.** Each mark is inline SVG with `data-brand-source="<the URL in BRAND-SOURCES.md>"`, `aria-hidden="true"`, 14 to 20 px, with the brand's name as adjacent text (rule 22). A CI check fails the build on an `<svg>` without `data-brand-source` and on an icon-set glyph or emoji near a brand name.
4. **The colour its guideline allows.** Draw the mark in the colourway the brand permits: Discord's Clyde in white on its own blurple (`#5865f2`, hover `#4752c4`, report section 7), GitHub's Octicon mark, Rust's gear under its CC-BY licence, Prism Launcher's mark under CC BY-SA with the attribution beside it. A mark is never recoloured to a tint its guideline forbids, and a brand colour is a brand's, never a Jarrow token used decoratively. Where a guideline allows a monochrome mark, `currentColor` at the surrounding text's colour is the default.
5. **Not verified means text.** Brands whose official source the starter table marks "not verified" (Modrinth, Fabric, NeoForge, Forge, CurseForge, GitHub Sponsors) are shown as their name in text until their row is resolved from the project's own repository or site, or by asking in their issues or Discord.
6. **The reference's four lapses, never repeated:** marks tinted to 40 % white against their guideline (Microsoft, Mojang), a self-drawn Windows mark, a dollar glyph for GitHub Sponsors, and raster Mojang art without a stated source.
7. **Minecraft.** No Mojang texture, block, character or key art. The footer says "Not an official Minecraft product." and names Mojang's trademark, as the usage guidelines require (`https://www.minecraft.net/en-us/usage-guidelines`).

## The Jarrow wordmark

"Jarrow" is set as text in Bricolage Grotesque 600, white, beside the logo if there is one. A logo, when drawn, is built on the scene's pixel grid (whole cells, `shape-rendering="crispEdges"`), in white and the accent, the way the reference's mark is a 16×16 grid "H" whose crossbar is its horizon (report section 4). No gradient, no rounded generic letter, no cube icon.

## How Jarrow reads as a Minecraft tool without Minecraft's assets

The mechanisms in report section 4, in Jarrow's colours: the scanlined pixel sun; stepped clouds; block-stepped ridges; Bayer-dithered gradients as the only "noise"; the 4 px grid and pixelated canvases; numbers set in the display face with their units; real screenshots from real runs as evidence, captioned with what produced them; Minecraft's own vocabulary in copy (loader, mod, JAR, mappings, mixin, world, server). Never a pixel font for text.

## Voice

Report section 6 is the style guide. In short:

- **Concrete nouns.** JAR, loader, mapping, harness, rule table, report, board. Not "solution", "experience", "platform".
- **Figures.** "1,000 mods on the board", "3 loaders", "4.1 MiB". Thousands separators, units after a space, dates written out ("3 November 2026").
- **Plain negatives.** "Not built yet (milestone M3)." "Not an official Minecraft product." "A mod that ships native libraries does not convert, and the report names them." A limitation is stated as plainly as a capability.
- **No promise words.** None of the marketing adjectives the report counts zero of (its "Register" paragraph), and not the outcome-promise word `AGENTS.md` bans; say "verified by the harness" and link the report.
- **Links say where they go.** "The board's red mods", "every rule table on GitHub"; never "Learn more" or "Click here". An outbound text link ends with a visually hidden "(opens in a new tab)".
- **States are sentences.** Say what is missing and since when: "No harness results yet: the first nightly run has not finished." "Reload to try again." Never "No data" or "Oops".
- **Sentence case** everywhere, a full stop on a heading that is a sentence, no uppercase.
