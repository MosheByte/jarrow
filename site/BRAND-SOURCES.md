# Brand and icon sources

Every brand, icon, logo and typeface on the site, where it comes from, and the terms it is used under. Jarrow's rule: no generated icons, logos, illustrations or screenshots, and no mark that the brand's own terms do not allow.

The evidence behind each entry (what was opened, at which commit, on which date, and the questions still open with each project) is in [docs/brand-sources-research.md](../docs/brand-sources-research.md). The design reference is section 7 of [docs/design-research/horizon.md](../docs/design-research/horizon.md).

## The rule

- A brand is shown with its logo only when the artwork's licence and the brand's published terms both allow a site that is not affiliated with it to show the mark. The logo is the brand's own SVG from its own kit, inline, unmodified except where the terms allow, `aria-hidden`, 14 to 20 px, with the brand's name as text beside it. Colour is what the guideline permits and nothing else.
- Every other brand is its name, set as text in the site's own type.
- No placeholder icon, generic glyph, drawn lookalike, Simple Icons copy or emoji stands in for a brand. Lucide icons are for UI actions only and never sit next to a brand name.
- Pages render brands through `src/brands/BrandMark.astro`, which reads `src/brands/brands.ts`. That list and the Brands table below must agree; `npm test` fails when they do not.

`npm test` runs `scripts/brand-check.mjs` over `src/` and `public/`. It fails on:

| Rule | What it catches |
| --- | --- |
| `svg-without-source` | an `<svg>` (or an SVG data URI) with neither `data-brand-source` nor `data-icon="lucide"` |
| `svg-source-unrecorded` | a `data-brand-source` that is not a literal URL listed in the Mark source column below for a logo brand |
| `svg-brand-not-hidden` | a brand `<svg>` without `aria-hidden="true"` |
| `lucide-unmarked` | a Lucide icon component used without `data-icon="lucide"` |
| `lucide-brand-glyph` | a Lucide icon named after a brand (an old Lucide brand glyph) |
| `icon-near-brand` | a Lucide icon within 40 characters of a name in the Names matched column |
| `emoji-near-brand` | an emoji within 40 characters of a name in the Names matched column |
| `simple-icons` | an import of the `simple-icons` package or the Iconify `simple-icons:` set |

Offenders in files that the check's own pull request could not edit are listed in `scripts/brand-check-baseline.json`. They are allowed up to the listed count and only that; the list shrinks as those files are fixed. `node scripts/brand-check.mjs --strict` ignores it.

To add a logo: record the brand's guideline and terms here first, add a component to `src/brands/` with the official SVG and a literal `data-brand-source` equal to its Mark source, switch the brand to `logo` in `brands.ts`, and add a credit line there if the licence asks for one (`BrandCredits.astro` renders it).

## Brands

The check reads this table. Shown as is `logo`, `text` or `not used`; Mark source is the official kit page that the SVG's `data-brand-source` names; Names matched are matched case-sensitively as whole words.

