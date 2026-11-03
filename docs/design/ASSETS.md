# Visual assets: what is missing

Inventory of 2026-11-03, taken from `main` at a09934b plus the open pull requests #22 (brand marks) and #33 (README). It lists every visual asset the product needs that does not exist yet or exists only as a placeholder. Nothing here has been drawn.

Third-party logos are out of scope: they come only from each brand's official page, as recorded in `site/BRAND-SOURCES.md` (#22). Colours are the design tokens: night blue `#0b0f1f`, foreground `#f4f3ff`, brand amber `#ffb454`, and the result trio mint `#8be0b4`, tier amber `#f2c46b` and red `#ff8a8a`, which is reserved for tiers and results.

The last column says whether the scene generators in `site/src/scenes/` can produce the asset. Rows marked "yes" are not for the artist and are listed under [Generated in code](#generated-in-code).

## For the artist

### (a) Logo and wordmark

| ID | Name | Where | Role | Size (px) | Format | Background | States | Content (10 words) | Scene generator |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| asset-01 | Logo mark | Nowhere yet: `site/public/favicon.svg:1` holds a Lucide arrow placeholder, and `site/src/components/Header.astro:22` is text only | The symbol that stands for Jarrow where the name does not fit. | 16×16 grid master, exported at 16, 32, 48, 64 | SVG, `shape-rendering="crispEdges"` | Transparent | `#ffb454` on `#0b0f1f`; `#1a1200` on white (amber on white is too faint) | Original 16-grid mark; no blocks, pickaxes or loader shapes. | No |
| asset-02 | Lockup (mark and name, horizontal) | `site/src/components/Header.astro:22-23` (the name as text with an "M0" tag) | The site header and every place the full name appears with the mark. | Height 32 (header), exports at 64 and 128; width follows the mark | SVG | Transparent | Dark: name `#f4f3ff`, mark `#ffb454`. Light: name `#0b0f1f`, mark `#1a1200` | Mark left of "Jarrow" in Bricolage Grotesque 600; fixed gap. | No |

### (b) Favicon and app icons

| ID | Name | Where | Role | Size (px) | Format | Background | States | Content (10 words) | Scene generator |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| asset-03 | Favicon | `site/public/favicon.svg:1` (Lucide "arrow-right-left" placeholder), `site/astro.config.mjs:45`, `site/src/layouts/Base.astro:39` | Browser tab and bookmarks. | 16×16 viewBox | SVG | Transparent | Dark and light through `prefers-color-scheme`, as the current file does | asset-01 only, hand-tuned at 16 px; no text. | No |
| asset-04 | Favicon fallback | Missing: `site/src/layouts/Base.astro:39` links the SVG only | Browsers and crawlers that ignore SVG icons. | 16, 32 and 48 in one file | ICO | Transparent | None | Same pixels as asset-03; no resampling. | No |
| asset-05 | Apple touch icon | Missing from the `<head>` in `site/src/layouts/Base.astro` | iOS and iPadOS home screen. | 180×180 | PNG | `#0b0f1f` (iOS fills transparency with black) | None | asset-01 at 10× (160 px), centred. | No |
| asset-06 | Web app icons | Missing: there is no `manifest.webmanifest` | Android home screen, needed only if a manifest is added. | 192×192, 512×512, maskable 512×512 with the mark inside the central 80% | PNG | `#0b0f1f` | Regular and maskable | asset-01 at a whole-number scale, centred; no text. | No |
| asset-07 | GitHub organization avatar | https://github.com/jarrow-dev (a 3,208-byte PNG, likely GitHub's default identicon; unverified) | The organization's face on GitHub, on the repository page and in its default link card. | 500×500 | PNG | `#0b0f1f` | None | asset-01 at 18× (288 px), centred on night blue. | No |
| asset-08 | Rust API docs logo | Missing: no `#![doc(html_logo_url = …, html_favicon_url = …)]` in `crates/*/src/lib.rs`; the docs are at https://jarrow-dev.github.io/jarrow/ | rustdoc sidebar logo and tab icon. | Logo 128×128; favicon is asset-03 | SVG | Transparent | rustdoc's dark and light themes | asset-01 only; nothing Rust-branded. | No |
| asset-09 | GitHub OAuth app logo | `infra/INVENTORY.md:65` (site sign-in, M4) | GitHub's authorize screen for Jarrow. | 512×512 | PNG | `#0b0f1f` | None | asset-01 centred; no GitHub mark beside it. | No |
| asset-10 | Status page logo | `infra/INVENTORY.md:48` (Better Stack status page, M5; none exists yet) | Header of the public status page. | asset-02 at 64 px height, 2× PNG | PNG | Transparent | Dark | asset-02 as it is; nothing added. | No |

### (e) Original UI icons

Every icon in this group sits on the same 16 px grid, in one ink at a time, and is shown only at whole-number scales (16, 32, 48).

Two notes:

- **No loader icons.** Fabric, NeoForge, Forge and Quilt appear as text: their brands publish no usage guideline (`site/BRAND-SOURCES.md`, #22), and a drawn stand-in fails the brand check.
- **Lucide comes first.** `AGENTS.md` says UI icons come from Lucide, so replacing a Lucide icon with one from this set needs a `D-` entry first.

| ID | Name | Where | Role | Size (px) | Format | Background | States | Content (10 words) | Scene generator |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| asset-11 | Agent | README "Agents" heading (#33, `README.md:39`: draft `site/public/icons/pixel/agent.svg`, shown at 24 px, which is not a whole-number scale); `site/src/components/TeamGrid.astro:169` and `docs/team.md:23` show the agent's GitHub avatar | Marks the AI agent group and its members. | 16×16 | SVG | Transparent | On `#0b0f1f` and on white | Original agent glyph; no Anthropic, Claude or robot-face clichés. | No |
| asset-12 | Bot | README "Bots and automation" heading (#33, `README.md:45`: draft `site/public/icons/pixel/bot.svg`); `site/src/components/TeamGrid.astro:164-172` (GitHub App avatars, and a Lucide key for the deploy key) | Marks bots and automation. | 16×16 | SVG | Transparent | On `#0b0f1f` and on white | Generic automation glyph; no GitHub, Dependabot or Renovate shapes. | No |
| asset-13 | Tier A | `site/src/pages/board.astro:84`, `site/src/pages/index.astro:66`, `site/src/pages/convert.astro:32` (text pill `tier-a` today) | Converts in the browser; the JAR stays on the machine. | 16×16 | SVG | Transparent | Mint `#8be0b4` | Readable by shape alone; mint only; never decorative. | No |
| asset-14 | Tier B | `site/src/pages/board.astro:84`, `site/src/pages/index.astro:66`, `site/src/pages/convert.astro:37` | Deep conversion in a short-lived CI job. | 16×16 | SVG | Transparent | Tier amber `#f2c46b`, never brand amber | Readable by shape alone; distinct from Tier A. | No |
| asset-15 | Unsupported | `site/src/pages/board.astro:84`, `site/src/pages/index.astro:67`, `site/src/pages/convert.astro:42` | Refused, with a named reason. | 16×16 | SVG | Transparent | Red `#ff8a8a` | Refusal, not danger; no skull, no hazard sign. | No |
| asset-16 | Tier unknown | `site/src/lib/board.ts:30` (the `unknown` tier) | Not classified yet. | 16×16 | SVG | Transparent | Foreground at 50%, no result colour | Neutral; must not resemble any tier or result. | No |
| asset-17 | Result green | `site/src/pages/board.astro:89` (text label), `site/src/pages/index.astro:99` (CSS dot) | The harness found the converted mod behaves the same. | 16×16 | SVG | Transparent | Mint `#8be0b4` | Shown only with a harness result; check-like shape. | No |
| asset-18 | Result yellow | `site/src/pages/board.astro:89` | Loads, with minor differences (score 0.5). | 16×16 | SVG | Transparent | Tier amber `#f2c46b` | Partial match; differs from green by shape. | No |
| asset-19 | Result red | `site/src/pages/board.astro:89` | Material differences, or the mod failed to load. | 16×16 | SVG | Transparent | Red `#ff8a8a` | Failure; must differ from the Unsupported icon. | No |
| asset-20 | Result not run | `site/src/pages/board.astro:89`, `site/src/pages/index.astro:99,108` (CSS dot `result-not-run`) | No harness run yet for this cell. | 16×16 | SVG | Transparent | Foreground at 50% | Empty, not failed; no result colour. | No |
| asset-21 | Board empty | `site/src/pages/board.astro:100-108` ("No rows yet") | The first nightly has not run. | 32×32 (the 16 grid at 2×) | SVG | Transparent | Foreground at 50% | Empty table; no sample rows, no invented numbers. | No |
| asset-22 | Board unavailable | `site/src/pages/board.astro:101` ("Board unavailable"), `site/src/components/BoardCounter.astro:15` | `board.json` could not be read or validated. | 32×32 | SVG | Transparent | Foreground at 50%; not red, since it is not a result | Missing data, not a failed mod. | No |
| asset-23 | Drop zone | `site/src/pages/index.astro:52` and `site/src/pages/convert.astro:19` (not built: "There is no drop zone here yet") | Where a JAR is dropped. | 32×32 | SVG | Transparent | Idle in foreground; drag-over in `#ffb454` | A file arriving; no Java cup, no blocks. | No |

### (f) Issue, discussion and label icons

None. GitHub allows no custom image in these places:

- **Issue forms:** the 5 forms in `.github/ISSUE_TEMPLATE/` take no images.
- **Discussion categories:** each takes one emoji from GitHub's own set. Today they are :mega: Announcements, :speech_balloon: General, :bulb: Ideas, :hash: Nightly results, :ballot_box: Polls, :pray: Q&A, :hash: Rule proposals, :raised_hands: Show and tell and :hash: Show your port.
- **Labels:** the 24 labels take a colour only.

### (g) Discord emoji and stickers

None. The project has no Discord server.

### (h) Everything else

| ID | Name | Where | Role | Size (px) | Format | Background | States | Content (10 words) | Scene generator |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| asset-24 | Mod icon fallback | `site/src/lib/board.ts` (`icon_url` may be null; `site/src/lib/board.test.ts:45`); board rows show no icons yet | Stands in for a mod that has no Modrinth icon. | 32×32, plus 64×64 at 2× | SVG | Transparent | None | Neutral tile; never a block or a mod's art. | No |

## Generated in code

These rows need no artist. The scene generators draw them, or code sets them in type.

| ID | Name | Where | Role | Size (px) | Format | Background | States | Content (10 words) | Scene generator |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| code-01 | Wordmark "Jarrow" | `site/src/components/Header.astro:22` | The name on its own. | Follows the type size | Live text; SVG outlines only where the font cannot load | Transparent | Dark and light | Bricolage Grotesque 600 as it is; no custom letters. | No (type) |
| code-02 | README banner | `README.md:1` and `docs/media/banner.webp` (#33), rendered by `site/scripts/render-banner.mjs` | The first line of the README. | 1600×600, ≤ 60 KB (now 33,758 bytes) | WebP | Sunrise scene | One image for GitHub's light and dark themes | Sunrise and the word Jarrow; add asset-01 once drawn. | Yes (sunrise) |
| code-03 | GitHub social preview | Repository settings, Social preview (not set: `usesCustomOpenGraphImage` is false, so GitHub draws its own card) | Link previews of the repository. | 1280×640 (GitHub's minimum is 640×320), ≤ 1 MB | PNG | Sunrise scene | None | Sunrise, asset-02, the one-line description; no badges. | Yes (sunrise) |
| code-04 | Report badge | `crates/report` (badge data, planned) | Mod authors link a harness report with it. | 20 high, width follows the text | SVG | None | Green, yellow, red, not run | Result word and a report link; never a promise. | No (`crates/report`) |

Every page needs its own `og:image`: `site/src/layouts/Base.astro:33-37` writes `og:type`, `og:site_name`, `og:title`, `og:description` and `og:url`, but no image. Each card is 1200×630 PNG with the page's scene behind the page title and asset-02, and no states.

| ID | Page | Scene | Content (10 words) | Scene generator |
| --- | --- | --- | --- | --- |
| og-01 | `/` | Sunrise | Title and lede; numbers only from `board.json`. | Yes |
| og-02 | `/convert` | Sunrise (default; the design direction names no scene) | Title only; no drop zone until it works. | Yes |
| og-03 | `/board` | Ember field | Real counts from `board.json`, or none at all. | Yes |
| og-04 | `/board/<id>` (planned) | Ember field | Mod name as text; never the mod's own icon. | Yes |
| og-05 | `/directions` (planned) | Sunrise (default) | Loader names as text, never as logos. | Yes |
| og-06 | `/authors` | Aurora band | Title only; no author names or avatars. | Yes |
| og-07 | `/contributors` | Aurora band | Title only; avatars belong on the page. | Yes |
| og-08 | `/about` | Aurora band | Title only. | Yes |
| og-09 | `/docs/*` | Sunrise (default) | The docs page title, set in Bricolage. | Yes |
| og-10 | `/releases` (planned) | Sunrise | Latest version number, read at build time. | Yes |
| og-11 | `/roadmap` | Sunrise (default) | Current milestone name, read at build time. | Yes |
| og-12 | `/status` | Sunrise (default) | Title only; no live uptime figures. | Yes |
| og-13 | `/privacy` (planned) | Sunrise (default) | Title only. | Yes |
| og-14 | `/404` | Sunrise (default) | Not used for sharing; same card as `/`. | Yes |
| og-15 | `/blog` | Sunrise (default) | Post title per post. | Yes |
| og-16 | `/compat`, `/deps`, `/profiler`, `/translate` (later tabs) | Sunrise (default) | Tab name and "not built yet". | Yes |

## Inspiration

- **Primer Octicons:** a strict 16 px grid and one weight across a large set. https://primer.style/octicons/
- **Kenney 1-Bit Pack:** 16×16 tiles in one ink, CC0. https://kenney.nl/assets/1-bit-pack
- **Pixelarticons:** pixel UI icons that stay legible at small sizes. https://pixelarticons.com/
- **PICO-8:** 8×8 sprites from a fixed small palette. https://www.lexaloffle.com/pico-8.php
- **Playdate:** 1-bit drawings next to one signature yellow. https://play.date/
- **Susan Kare's Macintosh icons:** whole ideas in a 32×32 grid. https://kareprints.com/
- **Departure Mono:** a pixel typeface built on one grid. https://departuremono.com/
- **Horizon:** the reference site this design follows, with pixel scenes over one accent. https://playhorizon.dev/

The form language:

- **Square pixels only.** Every shape is square pixels on one 16 px grid, with no anti-aliasing, curves or gradients, and it must read at 16 px before it is scaled up by whole numbers.
- **One ink.** Each icon uses one ink: brand amber `#ffb454` on night blue for Jarrow's own marks, and the mint, tier amber and red trio only for tier and result icons.
- **Jarrow's own world.** Shapes come from a sunrise, a horizon or a file changing hands, never from Minecraft, its blocks, Java or any loader's logo.
