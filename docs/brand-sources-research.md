# Brand sources: research log

The evidence behind [site/BRAND-SOURCES.md](../site/BRAND-SOURCES.md): for each brand the site names, what was opened, at which commit, and what it allows. Researched on 2026-11-03 through the GitHub API (`gh api`) and Firecrawl (search and scrape). Nothing was posted or sent anywhere; the questions at the end are for the owner to ask in each project's own channel.

The design reference is section 7 of [docs/design-research/horizon.md](design-research/horizon.md), whose starter table left six entries "not verified": Modrinth, Fabric, NeoForge, Forge, CurseForge and GitHub Sponsors. All six are resolved below. None of them may show a logo yet.

## How a brand is classified

A brand is `logo` only when both hold:

1. The artwork's licence allows the copy (MIT, CC BY, CC0, or a kit published for third parties).
2. The brand's published terms allow a site that is not affiliated with it to show the mark, or the mark's owner released it for free use as the official logo.

A file licence alone is not enough. CC0 1.0 (section 4a) and CC BY 4.0 (section 2b2) both say in their text that they license no trademark rights, so "the logo file is CC0" answers the copyright question and leaves the trademark one open. A brand whose official logo exists only as a raster is text until there is an official SVG: the site does not trace, redraw or convert marks.

Everything else is `text`, in the site's own type.

## The six "not verified" entries

### Modrinth: text