| Brand | Shown as | Mark source | Names matched |
| --- | --- | --- | --- |
| Minecraft | text | — | Minecraft |
| Mojang Studios | text | — | Mojang |
| Microsoft | text | — | Microsoft |
| Windows | text | — | Windows |
| winget | text | — | winget |
| Fabric | text | — | Fabric, FabricMC |
| NeoForge | text | — | NeoForge, NeoForged |
| Forge | text | — | Forge, MinecraftForge |
| Quilt | text | — | Quilt, QuiltMC |
| Modrinth | text | — | Modrinth |
| CurseForge | text | — | CurseForge |
| GitHub | logo | https://primer.style/octicons/icon/mark-github-16/ | GitHub |
| GitHub Sponsors | text | — | GitHub Sponsors |
| GitHub Actions | text | — | GitHub Actions |
| CodeQL | text | — | CodeQL |
| Dependabot | text | — | Dependabot |
| GitLab | text | — | GitLab |
| Rust | logo | https://github.com/rust-lang/rust-artwork/tree/main/logo | Rust |
| Cargo | text | — | Cargo |
| crates.io | text | — | crates.io |
| WebAssembly | logo | https://github.com/carlosbaraza/web-assembly-logo | WebAssembly, Wasm |
| npm | text | — | npm |
| Homebrew | text | — | Homebrew |
| Scoop | text | — | Scoop |
| AUR | text | — | AUR, Arch Linux |
| Linux | text | — | Linux |
| macOS | text | — | macOS, Apple |
| Gradle | text | — | Gradle |
| Sigstore | text | — | Sigstore, Cosign |
| OpenSSF | text | — | OpenSSF, Scorecard |
| Cloudflare | text | — | Cloudflare |
| Anthropic | text | — | Anthropic |
| Claude | text | — | Claude |
| Codecov | text | — | Codecov |
| Sentry | text | — | Sentry |
| PostHog | text | — | PostHog |
| Better Stack | text | — | Better Stack |
| UptimeRobot | text | — | UptimeRobot |
| Sinytra Connector | text | — | Sinytra |
| Forgified Fabric API | text | — | Forgified Fabric API |
| Architectury | text | — | Architectury |
| Discord | not used | — | Discord |

## Sources and terms

Checked on 2026-11-03. "Not opened" marks a URL taken from the design report's starter table that was not reopened for this list.

| Brand | Official page | Guideline or terms | Artwork licence | Shown as, and the constraints |
| --- | --- | --- | --- | --- |
| Minecraft | https://www.minecraft.net/ | https://www.minecraft.net/en-us/usage-guidelines | Mojang's; no logo download for third parties | Text. Nothing may suggest the site is official, approved or associated; no part of the brand in Jarrow's own branding. Every page carries the disclaimer "Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft." |
| Mojang Studios | https://www.minecraft.net/ | same as Minecraft | Mojang's | Text. Never the Mojang mark, tinted or not (one of Horizon's four lapses). |
| Microsoft | https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks (not opened) | https://cdn-dynmedia-1.microsoft.com/is/content/microsoftcorp/microsoft/mscle/documents/legal/intellectualproperty/trademarks/Microsoft_logo_third_party_usage_guidance_.pdf (not opened) | Microsoft's | Text. Logo use needs a licence; the logo is never single-colour. |
| Windows | same as Microsoft | https://cdn-dynmedia-1.microsoft.com/is/content/microsoftcorp/microsoft/mscle/documents/presentations/Windows_Trademark_guidelines_2023.pdf (not opened) | Microsoft's | Text. Any Windows logo needs a trademark licence; never a drawn four-square glyph. |
| winget | https://learn.microsoft.com/en-us/windows/package-manager/ (not opened) | Microsoft trademark page above | Microsoft's | Text. |
| Fabric | https://fabricmc.net/ | none published | The official logo by unascribed is CC0 1.0, in https://github.com/FabricMC/community/tree/main/media/unascribed, as a 500×500 PNG only | Text. CC0 waives copyright but licenses no trademark rights, there is no usage guideline, and there is no SVG master. Question open with FabricMC. |
| NeoForge | https://neoforged.net/ | none published | Official art by Ridanisaurus, CC BY 4.0, in https://github.com/neoforged/.github/tree/main/art, PNG only | Text. CC BY 4.0 licenses no trademark rights, there is no usage guideline, and there is no SVG master. Question open with NeoForged. |
| Forge | https://minecraftforge.net/ | none published | Not stated; the repository's LGPL-2.1 licence names no logo | Text. Question open with the Forge team. |
| Quilt | https://quiltmc.org/ | none beyond the file licence | `brand/` in https://github.com/QuiltMC/art is CC0 1.0, with SVGs | Text. CC0 waives copyright only, and its text says it licenses no trademark rights. That is a file licence, not a guideline for showing the Quilt logo next to another project. Question open with QuiltMC. |
| Modrinth | https://modrinth.com/ | https://modrinth.com/legal/terms (Trademarks section); `COPYING.md` in https://github.com/modrinth/code | All rights reserved, Rinth, Inc. | Text. The marks need Rinth's prior written permission, and branding is prohibited without it. The name is used only to say where data comes from. |
| CurseForge | https://www.curseforge.com/ | none found; https://www.overwolf.com/brand-guidelines/ covers the Overwolf logo only | not stated | Text. Question open with CurseForge support. |
| GitHub | https://brand.github.com/foundations/logo | https://docs.github.com/en/site-policy/other-site-policies/github-logo-policy and the "Do these things" list on the brand page | Octicons: MIT, copyright GitHub, Inc. (https://github.com/primer/octicons) | Logo: Octicon `mark-github`, 16 px grid, `src/brands/GitHub.astro`. Allowed to link to GitHub, as a social button for the repository, and to show integration, always less prominent than Jarrow's own name and never implying affiliation. White, black, or grey in a few cases; never the amber accent, no effects, no distortion. Clear space 25% of the mark. |
| GitHub Sponsors | https://github.com/sponsors | GitHub logo policy above | — | Text. GitHub's brand toolkit has no separate Sponsors mark; the Octicon heart is a UI glyph and never stands in for it. |
| GitHub Actions | https://github.com/features/actions | GitHub logo policy above | — | Text. |
| CodeQL | https://codeql.github.com/ | GitHub logo policy above | — | Text. |
| Dependabot | https://github.com/dependabot | GitHub logo policy above | — | Text. |
| GitLab | https://about.gitlab.com/ | https://handbook.gitlab.com/handbook/marketing/brand-experience/trademark-guidelines/ | GitLab's | Text. The guidelines permit the logos only for a narrow purpose and generally refuse requests for websites. |
| Rust | https://www.rust-lang.org/ | https://rustfoundation.org/policy/rust-trademark-policy/ | CC BY 4.0, owned by the Rust Foundation (https://github.com/rust-lang/rust-artwork/tree/main/logo) | Logo: `rust-logo-single-path.svg`, `src/brands/Rust.astro`. Non-commercial use that states software is written in Rust needs no permission; nothing may look official or endorsed. CC BY asks for credit, a licence link and a note of changes: the fill is the text colour instead of black, and `BrandCredits.astro` says so on every page that shows it. |
| Cargo | https://doc.rust-lang.org/cargo/ | Rust trademark policy above | CC BY 4.0 (the Cargo logo) | Text. |
| crates.io | https://crates.io/ | https://crates.io/policies (not opened); Rust trademark policy above | — | Text. |
| WebAssembly | https://webassembly.org/ | https://github.com/WebAssembly/design/issues/980 (the Community Group's vote for the official logo, released under CC0) | CC0 1.0, by Carlos Baraza (https://github.com/carlosbaraza/web-assembly-logo) | Logo: the icon, white variant, `src/brands/WebAssembly.astro`. Official colourways are purple `#654ff0`, white and black; on the dark page it is white, unmodified. |
| npm | https://www.npmjs.com/ | https://docs.npmjs.com/policies/logos-and-usage | npm's | Text. The logos signify npm and need npm's permission. |
| Homebrew | https://brew.sh/ | none found | Code BSD-2-Clause, documentation CC BY 4.0; the logo's terms are not stated | Text. |
| Scoop | https://scoop.sh/ | none found | not stated | Text. |
| AUR | https://aur.archlinux.org/ | https://terms.archlinux.org/docs/trademark-policy/ | Arch Linux's | Text. |
| Linux | https://www.linuxfoundation.org/legal/the-linux-mark (not opened) | same | Tux is Larry Ewing's, not the Linux Foundation's | Text. |
| macOS | https://www.apple.com/macos/ | https://www.apple.com/legal/intellectual-property/guidelinesfor3rdparties.html (not opened) | Apple's | Text. No Apple logo without a written licence; word marks only in referential phrases. |
| Gradle | https://gradle.org/ | https://develocity.ai/brand/ and https://develocity.ai/legal/gradle-technologies-trademark-usage-guidelines/ | Gradle, Inc.'s | Text. The guidelines apply to parties with a licence or permission. |
| Sigstore | https://www.sigstore.dev/ | https://www.linuxfoundation.org/legal/trademark-usage | Files Apache-2.0 in https://github.com/sigstore/community/tree/main/artwork, with a brand guide | Text. Linux Foundation project logos on websites need written permission. |
| OpenSSF | https://openssf.org/ | https://www.linuxfoundation.org/legal/trademark-usage | Files Apache-2.0 in https://github.com/ossf/artwork | Text. Same as Sigstore. |
| Cloudflare | https://www.cloudflare.com/ | https://www.cloudflare.com/trademark/ | Cloudflare's | Text. Logos need written permission; customers may use the unaltered web badges from https://www.cloudflare.com/press-kit/. |
| Anthropic | https://www.anthropic.com/ | https://www.anthropic.com/legal/trademark-guidelines | Anthropic's | Text. Trademarks only as Anthropic permits, in materials it approves first. |
| Claude | https://claude.com/ | same as Anthropic | Anthropic's | Text. |
| Codecov | https://about.codecov.io/ | https://about.codecov.io/brand/ (logo download; no third-party terms found) | Codecov's | Text. |
| Sentry | https://sentry.io/ | https://sentry.io/branding/ (logo download; no third-party terms read) | Sentry's | Text. |
| PostHog | https://posthog.com/ | https://posthog.com/handbook/brand/assets | PostHog's | Text, as no page shows it. Its terms allow the logo unmodified, not as Jarrow's own branding, without implied endorsement, so it can become a logo if a page needs one. |
| Better Stack | https://betterstack.com/ | https://betterstack.com/docs/brand/ (logo download; asks to email when unsure) | Better Stack's | Text. |
| UptimeRobot | https://uptimerobot.com/ | none found | not stated | Text. |
| Sinytra Connector | https://github.com/Sinytra/Connector | none found | Code MIT | Text. |
| Forgified Fabric API | https://github.com/Sinytra/ForgifiedFabricAPI | none found | Code Apache-2.0 | Text. |
| Architectury | https://github.com/architectury | none found | not stated | Text. |
| Discord | https://discord.com/branding (not opened) | same | Discord's | Not used: Jarrow links to no Discord server. If it ever does, white on blurple only. |