- `COPYING.md` in [modrinth/code](https://github.com/modrinth/code) at commit `6d1d56516463a738b6ef49810885fe54614f8e72` (2026-10-11), section "Modrinth Branding": the wrench-in-labyrinth logo and other branding are all rights reserved by Rinth, Inc., any use needs its explicit written permission, and forks must remove them.
- The Terms of Use, `apps/frontend/src/pages/legal/terms.vue` in the same repository at commit `755825b09ac6f88a38c0727d51896f94a338b65a` (2026-08-13), section "Trademarks": the Modrinth name and logo are the company's trademarks and need its prior written permission. Served at https://modrinth.com/legal/terms.
- The modrinth organisation's 21 public repositories hold no brand kit or press page.
- Result: text. The name is used referentially, to say that board data comes from the Modrinth API.

### Fabric: text

- [FabricMC/community](https://github.com/FabricMC/community), `media/README.md` and `media/unascribed/README.md` at commit `5ed1043590b715d3afdb7799bb3eb911485c8b80` (2020-11-28): the official Fabric logo, created by unascribed in 2016, is under CC0 1.0. The files are `png/fabric.png` (500×500 px, 2,205 bytes, pixel art on a 16-cell grid) and an `.xcf` source. There is no SVG.
- [FabricMC/fabricmc.net](https://github.com/FabricMC/fabricmc.net) (code MIT, site content CC BY-NC-SA 4.0): `assets/logo.png` is 52×56 px. Its only SVGs are icons for CurseForge, GitHub and Modrinth.
- A code search of the 62 FabricMC repositories for "trademark", "logo usage" and "brand" found licence files and the fabricmc.net download buttons, but no usage guideline for the Fabric logo.
- Result: text. Two things block a logo: no trademark terms, and no vector master. A pixel-exact SVG conversion of the PNG is technically possible but is not an official SVG, so it waits for Fabric's answer.

### NeoForge: text

- `art/README.md` in [neoforged/.github](https://github.com/neoforged/.github/tree/main/art) at commit `7d4e38307737cfb7de594a44bae5a48c3208d078` (2024-04-28): these are the official art assets of NeoForged, including the logo and wordmark, created by Ridanisaurus, copyright the NeoForged team, under CC BY 4.0. The files are PNGs (`neoforge_logo.png`, `neoforge_logo_pixel.png`, `neoforged_logo.png`, `neoforged_icon_16x16.png`) and Paint.NET sources. There is no SVG.
- [neoforged/websites](https://github.com/neoforged/websites) (content CC BY 4.0) uses `assets/img/content/branding/banner.png` as the site logo. Its 2023 post `content/news/theproject.md` explains the fork from Forge and says the team would develop its own brand because some core assets are licensed by Forge LLC.
- A code search of the 66 neoforged repositories (for "trademark", "logo" and "brand") and the neoforged.net sources found no usage guideline.
- Result: text, for the same two reasons as Fabric.

### Forge: text

- [MinecraftForge/MinecraftForge](https://github.com/MinecraftForge/MinecraftForge), default branch `26.3`: `LICENSE.txt` covers the code (LGPL-2.1) and data from MCP; it says nothing about the logo. The repository root holds `forge_installer_logo.png` and `icon.ico` with no terms.
- https://minecraftforge.net/ redirects to the forums; no brand, press or trademark page was found by search.
- Result: text.

### CurseForge: text

- https://www.overwolf.com/brand-guidelines/ is Overwolf's page and covers only the Overwolf logo (download SVG or PNG; do not edit, distort or recolour).
- https://www.curseforge.com/terms returned 404 on 2026-11-03. Search found no CurseForge brand page or logo kit; the `curseforge` GitHub organisation has no public repositories.
- Result: text.

### GitHub Sponsors: text

- GitHub's brand toolkit (https://brand.github.com/foundations/logo) and its 2026 guidelines describe the Invertocat, the wordmark and product lockups; there is no separate GitHub Sponsors mark.
- The Octicons `heart` and `sponsor-tiers` glyphs are UI icons in GitHub's product, not a Sponsors logo.
- Result: text. Never a heart or a dollar glyph in its place (one of Horizon's four lapses).

## Quilt: text

- [QuiltMC/art](https://github.com/QuiltMC/art) at commit `849d6dfd9ba639895e15c7b200e06b3bb50e9c2e`: the repository is CC0 1.0 unless a folder says otherwise; `brand/` is Quilt's official branding, mostly by Prospector, with `brand/LICENSE` CC0 1.0 and SVGs, including `brand/svg/quilt_mini_mono_white_transparent.svg` (last changed at `0ddcb7046d3835f884b05ff169083029b793d02f`, 2021-04-20) and "Available for Quilt" and "Powered by Quilt" badges.
- The quiltmc.org footer says the logos on that site belong to their owners and that Quilt is not an official Minecraft product; it gives no guideline for others showing the Quilt logo.
- Result: text. CC0 covers the files, not the trademark, which is also how PR #16 reads it. The badges suggest Quilt expects third parties to use the mark, so this is the most likely of the four loaders to become a logo once asked.

## The three logos the site may show

### GitHub: logo (Octicon `mark-github`)

- https://docs.github.com/en/site-policy/other-site-policies/github-logo-policy points to the brand toolkit. https://brand.github.com/foundations/logo lists what is allowed: link to GitHub with a permitted logo, use the Invertocat as a social button for a project, show an integration, and keep the GitHub logo less prominent than your own name. It forbids implying affiliation, using the mark as your own logo, and changing its colour or shape. The mark appears in white, black, or in a few cases grey or green. The 2026 guidelines PDF asks for clear space of 25%.
- The Octicon is GitHub's own icon of the mark. [primer/octicons](https://github.com/primer/octicons): MIT, copyright 2026 GitHub Inc. Tag `@primer/octicons@19.42.0` (published 2026-10-23) is commit `9a274e212ee25c400ff666ce25a57866b7eacc25`; `icons/mark-github-16.svg` there is blob `0e55a8ef2db1913cb35f81ae09ffdb58b5422319`, SHA-256 `7421820090b50ac79d7c2bf7c951a433d7098a8a9c474809207bdd45f62d9d46`. Kit page: https://primer.style/octicons/icon/mark-github-16/.
- `site/src/brands/GitHub.astro` copies the path unmodified, fills it with the text colour, and carries the MIT notice.

### Rust: logo

- https://rustfoundation.org/policy/rust-trademark-policy/ (published 2025-01-23, modified 2025-06-25): the Rust and Cargo logos are owned by the Rust Foundation and distributed under CC BY 4.0, and their use is also governed by the trademark policy. Stating accurately that software is written in Rust needs no approval, commercial or not; nothing may appear official, affiliated or endorsed.
- `logo/README.md` in [rust-lang/rust-artwork](https://github.com/rust-lang/rust-artwork/tree/main/logo) at commit `6e11af7aa21a0022b813fa477330a74d5c4e47e2`: CC BY requires credit, a licence link and a note of changes; most non-commercial uses of the names and logos need no permission; a white-outline variant exists for dark backgrounds.
- File: `logo/rust-logo-single-path.svg` at repository commit `7b54b6689dc310db2f301d7bcda847f016bd447c` (file last changed at `012085d9f2a37dc0aa82094c8d8a2cb707364081`), SHA-256 `840941e53f1f3ae8acbdbf41df54f02f8da55ce6bdb08ce68c9ceb11263fe3b3`.
- `site/src/brands/Rust.astro` copies viewBox, transform and path unmodified. The one change is the fill: the text colour instead of black. `BrandCredits.astro` states it next to the CC BY credit on every page that shows the mark.

### WebAssembly: logo

- [WebAssembly/design#980](https://github.com/WebAssembly/design/issues/980), "WebAssembly logo voting", closed 2017-02-21: the Community Group's vote for the official logo, to be released under CC0 with the author's approval; Carlos Baraza's design won.
- [carlosbaraza/web-assembly-logo](https://github.com/carlosbaraza/web-assembly-logo): `LICENSE` is CC0 1.0. `dist/icon/web-assembly-icon-white.svg` at repository commit `f0f411529c1dafffa233be1bd95b80b79144b675` (file last changed at `e59d423d45699f9531e71fa858a7efbe0af81734`, 2020-06-10), SHA-256 `285a1e4032d3964c013f073293efce2e6bac9994885cac530da0e6619cc6e988`.
- webassembly.org uses the same mark in purple `#654ff0` (`css/webassembly.svg` in [WebAssembly/website](https://github.com/WebAssembly/website), commit `2b6d598c9c61130927e730260ff80b2f1cadfcc9`). No trademark policy for the mark was found; the Community Group chose a public-domain logo so that anyone may use it.
- `site/src/brands/WebAssembly.astro` keeps the white variant's geometry and colour; only the markup changed (see the component's header).

## The other brands: text

| Brand | Opened | Finding |
| --- | --- | --- |
| Minecraft, Mojang Studios | https://www.minecraft.net/en-us/usage-guidelines | Nothing may look official, approved or associated; no part of the brand in your own branding; a prominent disclaimer of the form "Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft." is required. Jarrow writes it in sentence case. |
| Microsoft, Windows, winget | The design report's starter table (Microsoft's third-party logo guidance and the Windows trademark guidelines) | A licence is needed for any Microsoft or Windows logo. Not reopened. |
| GitHub Actions, CodeQL, Dependabot | GitHub's logo policy above | GitHub products; named as text. Their GitHub App avatars appear as avatars, not as marks. |
| GitLab | https://handbook.gitlab.com/handbook/marketing/brand-experience/trademark-guidelines/ (revised 2024-07-03) | Logos permitted only for one narrow purpose; requests to use them on websites are generally refused. |
| Cargo, crates.io | Rust trademark policy above | Rust trademarks; no reason to show the Cargo logo. |
| npm | https://docs.npmjs.com/policies/logos-and-usage | The logos signify npm and need its permission, considered case by case. |
| Homebrew | README of [Homebrew/brew](https://github.com/Homebrew/brew); [Homebrew/brew.sh](https://github.com/Homebrew/brew.sh) | Code BSD-2-Clause, docs CC BY 4.0; no terms for the logo. |
| Scoop | [ScoopInstaller](https://github.com/ScoopInstaller) repositories, https://scoop.sh | No brand page. |
| AUR | https://terms.archlinux.org/docs/trademark-policy/ (version 2021-04-18) | Name use that states intent for Arch is allowed; questions go to trademarks@archlinux.org. |
| Linux, macOS | The design report's starter table | Text only; Apple allows no logo without a written licence. Not reopened. |
| Gradle | https://develocity.ai/brand/ | The Gradle marks' guidelines apply to parties with a licence or permission. |
| Sigstore, OpenSSF | https://www.linuxfoundation.org/legal/trademark-usage; [ossf/artwork](https://github.com/ossf/artwork) README; [sigstore/community](https://github.com/sigstore/community/tree/main/artwork) | Artwork files are Apache-2.0, but Linux Foundation marks on websites need written permission. |
| Cloudflare | https://www.cloudflare.com/trademark/ | Logos need written permission; customers may use the unaltered web badges from the press kit. |
| Anthropic, Claude | https://www.anthropic.com/legal/trademark-guidelines; https://code.claude.com/docs/en/legal-and-compliance | Trademarks only as Anthropic permits, in materials it approves; other uses need written permission. |
| Codecov | https://about.codecov.io/brand/ | Logo download; no third-party terms found. |
| Sentry | https://sentry.io/branding/ | Logo download and a brand guide; no third-party terms read. |
| PostHog | https://posthog.com/handbook/brand/assets | Third parties may use the logo unmodified, not as their own branding, without implied endorsement. Allowed, but no page shows PostHog yet. |
| Better Stack | https://betterstack.com/docs/brand/ | Logo download; asks to email hello@betterstack.com when unsure. |
| UptimeRobot | Search for a brand or press page | None found. |
| Sinytra Connector, Forgified Fabric API, Architectury | [Sinytra/Connector](https://github.com/Sinytra/Connector) (MIT), [Sinytra/ForgifiedFabricAPI](https://github.com/Sinytra/ForgifiedFabricAPI) (Apache-2.0), [architectury](https://github.com/architectury) | Named only to say Jarrow copies none of their code; no brand pages. |
| Discord | The design report's starter table | Not used: the site links to no Discord server. |

## Questions to ask

Each question is written to be pasted as is. They go to the project's own channel, and the answer, with a link, goes into site/BRAND-SOURCES.md before any logo goes on the site.

### Fabric

Where: the Fabric Discord (https://discord.gg/v6v4pMv, linked from fabricmc.net/discuss), or an issue in https://github.com/FabricMC/community.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow) is an open-source, non-commercial tool that converts mods between loaders. It is not affiliated with FabricMC. Its website names the loaders it supports, and we would like to show the official Fabric logo (media/unascribed in FabricMC/community, CC0) at 16 to 20 px beside the word "Fabric", with a note on every page that Jarrow is not affiliated with the Fabric project. Is that use fine with you? And is there a vector (SVG) master of the logo, or would you accept an exact pixel-for-pixel SVG conversion of png/fabric.png?

### NeoForge

Where: the NeoForged Discord (https://discord.neoforged.net/), or an issue in https://github.com/neoforged/websites.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow) is an open-source, non-commercial tool that converts mods between loaders. It is not affiliated with NeoForged. Its website names the loaders it supports, and we would like to show the NeoForge logo from neoforged/.github/art (CC BY 4.0, by Ridanisaurus) at 16 to 20 px beside the word "NeoForge", credited to Ridanisaurus and the NeoForged team, with a note on every page that Jarrow is not affiliated with the NeoForge project. Is that use fine with you? Is there an SVG master of the logo or of the fox mark we should use instead of the PNGs?

### Forge

Where: the Forge Discord (https://discord.minecraftforge.net/), or an issue in https://github.com/MinecraftForge/MinecraftForge.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow) is an open-source, non-commercial tool that converts mods between loaders. It is not affiliated with the Minecraft Forge project. Its website names the loaders it supports. Under what terms, if any, may a site like ours show the Minecraft Forge logo beside the word "Forge"? Is there an official SVG and a usage guideline we should follow?

### Quilt

Where: the Quilt Discord (https://discord.quiltmc.org/), or an issue in https://github.com/QuiltMC/art.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow) is an open-source, non-commercial tool that converts mods between loaders. It is not affiliated with the Quilt Project. The brand/ folder in QuiltMC/art is CC0, which covers the files but, as CC0 itself says, not the trademark. May our website show the Quilt mark (brand/svg/quilt_mini_mono_white_transparent.svg, unmodified) at 16 to 20 px beside the word "Quilt", where it names the loaders it supports, with a note on every page that Jarrow is not affiliated with the Quilt Project? Are there colour or clear-space rules we should follow?

### Modrinth

Where: written permission from Rinth, Inc., as COPYING.md in modrinth/code asks: support@modrinth.com.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow) is an open-source, non-commercial tool that converts mods between loaders. It is not affiliated with Modrinth. Its board lists mods it reads from the Modrinth API, and says so in text. Would Rinth, Inc. give written permission to show the Modrinth logo at 16 to 20 px beside the word "Modrinth" where the site names its data source? If yes, which file and which colours should we use?

### CurseForge

Where: a ticket at https://support.curseforge.com/.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow) is an open-source, non-commercial tool that converts mods between loaders. It is not affiliated with CurseForge or Overwolf. Does CurseForge publish brand guidelines or a logo kit? May a site like ours show the CurseForge logo beside the word "CurseForge" where it names where mods are published, and if so, which file and which colours?

### GitHub Sponsors

Where: GitHub Support (https://support.github.com/), or the contact link on https://brand.github.com/.

> Hello! For a "Sponsor" link to https://github.com/sponsors/<account> on a third-party, non-affiliated project site, is there an official GitHub Sponsors mark, or should the link use the GitHub mark (the Octicon mark-github) with the words "GitHub Sponsors"?

### Rust (a confirmation, not a blocker)

Where: contact@rustfoundation.org, or trademark@rust-lang.org as logo/README.md in rust-artwork suggests.

> Hello! Jarrow (https://github.com/jarrow-dev/jarrow), a non-commercial open-source project written in Rust, shows the Rust logo (rust-logo-single-path.svg, unmodified geometry) at 16 px beside the words "Written in Rust" on its dark website. It is filled with the page's white text colour instead of black, and the page credits the Rust Foundation under CC BY 4.0 and states that change. Is a single-colour white rendering acceptable, or should we use rust-logo-white-outline.svg on dark backgrounds?