## Loader logos: none yet

The site shows no logo of any of the four loaders (Fabric, NeoForge, Forge, Quilt), and no Minecraft, Mojang or Microsoft logo. The loaders are named in plain text only (text wordmarks).

Why:

- The project allows a loader logo only under that loader's published brand guidelines. On 2026-11-03 the four projects' repositories and sites had no such guideline. What they do have:
  - Fabric's official logo is CC0 1.0, as a PNG only.
  - NeoForge's official art is CC BY 4.0, as PNGs only.
  - Forge states no terms for its logo.
  - Quilt publishes its branding in [QuiltMC/art](https://github.com/QuiltMC/art), and the `brand/` folder there is under CC0 1.0, with SVGs.
- CC0 and CC BY are file licences. Both say in their text that they license no trademark rights, so neither is a guideline for showing a loader's logo next to another project. Each loader gets the same text wordmark.
- A loader logo next to "Jarrow" could suggest an affiliation that does not exist. The footer notice on every page says plainly that the project is not an official Minecraft product and is not affiliated with Mojang, Microsoft or the loader projects ([docs/legal.md](../docs/legal.md#trademarks)).

The question for each project is written out in [docs/brand-sources-research.md](../docs/brand-sources-research.md#questions-to-ask). If one of the four projects publishes guidelines that allow this use, or answers yes, add the logo here first, with a link to the guideline or the answer, then on the site.

## Icons

| What | Source | License | Where it is used |
| --- | --- | --- | --- |
| UI icons (arrows, book, map, sun, moon, construction, calendar, external link) | [Lucide](https://lucide.dev) via the `@lucide/astro` package | [ISC](https://lucide.dev/license) | Buttons, notices and the theme toggle. UI actions only; each carries `data-icon="lucide"` and none sits within 40 characters of a brand name |
| Favicon | Lucide [`arrow-right-left`](https://lucide.dev/icons/arrow-right-left), paths unmodified, stroke width 2.5 for small sizes, coloured with `--jw-text` (`public/favicon.svg`) | ISC | Browser tab. On the brand-check baseline until the Jarrow mark replaces it |
| GitHub mark | GitHub's Octicon [`mark-github`](https://primer.style/octicons/icon/mark-github-16/), from [primer/octicons](https://github.com/primer/octicons) 19.42.0, in `src/brands/GitHub.astro` | [MIT](https://github.com/primer/octicons/blob/main/LICENSE); GitHub's logo policy governs the mark | Links to the repository. The older Simple Icons copy in `src/components/GitHubMark.astro` (header, docs header, home page button) is on the brand-check baseline until those files switch to `BrandMark` |
| Rust logo | [rust-lang/rust-artwork](https://github.com/rust-lang/rust-artwork/tree/main/logo), `rust-logo-single-path.svg`, in `src/brands/Rust.astro` | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), with credit | Nowhere yet |
| WebAssembly logo | [carlosbaraza/web-assembly-logo](https://github.com/carlosbaraza/web-assembly-logo), white icon, in `src/brands/WebAssembly.astro` | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Nowhere yet |
| Docs interface icons (search, menu, theme select, chevrons) | Built into [Starlight](https://starlight.astro.build) | MIT | Docs pages only. They are part of the docs framework, not drawn for this project |
| Deploy-key icon | Lucide [`key`](https://lucide.dev/icons/key), unmodified, from lucide 1.67.0: `docs/assets/key.svg` for the README and docs/team.md, `@lucide/astro` on the site; Lucide marks this icon as derived from Feather | [ISC](https://lucide.dev/license) (Feather part: MIT) | Avatar of the jarrow-bot deploy key in the README, docs/team.md and the /about page |

Simple Icons is not a source for this site. Its package disclaims any right to the marks it redraws, so a brand's own kit and terms always decide.

## The Jarrow wordmark

"Jarrow" is set as text in the display typeface. There is no logo image.

## Typefaces

Self-hosted through [Fontsource](https://fontsource.org); no font is loaded from a third-party server.

| Typeface | Package | License | Use |
| --- | --- | --- | --- |
| Bricolage Grotesque (variable) | `@fontsource-variable/bricolage-grotesque` | SIL Open Font License 1.1 | Headings, the wordmark, large numbers |
| Atkinson Hyperlegible Next (variable) | `@fontsource-variable/atkinson-hyperlegible-next` | SIL Open Font License 1.1 | Body text |
| JetBrains Mono (variable) | `@fontsource-variable/jetbrains-mono` | SIL Open Font License 1.1 | Code, labels |

## Third-party images

Two kinds, both from GitHub:

- The GitHub Actions status badge for `ci.yml` on the home page, loaded from github.com. It is the repository's own CI status.
- The avatars in "Who works on Jarrow" on the /about page (and in the README and docs/team.md), hot-linked from avatars.githubusercontent.com: each GitHub account's or GitHub App's own avatar, as GitHub serves it. The agents' avatar is the avatar of their maker's GitHub organization. The build fails if `site/src/data/team.json` names an avatar on any other host. The deploy key has no avatar and gets the Lucide icon above.

Everything else on the site is text and CSS.

Mod icons on the board, once it has rows, will be hot-linked from each mod's Modrinth page with attribution and a link back, never copied into the repository.
