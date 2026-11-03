# playhorizon.dev — design research

Analysis of https://playhorizon.dev and https://github.com/horizon-rust/horizon, made on 2026-11-03. Every page reachable from the site's navigation and footer, plus the 28 docs subpages, three mods subpages and the 404 page, was loaded in headless Chrome for Testing 153 at 1440×900 and at 390×844 (Android user agent, touch enabled); hover and keyboard checks were repeated in the Claude desktop app's browser pane at 1440×900. Screenshots are in `docs/design-research/horizon/`, named `<page>-<viewport>-first.png` (the first screen) and `<page>-<viewport>-full.png` (the whole page, stitched from viewport tiles with `prefers-reduced-motion: reduce` so that scroll-driven reveals do not hide content). Zoomed crops and state captures are named `home-1440-zoom-*.png`, `home-1440-slider-focus.png`, `home-1440-nojs-*.png`. The computed-style, network, Lighthouse and measurement dumps the text cites (`<slug>-1440.json`, `<slug>-390.json`, `home-measure.json`, `home-network.json`, `lh-home-*.json`) are in `docs/design-research/horizon/data/`.

Source material read: the two shipped stylesheets (`/_next/static/chunks/1954ozk_cgkuy.css`, 98,856 bytes, and `/_next/static/chunks/1b5g4h2mnvmk2.css`, 4,987 bytes; line numbers below refer to a Prettier-formatted copy of the first), the home page's nine JS chunks (the hero scene is in `2e2qzhmi6kfxn.js`; line numbers refer to its Prettier-formatted copy), the page HTML as served, the computed styles read through the DevTools protocol, two Lighthouse 13 runs, the repository README, and `crates/horizon-theme/tokens.css` from the repository. Where a claim could not be measured it says "not verified".

---

## 1. Inventory

The site is a Next.js app (Turbopack chunk names, `_next/image` optimizer, React server-component `?_rsc=` prefetches) with Tailwind v4 utilities. It has nine top-level destinations in the header (About → `/#about`, Hub, Benchmarks, Mods, Releases, Docs, Players → `/stats`, GitHub, Download), four footer link groups, 28 docs pages, 47 mod detail pages (`/mods/<slug>`), `/mods/submit`, `/mods/policy`, `/artists`, `/privacy` and a 404 page. `/about` and `/players` as paths return 404 (`curl -o /dev/null -w '%{http_code}'`); the nav items with those labels point at `/#about` and `/stats`.

| Page | Purpose | Sections in order (words) | Words in `<main>` | Height 1440 / 390 | First screen at 1440 | First screen at 390 |
|---|---|---|---|---|---|---|
| `/` | Landing page: what Horizon is, download, benchmark, community, gallery, contributors, audit | Horizon: The game you know, rebuilt in Rust. (91); Same inputs, identical outputs. (134); Your Minecraft Server, 24/7 Online. (26); Built in the open. (69); The same game you love, 6× faster. (25); Built by 199 people.…and Claude too (50); Every subsystem passes the harnesses that compar (184) | 579 | 5713 / 10054 | Header; eyebrow "Minecraft: Java Edition 26.3 · Written in Rust"; two-line h1 with accent second line; 23-word lede; two 48 px buttons; platform line with live count; the benchmark card (label, h2, lede, two bars, slider, hint) at right; sunset scene with sun under the card in the bottom 300 px | Header with Download button and hamburger; eyebrow; h1 (36 px); lede; two full-width stacked buttons; platform line; top of the benchmark card; star field only (the sun is below the fold) |
| `/hub` | Roadmap and work board: Now / Next / Later, Kanban, journal, how to help; "Simple / Technical" toggle | Where Horizon is going. (45); Being done now (759); Next (410); Recently shipped (75); How to help (48) | 804 | 4391 / 6930 | Aurora band hero with moon top-right; eyebrow "Hub · Now, next and how to help"; h1 "Where Horizon is going."; lede; segmented tabs (Overview, Roadmap, Kanban, Journal, Contribute, People) and Simple/Technical toggle; first paragraph and "Being done now" h2 | Aurora hero with moon; h1; lede; tab row scrolling horizontally; toggle; first paragraph |
| `/benchmarks` | Three machines, both games: hitches, fps, loading, memory; an interactive estimate for the visitor's computer | Horizon against Java Edition. (84); Pick a computer (267); Hitches in a minute of play (56); Frames per second (15); Waiting (20); Memory while playing (30); Where every frame landed (18); Feel the difference (114); Where Java Edition is still faster (89); What about your computer? (162); How it was measured (159) | 875 | 4802 / 8501 | Dithered bar silhouette behind the hero; h1 "Horizon against Java Edition."; lede; white "Compare a computer" and grey "Estimate yours" buttons; three machine summary rows at right; "Pick a computer" h2 and the machine picker | h1; lede; two buttons; three machine rows stacked below |
| `/mods` | Mod index (47 projects): search, type filter, sort; links to policy, "Make a mod", "Submit yours" | Horizon mods (1278) | 1278 | 4933 / 9477 | No hero: search field "Search 47 projects", "Mod policy / Make a mod / Submit yours" buttons, type filter tabs with counts, sort tabs, "47 projects", and the first seven mod rows with 48 px pixel icons, stars, download counts | Search, button row, filter tabs wrapped on three lines, sort tabs, first two rows |
| `/releases` | The latest 40 GitHub releases with notes, filtered by game / launcher / nightly; opened ones marked as seen | Every release, as it ships. (43); The latest 40 releases. (633) | 676 | 5924 / 8700 | Sunset scene with a wide low sun; eyebrow "Releases · horizon-rust/horizon"; h1 "Every release, as it ships."; lede; "Browse releases" and "All releases on GitHub"; "Changelog" label at the bottom edge | Eyebrow; h1; lede; stacked buttons; sun scene below |
| `/docs` | Docs index: what Horizon is and is not, supported systems, where to go next | What Horizon is (628) | 628 | 2946 / 4444 | Three columns: left rail (search "/" shortcut, grouped links), article "What Horizon is" with lede and "In one minute" list, right rail "On this page" | Search field; "Docs menu" dropdown; breadcrumb; h1; lede; "On this page" disclosure; first list |
| `/artists` | Call for 3D artists: what would be made, what fits, how to get in touch | We're looking for 3D artists. (213); What we'd make together (53); What fits (68); Get in touch (45) | 213 | 2025 / 2712 | h1 "We're looking for 3D artists."; lede; sections "What we'd make together" and "What fits" start | h1; lede; first list |
| `/stats` | Live player count ("Players" in the nav): now, peaks, history chart, platform split, official SMP | Who plays Horizon right now. (46); Now and the peaks (222); Over time (70); The Official Horizon SMP (103) | 268 | 3101 / 3921 | Ember field hero; eyebrow "Players · Live, every minute"; h1 "Who plays Horizon right now."; lede with link; four stat tiles (966 / 953 / 1,112 / 17); "On which system" bars | h1; lede; stat tiles stacked |
| `/privacy` | What the website keeps and what it does not, in sections | This site keeps almost nothing. (617); In short (68); Release views (94); What stays in your browser (66); What your browser loads from GitHub (52); Signing in (97); What the server fetches (53); The launcher, the game and Discord (62); Who is responsible, and your rights (83) | 617 | 3131 / 4614 | h1 "This site keeps almost nothing."; lede; "In short" list | h1; lede; "In short" |
| `/players` | 404 page (the nav item "Players" links to /stats; /players itself does not exist) | Nothing at this address. (24) | 24 | 1256 / 1631 | "404"; h1 "Nothing at this address."; one sentence; three buttons; footer starts at y=585 | Same, stacked; footer visible |
| `/docs/install` | Docs page "Install and first launch" | Article (932 words in main) with h2s: What you need; Download the launcher; First launch; Updating; One version for everyone: the Nightly; The Versions page; Something went wrong | 932 | 4016 / 6181 | Docs layout: left rail, article h1 "Install and first launch" with lede, first h2 "What you need", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/how-horizon-is-built` | Docs page "How Horizon is built" | Article (1014 words in main) with h2s: Who builds it; How changes are checked; Why it's safe to run; What it doesn't contain; Check it yourself | 1014 | 3909 / 6155 | Docs layout: left rail, article h1 "How Horizon is built" with lede, first h2 "Who builds it", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/faq` | Docs page "FAQ" | Article (992 words in main) with h2s: Do I need to buy Minecraft?; Is Horizon official?; Is Horizon the same as Minecraft Rust Edition?; Will my Forge, NeoForge or Fabric mods work?; Can I join public servers?; When will Microsoft sign-in work?; Can vanilla players join my Horizon world or server?; Can I use my existing worlds?; Where are my worlds and settings?; Does it change my normal Minecraft install? … | 992 | 4784 / 6844 | Docs layout: left rail, article h1 "FAQ" with lede, first h2 "Do I need to buy Minecraft?", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/troubleshooting` | Docs page "Logs, crashes and bug reports" | Article (683 words in main) with h2s: If the game crashes; Where the files are; What to try first; Reporting a bug; Data the launcher can send | 683 | 3527 / 5128 | Docs layout: left rail, article h1 "Logs, crashes and bug reports" with lede, first h2 "If the game crashes", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/performance` | Docs page "Performance" | Article (1017 words in main) with h2s: What Horizon does differently; The client; Servers; Where else to look | 1017 | 4304 / 6627 | Docs layout: left rail, article h1 "Performance" with lede, first h2 "What Horizon does differently", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/settings` | Docs page "Settings worth knowing" | Article (1021 words in main) with h2s: Graphics: presets, Simple and Advanced; Distant terrain (LOD); Graphics API on Windows; Shader packs; Resource packs; Controllers; Extras; Where your settings are stored | 1021 | 4926 / 7061 | Docs layout: left rail, article h1 "Settings worth knowing" with lede, first h2 "Graphics: presets, Simple and Advanced", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/multiplayer` | Docs page "Multiplayer" | Article (709 words in main) with h2s: Play with friends from your own world; Open to LAN; Joining a server; Vanilla players on a Horizon server | 709 | 3091 / 4743 | Docs layout: left rail, article h1 "Multiplayer" with lede, first h2 "Play with friends from your own world", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/hosting` | Docs page "Host a server" | Article (1794 words in main) with h2s: Before you start; From the launcher: the Servers page; From the command line; How fast is it | 1794 | 9484 / 11753 | Docs layout: left rail, article h1 "Host a server" with lede, first h2 "Before you start", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/mods` | Docs page "Mods and data packs" | Article (1081 words in main) with h2s: Mods; Finding resource packs and modpacks; Java mods from the Mods page (next launcher release); Data packs | 1081 | 4418 / 6666 | Docs layout: left rail, article h1 "Mods and data packs" with lede, first h2 "Mods", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/content-mods` | Docs page "Content mods" | Article (1326 words in main) with h2s: What a content mod is; The manifest; Blocks; Items; Recipes, loot, tags, models and names; Namespaces; Limits; Start one; Try it and share it | 1326 | 6691 / 9297 | Docs layout: left rail, article h1 "Content mods" with lede, first h2 "What a content mod is", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/make-a-mod` | Docs page "Make a mod" | Article (1205 words in main) with h2s: Two kinds of mod; How a native mod works; The path; Install the tools; The files of a native mod; Check it against Horizon; See it in the game; What players see | 1205 | 5232 / 7405 | Docs layout: left rail, article h1 "Make a mod" with lede, first h2 "Two kinds of mod", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/mod-manifest` | Docs page "The bundle" | Article (1060 words in main) with h2s: The manifest; Hook points or a pinned commit; compatibility.json; The .hnmod file; What a native mod can do | 1060 | 5288 / 7248 | Docs layout: left rail, article h1 "The bundle" with lede, first h2 "The manifest", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/mod-code` | Docs page "Hook points and helpers" | Article (2213 words in main) with h2s: Hook points; Helpers; The item panel and recipes (version 2); A worked example: Playtime | 2213 | 11124 / 14422 | Docs layout: left rail, article h1 "Hook points and helpers" with lede, first h2 "Hook points", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/test-a-mod` | Docs page "Test a mod" | Article (759 words in main) with h2s: Native mods; Content mods; Check it as the registry does | 759 | 3601 / 5431 | Docs layout: left rail, article h1 "Test a mod" with lede, first h2 "Native mods", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/publish-a-mod` | Docs page "Publish a mod" | Article (958 words in main) with h2s: Before you submit; Package a native mod; Sign it; Submit it; Review and publication; Updates; Reports and takedowns | 958 | 4291 / 6401 | Docs layout: left rail, article h1 "Publish a mod" with lede, first h2 "Before you submit", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/modpacks` | Docs page "Modpacks" | Article (893 words in main) with h2s: What the Modpacks page does; What carries over, and what does not; Example: All the Mods 10; From the command line | 893 | 3776 / 5700 | Docs layout: left rail, article h1 "Modpacks" with lede, first h2 "What the Modpacks page does", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/contributing` | Docs page "Contribute" | Article (824 words in main) with h2s: The repository; Build it; Parity harnesses; How pull requests are checked; What never goes in | 824 | 3879 / 5935 | Docs layout: left rail, article h1 "Contribute" with lede, first h2 "The repository", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/agents` | Docs page "Control Horizon with your AI agent" | Article (1224 words in main) with h2s: What an agent can do; Connect it from the launcher; Copy-paste config; A game you started yourself; The command line; Safety; No AI: turn it all off | 1224 | 6013 / 8714 | Docs layout: left rail, article h1 "Control Horizon with your AI agent" with lede, first h2 "What an agent can do", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/apis` | Docs page "APIs at a glance" | Article (618 words in main) with h2s: What you can build on; What the levels mean; How they are designed; References that can't drift | 618 | 3109 / 5417 | Docs layout: left rail, article h1 "APIs at a glance" with lede, first h2 "What you can build on", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api` | Docs page "Horizon APIs" | Article (721 words in main) with h2s: How these APIs are designed; References are generated; Each surface's page | 721 | 3622 / 6252 | Docs layout: left rail, article h1 "Horizon APIs" with lede, first h2 "How these APIs are designed", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/launcher` | Docs page "Launcher API" | Article (509 words in main) with h2s: Overview; Guide; Reference; Examples; Stability; Not there yet; Changelog | 509 | 3200 / 4627 | Docs layout: left rail, article h1 "Launcher API" with lede, first h2 "Overview", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/server` | Docs page "Server API" | Article (1010 words in main) with h2s: Overview; Guide; Reference; Examples; Stability; Not there yet; Changelog | 1010 | 5041 / 7476 | Docs layout: left rail, article h1 "Server API" with lede, first h2 "Overview", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/server-reference` | Docs page "Server reference" | Article (1876 words in main) with h2s: Commands; server.toml; Control API (/api/v1) | 1876 | 9924 / 16827 | Docs layout: left rail, article h1 "Server reference" with lede, first h2 "Commands", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/agent-control` | Docs page "Agent control API" | Article (548 words in main) with h2s: Overview; Guide; Reference; Examples; Stability; Not there yet; Changelog | 548 | 3366 / 5056 | Docs layout: left rail, article h1 "Agent control API" with lede, first h2 "Overview", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/data-formats` | Docs page "Data formats" | Article (498 words in main) with h2s: Overview; Rules for every format; Not there yet; Changelog | 498 | 3581 / 5034 | Docs layout: left rail, article h1 "Data formats" with lede, first h2 "Overview", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/mod-index` | Docs page "The registry index and .hpack" | Article (2347 words in main) with h2s: index.json; Before a file is installed; .hpack; The registry repository; Changelog | 2347 | 9452 / 13678 | Docs layout: left rail, article h1 "The registry index and .hpack" with lede, first h2 "index.json", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/shaders` | Docs page "Shader API" | Article (2333 words in main) with h2s: Overview: what a shader author will be able to do; The design; The pack format (schema 1); Guide, reference and examples; Stability; Not there yet; Changelog | 2333 | 8557 / 12388 | Docs layout: left rail, article h1 "Shader API" with lede, first h2 "Overview: what a shader author will be able to do", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/docs/api/stability` | Docs page "API stability and versioning" | Article (1148 words in main) with h2s: Versions; What counts as a breaking change; Stability levels; Deprecation and removal; Compatibility tests; Changelog; Today | 1148 | 5967 / 9778 | Docs layout: left rail, article h1 "API stability and versioning" with lede, first h2 "Versions", right rail | Search, "Docs menu" dropdown, breadcrumb, h1, lede, "On this page" |
| `/mods/jei` | One mod's page (example of the 47): description, versions, ratings | JEI for Horizon (38); Features (857); Native code (68); Versions (283) | 895 | 4563 / 7539 | Aurora band hero; "← Every mod"; 48 px pixel icon; eyebrow "Native mod by enderliker"; h1; lede; "No ratings yet · 3.1k downloads · Version 0.1.0 · MIT"; below: disclaimer sentence, prose, a Download 0.1.0 · 15 KB button and a File card (name, size, SHA-256, signing key) | Hero stacked; download button; file card |
| `/mods/submit` | Form page to submit a mod (requires sign-in) | intro (35) | 111 | 1351 / 1931 | No hero; "← Every mod"; h1 "Submit to the registry"; 48-word lede; links to Publish a mod / Make a mod; "Sign in to submit…" and a white "Sign in with GitHub" button; footer visible at y≈700 | Same, stacked |
| `/mods/policy` | Rules for mods listed on the site | Who makes the mods (679) | 702 | 3171 / 5043 | No hero; "← Every mod"; h1 "Mod policy"; lede; h2 "Who makes the mods" with a bold-lead bullet list | Same, stacked |

Word counts are `innerText` words of each `<section>` (nested sections appear once); page heights are `scrollHeight` in CSS px. Screenshots: `<slug>-1440-first.png`, `<slug>-390-first.png`, `<slug>-1440-full.png`, `<slug>-390-full.png`, where the slug is the path with `/` replaced by `-` (`docs-api-server`, `mods-policy`; the 404 was captured at `/players`).

Reading the table: every page shares the same 72 px fixed header (`header.h-18`, measured 72 px tall, `home-1440-first.png`) and the same four-column footer (logo, Mojang/Microsoft marks and disclaimer, then "Who makes it", "Licences", "Privacy"; `home-1440-full.png` bottom 670 px). Docs pages share one layout: a 208 px left rail with a search field and grouped links, a 600 px article column, and a 176 px "On this page" rail (`docs-1440-first.png`). At 390 the rails collapse into a search field plus a "Docs menu" dropdown above the article and an "On this page" disclosure (`docs-390-first.png`).

---

## 2. Tokens, measured

All values are computed styles read from the DOM at 1440×900 unless marked 390, or rules quoted from the stylesheet.

### 2.1 Type

Three families load, all self-hosted woff2 subsets via `@font-face` in `1b5g4h2mnvmk2.css`:

| Family | Role | Weight range served | `font-display` | Fallback (`size-adjust`) |
|---|---|---|---|---|
| Bricolage Grotesque | display: h1, h2, big numbers | 200–800 variable | swap | "Bricolage Grotesque Fallback" = local Arial, size-adjust 105.43 %, ascent-override 88.21 % |
| Geist | everything else | 100–900 variable | swap | "Geist Fallback" = local Arial, size-adjust 104.76 % |
| Geist Mono | `code`, `pre`, docs inline code | 100–900 variable | swap | "Geist Mono Fallback" = local Arial, size-adjust 134.59 % |

The three Latin subsets are `<link rel=preload as=font>` in the document head (`home-measure.json` → `linksRel`); Geist Mono is only fetched on pages that use `code` (docs pages: `document.fonts` lists it as loaded on `/docs/install`, not on `/`). `--font-display: var(--font-bricolage)` (CSS line 131) and `--default-font-family: var(--font-geist-sans)` (line 129) wire the families into Tailwind.

Measured roles (computed style, element, screenshot):

| Role | Family | Size | Weight | Line height | Letter spacing | Colour | Where measured |
|---|---|---|---|---|---|---|---|
| h1 (home hero) | Bricolage Grotesque | 59.9 px (`clamp(2rem,10.4cqi,4.75rem)`) | 600 | 61.1 px (1.02) | −2.1 px (−0.035 em) | #fff; last line `.text-accent` | `h1.rise.font-display`, `home-1440-first.png` |
| h1 at 390 | Bricolage Grotesque | 35.6 px | 600 | 36.3 px | −1.24 px | #fff | `home-390-first.png` |
| h1 (docs article) | Bricolage Grotesque | 48 px | 600 | 1.08 | −0.03 em | #fff | `docs-1440-first.png` |
| h2 (section) | Bricolage Grotesque | 48 px (`clamp(2rem,3.6vw,3rem)`) | 600 | 51.8 px (1.08) | −1.44 px (−0.03 em) | #fff | "Same inputs, identical outputs.", `home-1440-full.png` |
| h2 at 390 | Bricolage Grotesque | 32 px | 600 | 34.6 px | −0.96 px | #fff | `home-390-full.png` |
| h2 (card) | Bricolage Grotesque | 32 px | 600 | 35.2 px (1.1) | −0.64 px (−0.02 em) | #fff | "3× the frame rate.", `home-1440-zoom-benchmark-card.png` |
| h2 (docs prose) | Bricolage Grotesque | 26 px (1.625 rem) | 600 | 31.2 px (1.2) | −0.52 px (−0.02 em) | #fff | `.docs-prose h2`, CSS line 5078 |
| h3 (feature) | Geist | 18 px | 500 | 28 px | normal | #fff | "Your own copy", `home-1440-full.png` |
| h3 (docs prose) | Geist | 19 px (1.1875 rem) | 600 | 1.3 | −0.01 em | #fff | CSS line 5086 |
| Big number | Bricolage Grotesque | 40 px (card), 144 px (`clamp(6rem,18vw,11rem)`, "281") | 600 | 1 | −0.02 em | #fff / `.text-accent` | `.tabular-nums.font-display`, `home-1440-full.png` |
| Lede (hero) | Geist | 18 px | 400 | 29.25 px (1.625) | normal | white/70 → #b7b6bc on page | `p.text-lg.leading-relaxed.text-white/70` |
| Body (cards, features) | Geist | 15 px | 400 | 1.625 | normal | white/60 → #9f9da5 | `p.text-[15px].text-white/60` |
| Docs body | Geist | 16 px | 400 | 1.7 | normal | #ffffffb8 | `.docs-prose`, CSS line 5071 |
| Label / eyebrow | Geist | 14 px | 500 | 20 px | normal, **no uppercase** | white/60 | "Benchmarks", "Community", "Contributors" |
| Nav link | Geist | 14 px | 400 | 20 px | normal | white/65 (#aba9b1 on page) | `nav a`, `home-1440-first.png` |
| Small / hint | Geist | 12 px | 400 | 1.625 | normal | white/50 (#88858f) | "Drag the standard launcher bar…" |
| Footer paragraph | Geist | 14 px | 400 | 22.75 px (1.625) | normal | white/50 | `footer p` |
| Footer heading | Geist | 14 px | 500 | 20 px | normal | white/80 | `footer h2` "Who makes it" |
| Mono inline code | Geist Mono | 13.44 px (0.84 em) | 400 | inherits | normal | #ebe5ff on #ffffff12, radius 5 px, padding 0.12em 0.38em | `.docs-prose :not(pre) > code`, CSS line 5062 |
| Mono block | Geist Mono | 13.5 px | 400 | 1.65 | normal | #e7e1ff on #0a0614, radius 12 px, padding 1em 1.15em | `.docs-prose pre`, CSS line 5143 |
| Pill count | Geist | 10 px | 600 | 10 px | normal | #100a1f on #a98fff, 16 px tall, min 16 wide, full radius | "9+" in header, `home-1440-zoom-releases-pill.png` |

`textTransform: uppercase` is used by no text element on the home page (the probe returned an empty list). `tabular-nums` is applied to every number that changes or aligns: the download count, the fps readouts, the subsystem index "01"–"14", the score legend.

### 2.2 Colour

The site has two colour systems: the Tailwind utility set compiled into `1954ozk_cgkuy.css`, and the repository's `crates/horizon-theme/tokens.css`, which is the launcher's design system (generated by `cargo run -p xtask -- tokens`) and is **not** loaded by the website — none of its custom property names (`--page`, `--ink`, `--night-sky-0`…) appear in the shipped CSS. The website's only custom properties of its own are `--background: #100a1f` and `--foreground: #f4f1ff` (CSS line 4809–4810). Everything else is a literal in a utility class.

Website colours, by use (all from the stylesheet; "on page" = composited over #100a1f and read back from a 1×1 canvas):

| Hex | Where |
|---|---|
| #100a1f | page background (`:root{--background}`, `meta[name=theme-color]`, manifest `background_color`), the gradient's last stop, text on white buttons and on the accent pill |
| #0a0919 | top of the hero sky gradient; `bg-[#0a0919]` |
| #0a0614 | `pre` background in docs |
| #0c0718 / #0c0818 | benchmarks section backgrounds |
| #140d26 | the light-panel ink colour (benchmarks "What about your computer?" text on #f3f0fa; `text-[#140d26]`, `/55 /60 /65 /70` variants) |
| #18112c, #1c1433, #221a3a, #231a3d, #241a3d, #2a2045 | raised panels and selected rows on benchmarks, hub, mods (`bg-[#18112c] p-5` is the benchmark card; `#241a3d` is the selected machine) |
| rgba(255,255,255,0.04) → #1a1428 composited | the hero benchmark card (`bg-white/[0.04]`), gallery figure placeholders, mod rows |
| white at 0.03, 0.05, 0.06, 0.07, 0.08, 0.09, 0.1, 0.12, 0.14 | every other surface step: secondary buttons (0.08), hover of the same (0.12), hamburger (0.06), rows (0.03/0.04), footer rule (0.08), code background (0.07) |
| #f4f1ff | `--foreground`, body text default |
| #ffffff | headings, primary labels |
| white/80, /70, /65, /60, /50, /45, /40 | text hierarchy, see 2.1 |
| **#6b4bff** | primary button background, `::selection` background, Horizon bar gradient start, "overload-flow" gradient |
| **#7a5dff** | primary button hover (`hover:bg-[#7a5dff]`, verified in the browser pane: rgb(122,93,255) on hover) |
| #7c5cff | glow behind the Horizon fps bar (`bg-[#7c5cff] blur-md`) |
| **#a98fff** | the accent: focus rings (`focus-visible:ring-[#a98fff]`, slider thumb outline), the "9+" pill, bar fills, Discord-card-adjacent links, hub "Simple" toggle, logo horizon line (`fill-[#a98fff]`) |
| #c4b5ff, #c9bbff, #d4c9ff, #d8ceff, #d9ceff, #e4dcff, #e7e1ff, #e9dcff, #ebe5ff | lighter accent steps: docs links (#c4b5ff, hover #e4dcff), release-notes links (#a98fff, hover #c4b5ff), code text (#e7e1ff), hub links |
| linear-gradient(90deg, #ddd3ff, #a98fff 55 %, #8466ff) | `.text-accent` — the heading accent is **gradient text** (`background-clip: text`, CSS line 4823) |
| #5865f2 / #4752c4 | the Discord card background and its hover (`bg-[#5865f2] hover:bg-[#4752c4]`) |
| #5cc9f2, #5ccff5, #86d8f6 | the Sculk sponsor card's cyan: heading accent, button, hover |
| #f2c46b, #f6dca2, #ffe9a8, #ffd3a6 | amber: warning text, "30 to 60 fps" bars |
| #8be0b4, #bdf0d4, #4f9e80 | mint: "Over 120 FPS", success text |
| #ff8a8a, #f7b2a8, #f07a6a, #ff6b6b | red: "Under 30", errors |
| #d97757 | the hidden "…and Claude too" caption (Anthropic's terracotta) |
| #f3f0fa, #ece8f7, #f4f3fa | the one light panel (benchmarks estimator) and docs figure background |
| #702bfa + #efe9fb | the "Drawn by" mark on the gallery |

Hero sky (canvas, `2e2qzhmi6kfxn.js` lines 418–432): 15-step ramp #0a0919 → #0f0c28 → #151136 → #1d1444 → #281850 → #361b5b → #481e66 → #5d236e → #752873 → #902f76 → #ab3a75 → #c64a71 → #de5f6a → #ef7c63 → #f89d62. Sun (lines 436–442): #fff5d2 → #ffe4a0 → #ffca78 → #ffac5f → #ff8c58 → #fb6c62 → #ee5277. Ridges (lines 954–979): #3f1f5c/#c45a7f, #25123e/#7a3068, #100a1f/#2c1640. The CSS fallback behind the canvas is `linear-gradient(to bottom, #0a0919 0%, #151136 38%, #361b5b 62%, #902f76 80%, #100a1f 86%)` (line 1097 of the chunk; `home-1440-nojs-first.png` shows it alone).

### 2.3 Spacing, container, radii, borders, shadows, buttons

- Spacing unit: `--spacing: 0.25rem` (CSS line 87); every margin and padding in the markup is a multiple of 4 px (`p-6` = 24, `p-8` = 32, `gap-x-16` = 64, `pt-12` = 48, `pb-24` = 96).
- Container: `max-w-6xl` = `--container-6xl: 72rem` = 1152 px, centred, `px-6 lg:px-10` (24 px gutters below 1024, 40 px above). Measured: content box 1152 px wide at left 144 px (`home-measure.json` → `container`). At 390 the container is the viewport, 24 px gutters (`home-390-first.png`: text starts at x=24).
- Header: `h-18` = 72 px; transparent at top; after scrolling becomes `bg-[#100a1f]/75 backdrop-blur-md` (blur 12 px) with a 300 ms transition on background-color and backdrop-filter (`headerScrolled` in `home-measure.json`). No border, no shadow.
- Radii (CSS lines 115–119 plus literals): 2 px (stat bars, slider thumb), 3 px (`icon.svg` corner, legend squares), 4–5 px (inline code), 6 px (code-copy button), 8 px (`rounded-lg`: nav Download button, hamburger, 40 px buttons, release-notes images), 10 px (hero buttons `rounded-[10px]`), 12 px (`rounded-xl`: docs blockquote, `pre`, tables, hub cards, dropdowns), 16 px (`rounded-2xl`: the hero card, community cards, gallery figures via `clip-path: inset(0 round 1rem)`, benchmark panels, docs figures), 24 px (`rounded-3xl`: benchmark machine panel), 28 px (`rounded-[28px]`: the light estimator panel), 9999 (pills, avatars).
- Borders: almost none. The hero card has `border: 0px` and `ring-inset` with no ring colour set → no visible edge; separation is by background alpha alone. The few borders: `border-t border-white/[0.08]` on the footer, `border-white/[0.08]`-class rules between subsystem rows, `border border-dashed border-white/40` on the HORIZON15 coupon button, `border-[#5cc9f2]/60` on the sponsor card's secondary button, `border-left: 2px solid #ffffff26` on release-notes blockquotes.
- Shadows: `boxShadow: none` on the hero card, buttons, header and footer. The only shadows are glows: `shadow-[0_0_10px_#a98fff]` and `0 0 12px -2px #a98fff73` on the "9+" pill and live dots (`home-1440-zoom-releases-pill.png` shows the halo), and `bg-[#7c5cff] blur-md` behind the Horizon fps bar.
- Backdrop blur: hero card `backdrop-blur-xl` (24 px), scrolled header 12 px.
- Buttons (measured, `home-measure.json` → `buttons`):

| Button | Height | Padding x | Radius | Font | Background | Text | Hover (from CSS) |
|---|---|---|---|---|---|---|---|
| Primary "Download the launcher" | 48 px | 20 px | 10 px | 15 px / 500 | #6b4bff | #fff | #7a5dff |
| Secondary "View on GitHub" | 48 px | 20 px | 10 px | 15 px / 500 | white/8 | #fff | white/12 |
| Header "Download" | 36 px | 16 px | 8 px | 14 px / 500 | white/10 | #fff | white/15 |
| Header "GitHub" link | 36 px | 12 px | — | 14 px / 400 | none | white/65 | white |
| White button ("Compare a computer", 404 "Back home") | 40 px | 16 px | 8 px | 14 px / 500 | #fff | #100a1f | white/90 |
| Pill toggle (hub "Simple") | 32 px | 12 px | 6 px | 14 px / 500 | #a98fff or #fff | #100a1f | — |
| Hamburger | 36×36 | — | 8 px | — | white/6 | white/80 | white/10 |
| Coupon "HORIZON15" | 44 px | 16 px | 8 px | 15 px / 500, 0.025 em | none, dashed border | #fff | — |

Icon size inside buttons: 17–18 px (`size-[17px]`, `size-[18px]`); gap 10 px (`gap-2.5`).

### 2.4 Light/dark

The site is dark only: `:root { color-scheme: dark }` (CSS line 4813), `<meta name="color-scheme" content="dark">`, `theme-color #100a1f`, and no `prefers-color-scheme` media query exists in either stylesheet (the probe's media-query list is `['(hover: none)', '(prefers-reduced-motion: reduce)']`). Light appears only as panels inside the dark page: the benchmarks "What about your computer?" estimator (`bg-[#f3f0fa] text-[#140d26]`, `benchmarks-1440-full.png`), the white "Source" community card, white buttons, and docs figure frames (`background: #f4f3fa`).

The repository's `tokens.css` is a different, two-theme system (light `:root` with `--page: #ece7f7`, dark under `prefers-color-scheme: dark` and `[data-theme="dark"]` with `--page: #0f0d17`, `--accent: #9a86ff`, a "night" set `--night-sky-0…5`, and a "glass" set with `--glass-accent: #6b4bff`, `--glass-page: #0a0919`, `--glass-warn: #f2c46b`, `--glass-danger: #ff8a8a`). The glass set shares four literals with the website (#6b4bff, #0a0919, #f2c46b, #ff8a8a); the rest of the website's palette is not derived from that file. Its type scale (`--text-display: clamp(34px … 64px)`, `--text-night-display: clamp(40px … 72px)`, weight 700, tracking −0.035em) is close to but not equal to the website's h1 (60 px, weight 600, −0.035 em).

---

## 3. The illustration system

Nothing on the site is a static illustration file. Every scene is drawn at runtime into a `<canvas>` by three React components in `2e2qzhmi6kfxn.js` (`HorizonScene`, `PixelAurora`) and `17jhgilgykja-.js` (`PixelFire`), from a seeded pseudo-random generator (`mulberry32`-style, lines 455–462, seed 7) so every visitor sees the same picture. Each canvas is `aria-hidden="true"` with no alt text, which is right: the scenes carry no information. All carry `[image-rendering:pixelated]` and fade in over 700 ms (`opacity-0 transition-opacity duration-700 … data-ready:opacity-100`) once the first frame is drawn. The README banner `docs/media/banner.webp` (1600×600, 17,988 bytes) is a baked export of the same scene with the hero line set in a pixel font; the live site does not use a pixel font anywhere.

### 3.1 The hero scene (home, releases)

Screenshots: `home-1440-first.png`, `home-1440-zoom-sun.png`, `home-1440-zoom-clouds.png`, `home-1440-zoom-ridges.png`, `releases-1440-first.png`, `home-390-full.png`.

**Subject.** A sun setting behind three ranges of block-stepped hills, under a sky that runs from near-black violet at the top to magenta and orange at the horizon; flat clouds; a star field; a band of fog at the horizon; pixel trees on the front ridge; blinking fireflies.

**Framing at 1440.** The scene fills the whole hero section (`min-h-svh`, measured 900 px); the sky gradient covers the full height, but the horizon line sits at 0.45 × scene height from the bottom and the hills occupy the bottom ~230 px, so the illustration lives under the copy and the card, not behind them (`home-1440-first.png`: text block ends at y≈560, sun top at y≈600). The sun is placed at 63 % of the width on wide screens (`sunX` default `0.63` when `innerWidth ≥ 1024`, line 1056), i.e. directly under the benchmark card, which makes the card's blurred backdrop pick up the orange band. The sun radius is 0.4 × "landscape height" × scale.

**Framing at 390.** The scene height drops to `clamp(0.3 × innerHeight, 180, 320)` = 253 px at 844 px tall (line 745), the sun moves to 50 % width, and the scene is pinned to the bottom of a hero that is now 1,390 px tall, so a phone user sees stars behind the h1 and the sun only after scrolling past the card (`home-390-full.png`).

**Pixel grid.** `px = clamp(round(width / 340), 3, 6)` (line 751): 4 px at 1440, 3 px at 390, 6 px at ≥ 2040. Measured on `home-1440-first.png`: horizontal and vertical run lengths in the ridge and sun areas are multiples of 4 (y=780 row: runs of 4, 8, 12, 16, 32; column x=300: 16 runs of exactly 4). The low-resolution buffer is 360×225 at 1440×900 (`heroCanvas[1]` in `home-measure.json`: canvas 360×225, CSS 1440×900).

**Two layers.** The scene is two stacked canvases. The lower one (`alpha: false`, 1440×900 at device scale, no pixelation) holds the smooth sky gradient, a radial orange glow around the sun (`createRadialGradient`, lines 544–550, alpha pulsing 0.45–0.5 at 0.9 rad/s), round antialiased stars with a four-point sparkle on 6 % of them, and a rare shooting star (a 1.5 px stroked line, lines 575–600, once every ~9 s with 80 % probability). The upper one is the 360×225 `ImageData` buffer, scaled up with `image-rendering: pixelated`, holding everything blocky: clouds, the sun disc, three ridges, trees, fog, fireflies. The pixel grid therefore applies to the terrain and the sun but not to the sky gradient or the stars, which is why a zoomed crop shows 1-px gradients behind 4-px blocks (`home-1440-zoom-sun.png`).

**Palette count.** 15 sky colours, 7 sun colours, 6 ridge colours (3 ridges × body + rim), 2 star colours (#d2c8ff cool, #fff3e4 warm), fog #e0697c, cloud rim #ffb27c, cloud underside #f3a6bd, firefly #ffd98c: 33 named colours, with per-pixel mixing (`n(a,b,t)` lerps in 8-bit RGB, line 403) between them, so the rendered image is not a strict indexed palette.

**Sun.** Drawn scanline by scanline (lines 630–670): a disc of radius `r` in low-res pixels, each row's colour taken from the 7-stop sun ramp by vertical position, and below 38 % of the disc's height every `max(4, round(r/5))`-th row is skipped with a widening gap (the horizontal bands in `home-1440-zoom-sun.png`), the retro "scanlined sun". The band offset scrolls with time (`f = 2.2 × t`), so the bands drift downward. On `/releases` the same component is called with a wider, lower sun (`wideSun` prop: `scale`, `rise`), giving the larger disc with five gaps in `releases-1440-first.png`.

**Clouds.** Eight clouds (3 big: 120–260 px wide ÷ px, 20–26 % as tall; 5 thin streaks: 8–20 % of width, 3–6 rows tall; lines 870–890) built as bitmask stacks of shrinking rows (`mask` function, lines 806–840): each row is narrower than the one below by 6–26 % per side, giving the stepped silhouette; the bottom row is marked rim (lit from the sun with a horizontal Gaussian `glow[u]` so clouds nearer the sun are brighter) and the top edge marked highlight. They scroll at 0.5–2.6 px/s and parallax with the pointer (`depth` 0.4 big, 0.8 thin).

**Ridges and trees.** Three ridges from 1-D value noise with 4 octaves (lines 896–930), quantised to the grid (`Math.round(... / n) * n`), each with a lit rim pixel on the top edge whose strength follows the sun glow. Trees are three sprites (7×7 round, 5×8 pine, 5×5 bush, lines 464–487 as ASCII rows) placed every 14–44 columns on the front ridge. Fireflies: `round(width/36)` dots blinking with `sin³`.

**How it meets the UI.** The card does not float "over" the illustration in the sense of overlap: at 1440 the card ends at y=584 and the sun's top is at y≈600 (`home-1440-first.png`), but the card's `backdrop-blur-xl` plus 4 % white tint means any part of the sky under it is blurred, and the sky gradient under the card is already magenta. There is no CSS gradient bridge between the scene and the next section: the scene's own fog band and the sky's last stop (#100a1f at 86 %) end in the page colour, and the hero's `overflow-hidden` crops it. The bottom of the hero is a straight edge at y=900.

**Resize.** A `ResizeObserver` rebuilds the whole scene (new seed-7 layout at the new size) on every size change (line 742), so resizing re-generates clouds and ridges rather than scaling a bitmap; the pixel size steps between 3/4/5/6 at 1020/1530/2040 px widths.

**Static or animated.** Animated at ≤ 32 fps (`if (e - L < 31) return`, line 741): clouds drift, stars twinkle, bands drift, fireflies blink, sun glow pulses; the pointer moves clouds and ridges by up to 8 px × depth (`pointermove`, line 756), and scrolling sinks the sun by up to 0.9 r (`O.sink`, line 760). Under `prefers-reduced-motion: reduce` the component draws one frame and attaches no pointer or scroll listener (line 1079: `!D && !E && …`; `E` is the media query). Off-screen, an `IntersectionObserver` cancels the loop (line 1073).

**What it is not.** No raster file is loaded for the hero: the image list for `/` (`home-1440.json` → `imgs`) has three canvases and no `<img>` in the hero; the network log shows no hero image. It is not a Mojang texture: no block texture, no vanilla sky; the sun is a disc, not Minecraft's square sun. It is not an AI texture or a stock photo: the generator and every palette value are in the shipped JS.

### 3.2 The aurora band (hub hero, home "Community" section)

Screenshots: `hub-1440-first.png`, `home-1440-full.png` at y≈2850–3400, `hub-390-first.png`.

`PixelAurora` (`2e2qzhmi6kfxn.js` lines 1120–1400): a canvas sized to its parent (`hubs`: full hero; home: 1440×560 for the community section, `heroCanvas[2]`), same `px` rule, same 10-colour subset of the sky ramp (#100a1f → #ab3a75). Two overlapping wave bands are computed per column as sums of sines (`front`: 0.8 h ± 0.07 sin(0.004x + 0.32t) ± 0.035 sin(0.011x − 0.55t); `back`: 0.6 h …), each with a "rays" modulation along x; each pixel's brightness is a function of distance below the band edge (exponential falloff, `Math.exp(-f / w)`) and is then **ordered-dithered with an 8×8 Bayer matrix** (the 64-entry table at line 1121) before being mapped to a palette index. That dithering is what reads as the "1 px grid noise" across the hub hero and the community band: it is not a noise texture, it is the Bayer threshold pattern made visible by a shallow brightness ramp. A 12×12 three-tone moon sprite (`a`/`b`/`c` rows, lines 1139–1152, #fff3d6/#e3d5b5/#bfae90) sits top-right (`moonTop` 32 px) with a 2.4-sprite-wide halo, and ~`width/30` stars blink in two tones (#e9e1ff, #8f7fc4) with a one-pixel cross at > 0.93. The band's top and bottom edges are the stepped "pixel wave" dividers visible on the home page: they are not separate divider elements or SVG masks — the stepping is the band's own dithered falloff cut off by the section's `overflow-hidden`. On the home page the section is `relative isolate … overflow-hidden py-16 sm:py-24` with the canvas `absolute top-0 left-0 -z-10`, and the content (cards) sits on top. Animated at ≤ 32 fps; static under reduced motion; stops off-screen.

### 3.3 The ember field (stats hero) and the bar silhouette (benchmarks hero)

`PixelFire` (`17jhgilgykja-.js`): a 14-colour ramp (#100a1f → #fcd795, lines 16–29) drawn as a dithered field that is densest at the bottom of the hero and fades upward (`stats-1440-first.png`), same `px` rule (line 140), same reduced-motion and intersection logic (lines 76, 111, 162). The benchmarks hero (`benchmarks-1440-first.png`) shows a dithered skyline of vertical bars behind the copy; its drawing code is in `36z3gw_pz2l7a.js` (a `canvas` with `getContext("2d")` and the same palette seed `#100a1f` at line 1868). Exact generator not read in detail: not verified beyond the palette and the pixel rule.

### 3.4 Raster images that are not illustrations

- Game screenshots as evidence: `/media/nether.webp` (1600×900 source, served at 750 w), `village-800.webp`, `night-800.webp`, `underwater.webp`, all with descriptive alt text ("A piglin with a golden sword in a crimson forest above a sea of lava in the Nether"), in `clip-path: inset(0 round 1rem)` figures, with a `motion-safe:group-hover:scale-[1.03]` zoom over 700 ms (`home-1440-full.png`, y≈3450–4200). The README's `docs/media/*.webp` are the same kind (sunset 1600×900 78 KB, village 219 KB, nether 115 KB, survival 71 KB, settings 1280×720 41 KB, two launcher captures 1280×800).
- `/about-wallpaper.jpg` (738×384 source, alt "A Minecraft windmill above a field of lavender") in the "What it is" section, flipped (`-scale-x-100`), with a left-to-right mask (`mask-image: linear-gradient(to right, transparent, black 65%)` at ≥ 48rem, top-to-bottom below).
- `/media/cherry-grove.jpg` at 45 % opacity under the "How it's built" card.
- `/media/steve-alex-wolf.webp` (575×450, alt ""): Mojang's Steve, Alex and wolf render, in the Sculk Hosting sponsor card, breaking out of the card's top edge (`-mt-16`, `@[48rem]:absolute`). This is third-party key art, see section 7.
- `/minecraft.webp` (64×71, 2,574 bytes): a grass-block icon at 18 px in the hero eyebrow, see section 7.
- `/media/bench/*.webp`: product photos of the three benchmark machines and their chips, each credited (Wikimedia Commons, CC BY-SA 4.0 / CC BY 4.0 / CC0 with author names) in the page copy.
- 208 contributor avatars from `avatars.githubusercontent.com`, proxied through `/_next/image` at 48 px, in 40 px circles.

---

## 4. Why it reads as a Minecraft site without Minecraft's assets

| Element | Mechanism (measured) | Done badly, it would be |
|---|---|---|
| The logo | `icon.svg`: a 16×16 grid, two 2×4 white posts, two 2×4 posts at 40 % white below them (the reflection), one 14×1 row of #a98fff across the middle. `shape-rendering="crispEdges"`. The posts and bar are whole grid cells, so the mark is an "H" whose crossbar is a horizon and whose lower half is its reflection in water. (`home-1440-zoom-logo.png`) | A rounded-sans "H" with a gradient, or a cube icon from an icon set. |
| Pixel sun with scanline gaps | Section 3.1: disc drawn row by row at 4 px, gaps every `r/5` rows below 38 % height, drifting. | A smooth circle with a `box-shadow` glow; or a Minecraft square sun lifted from a texture. |
| Stepped clouds | Row-stack bitmasks, each row narrower than the last (3.1). | Rounded `border-radius` blobs; or cloud emoji. |
| Block-stepped hills with pixel trees | Quantised value-noise ridges, three depths, 1-px rim light, 5–8 px tree sprites (3.1). | A smooth SVG wave path, or a photo with a blur. |
| Bayer-dithered gradients | 8×8 ordered dither on the aurora and ember fields (3.2, 3.3): the only "noise" on the site, and it is deterministic. | A PNG noise texture at 5 % opacity over everything; a "grain" filter. |
| 1 px/4 px grid discipline | `image-rendering: pixelated` on every scene canvas, cell size tied to viewport width so cells stay whole; spacing scale of 4 px; radii 2–16 px; square sub-pixel shapes (legend squares `rounded-[3px]`, bars `rounded-[2px]`). | Pixel art scaled to a fractional size (blurry, uneven cells). |
| The way numbers are set | `tabular-nums` everywhere; big counts in the display face at 40–144 px ("152 fps", "281", "966 playing now") with the unit in 14 px white/50 beside them; index numbers "01"–"14" in 12 px white/40. | Numbers in the body face with proportional figures, jumping as they animate. |
| Score cells | Four 10×10 squares (`size-2.5 rounded-[3px] bg-[#a98fff]`) per subsystem, filled in quarters — a hearts/hunger-bar reading without the heart glyph. (`home-1440-full.png` y≈4780–5100) | A 100 % progress bar or five stars. |
| Real game frames as evidence | Four webp screenshots in the gallery, captioned "Every frame here was rendered by Horizon, with vanilla's block models, lighting, sky and fog." and "Drawn by [Horizon mark]". | Stock renders, or screenshots with no claim about who rendered them. |
| Vanilla vocabulary in copy | "vanilla", "worlds", "chunks", "render distance 12 (default) / 32 (far)", "Open to LAN", "hitches", "1 % lows". | "Seamless gameplay experience". |
| Fireflies, moon sprite, shooting star | Time-based sprites in the scenes (3.1, 3.2). | GIFs. |
| Mod icons | Each mod has a 48×48 pixel-art icon in a 12 px-radius tile (`mods-1440-first.png`); inline SVG `viewBox="-2 -2 14 14"` at 48 px for the first-party ones. | Lucide icons on coloured circles. |
| The one pixel font that is absent | The site sets no pixel font (only Bricolage, Geist, Geist Mono load); the README banner uses one once. Restraint is part of the mechanism: the pixel language stays in the pictures and the grid, not in the text. | Press Start 2P for every heading. |

---

## 5. Why it does not read as generated design

Each row compares what a template or generated landing page does with what this site does, with the measurement.

**Hero copy.** The h1 is 8 words ("Horizon: The game you know, rebuilt in Rust." including the `sr-only` "Horizon:"; 7 visible). The lede is 23 words and contains two numbers (26.3, 281) and three concrete nouns (client, game logic, harnesses): "The Minecraft: Java Edition 26.3 client and its game logic, rewritten in Rust and checked against the vanilla game by 281 differential harnesses." No adjective in either. The eyebrow is a fact, not a slogan: "Minecraft: Java Edition 26.3 · Written in Rust". The download line is live data: "Launcher 0.2.37 · 23.3 MB · 233,445 downloads" (the count changed between loads: 233,445 → 234,132 → 234,659, so it is read from GitHub at render time, and the size switches with the detected OS: 23.3 MB on Linux, 16.0 MB on the Android user agent, i.e. the Windows zip).

**The benchmark card in the first screen.** A real `<input type="range" min=0 max=400 step=5 aria-label="Standard launcher frame rate" aria-valuetext="50 fps">` (`home-1440.json` → `sliders`), whose value sets the comparison: dragging to 100 changes the Horizon readout to 303 (`home-measure.json` → `slider`: 147/50 → 303/100). The bars are `clip-path: inset(0 calc(100% − var(--fill)) 0 0 round 2px)` and animate in with a 0.45 s clip-path transition. The small print says what the number is ("Average frames per second in vanilla 26.3 with no mods installed") and what it is not ("Results vary with hardware").

### Tells of generated design that this site avoids

1. **Three-column feature grid with Lucide icons in tinted circles.** Here: the "What it is" section has three features in a row, but the icons are 20 px line glyphs sitting inline before the 18 px heading with no circle, no tint, no card (`home-1440-full.png` y≈1180–1310); the heading row is `flex items-center gap-2.5`.
2. **Gradient text on every heading.** Here: `.text-accent` *is* gradient text, but it is applied to the last two to three words of a heading only, and never to body text, labels or buttons; measured on 9 of 14 h1/h2 on the home page, never on docs h2 or footer h2.
3. **ALL-CAPS tracked eyebrow on every section.** Here: zero elements with `text-transform: uppercase` on the home page; eyebrows are sentence case, 14 px, weight 500, white/60 ("Benchmarks", "Community", "Contributors").
4. **Abstract blob/mesh gradient background.** Here: the only gradients are the sky ramp (a sunset), the accent text, and the sponsor card's radial cyan glow; no `blur-3xl` orbs (the stylesheet defines `--blur-2xl` but no element uses a blurred coloured orb).
5. **"Trusted by" logo row / testimonials.** Here: none. Social proof is a counted list: "Built by 199 people." with six top contributors and their commit counts (9,265; 1,176; 1,066; 958; 593…) and 193 avatars, every one linked to its GitHub profile with the name as alt text.
6. **Fake metrics ("10k+ users").** Here: 233,445 downloads to the unit, "966 playing now · Updated 05:18 PM, again every minute", "Last audited 2026-10-13".
7. **A pricing table or "Get started free".** Here: "Download the launcher" and "View on GitHub"; the software is MIT and the footer says so.
8. **Cards with 1 px borders and drop shadows.** Here: `border: 0` and `box-shadow: none` on every card; surfaces are white at 3–8 % alpha; the hero card has a 24 px backdrop blur instead of a border.
9. **Generic stock or AI hero image.** Here: a procedurally drawn scene whose code ships with the page (section 3).
10. **Buttons with gradient fills and glow.** Here: flat #6b4bff, flat white, flat white/8; the only glow is on the 16 px "9+" pill.
11. **A footer of four link columns with "Product / Company / Resources / Legal".** Here: three paragraphs in plain sentences: "Horizon is made by Ludovic Marie; reach him through lu4ovic.com. This site is developed by Anthony S of DezerX, who is responsible for it." / "Horizon's original work is under the MIT licence. The fonts are Geist and Geist Mono, under the SIL Open Font License. Every asset on this site is listed in LEGAL.md." / a 90-word privacy paragraph that names the hostnames the page calls (api.github.com, raw.githubusercontent.com, avatars.githubusercontent.com).
12. **"© 2026 Horizon. All rights reserved."** Here: "Made in the open, in Rust." with the Rust mark, and the repo path `horizon-rust/horizon`.
13. **Empty states that say "No data".** Here: "No samples in this range yet: the history began on 31 October 2026." / "No measured computer at this render distance to estimate from yet." / "Nothing on the roadmap matches" / "Asking the server…" as the loading state for the live count (server-rendered placeholder "–").
14. **A 404 that says "Oops! Page not found".** Here: "404 / Nothing at this address. / The page moved, or never was. These are the places most people look for." with three buttons (`players-1440-first.png`).
15. **Error toasts in red boxes.** Here: "A server error occurred. Reload to try again." and "Reload to try again, or go back." as plain sentences (strings in the shipped JS).
16. **Hover effects that lift and shadow every card.** Here: hover changes colour alpha only (`hover:bg-white/[0.12]`, `hover:text-white`); the only transform on hover is the gallery image's 3 % scale and the hidden caption's 4 px rise.
17. **Icons for every list item.** Here: the 14-subsystem audit uses numbers "01"–"14" and score squares; the FAQ uses questions as h2 with no icons.
18. **"Learn more →" links.** Here: links say where they go: "Visit contributors", "All contributors on GitHub", "the open reports", "what it can send, and how to delete it"; text links that leave the site carry an sr-only `(opens in a new tab)` and, in body copy, a 14–16 px arrow glyph; buttons and brand-mark links (GitHub, Discord, Download) do not.
19. **Marketing superlatives.** Here: the one superlative-shaped claim, "6× faster", is a number, and the benchmarks page has a section titled "Where Java Edition is still faster".
20. **Hidden affiliate banners.** Here: the Sculk Hosting card is labelled by its own wordmark, can be dismissed ("Hide this partner", stored as `horizon:partners-hidden` in localStorage per the privacy page), and the privacy page says so.
21. **A chat widget, cookie banner, newsletter modal.** Here: none; the privacy page states "No cookies for visitors."
22. **Uncredited photos.** Here: every chip photo on the benchmarks page carries the Commons author and licence.
23. **Auto-playing video hero.** Here: no `<video>` element on any page (probe `imgs` lists none).

**Where the site does use a common pattern:** gradient text on heading accents (item 2), a card grid of four community links in brand colour (Discord blue, white, two grey), stacked avatar circles, and the "9+" badge. These are listed so the inheriting project copies the restraint, not a myth.

---

## 6. Copy and voice

Ten headings, verbatim:

1. "The game you know, rebuilt in Rust."
2. "3× the frame rate."
3. "Same inputs, identical outputs."
4. "Built in the open."
5. "The same game you love, 6× faster."
6. "Built by 199 people."
7. "Every subsystem passes the harnesses that compare it."
8. "Horizon against Java Edition."
9. "Where Horizon is going."
10. "This site keeps almost nothing."

Ten sentences, verbatim:

1. "The Minecraft: Java Edition 26.3 client and its game logic, rewritten in Rust and checked against the vanilla game by 281 differential harnesses."
2. "Drag the standard launcher bar to try your own frame rate. Results vary with hardware."
3. "Install Java Edition with the official launcher and play 26.3 once. Horizon builds its resources from those files on your machine, and never downloads or shares game files."
4. "Every change lands on GitHub in public. Come ask questions, report bugs or send a pull request."
5. "Every frame here was rendered by Horizon, with vanilla's block models, lighting, sky and fog."
6. "Everyone with commits in horizon-rust/horizon, most commits first, and the people behind this site."
7. "The same three computers ran both games through the same world, on the same flight path, with the same settings. Here is what you would notice, in plain words, including where Java Edition still wins."
8. "A hitch is a frame slower than one refresh of a 60 Hz screen, so you see the same picture twice."
9. "One thing is kept on the server: a one-way hash of your IP address when you open a release, so each release counts each visitor once."
10. "The page moved, or never was. These are the places most people look for."

**Register.** Sentence case everywhere, including nav items and buttons ("Download the launcher", "Compare a computer"). Full stops at the end of headings when the heading is a sentence; none on labels ("Benchmarks"). Concrete nouns: client, launcher, harness, chunk, hash, release. Numbers written as figures, with units and thousands separators (233,445; 50.8 s; 0.8 GB; 60 Hz). Contractions are used ("doesn't", "you've", "we'd"). First person plural appears only on the artists page ("we'd like to hear from you"); elsewhere the subject is Horizon, the site, or "you". British spelling ("licence", "colour", "behaviours"). Across the home, docs index, benchmarks, privacy and artists pages the words "seamless", "powerful", "effortless", "blazing", "next-generation" do not occur (grep over the served HTML: 0 hits). Negative facts are stated as plainly as positive ones: "Not an official Minecraft product.", "Not able to sign in to a Microsoft account yet.", "No capes, or anything that hangs from the shoulders like one.", "100% means 100% of what the harnesses compare, not that nothing else differs."

**The accent-word pattern.** Page h1s put the last clause on its own line and give it `.text-accent`: "rebuilt in Rust.", "Java Edition.", "is going.", "as it ships.", "3D artists.", "right now.", "almost nothing.", "at this address." (eight of eight page heroes). Section h2s use it on a number or the last two words: "3×", "6× faster." It is **not** used on: content h2s that are plain statements ("Same inputs, identical outputs.", "Built in the open.", "Built by 199 people.", "Every subsystem passes…"), any docs h1 or h2, the footer, the FAQ questions, the hub's sub-headings, or the sponsor card (which uses its own flat cyan `text-[#5ccff5]` instead, so the sponsor's accent is visibly not the site's). The rule the site follows: the accent marks the promise in a hero and a number in a section; it never marks an ordinary heading.

---

## 7. Logos and third-party assets

Catalogue of every third-party brand, product, platform or technology named or linked on the site, and how it is shown. "Simple Icons" means the SVG path matches the simple-icons package glyph for that brand (the first path commands were compared: Rust `M23.8346 11.7033…`, Discord `M20.317 4.3698…`, Linux `M12.504 0c-.155…`, Mojang Studios `M12.484 2.077…`, Microsoft `M0 0v11.408…`, Claude `m4.7144 15.9555…`). "Octicon" means GitHub's own Primer `mark-github` 16 px path (`M8 0C3.58 0 0 3.58 0 8…`). "Lucide-style" means a 24-grid stroke icon (`stroke-width="1.75"`) that is not a brand mark.

| Brand | Shown as | Format, size | Source of the asset | Guideline compliance | Screenshot / file |
|---|---|---|---|---|---|
| Minecraft (Mojang) | Grass-block icon in the hero eyebrow; the word "Minecraft: Java Edition 26.3" | `/minecraft.webp`, 64×71 raster, rendered 16×18 px | Resembles the Minecraft app icon; origin not stated on the site; not verified | Shown at 18 px; Mojang's guidelines govern use of brand assets, and the site adds "Minecraft is a trademark of Mojang Synergies AB" and "Not an official Minecraft product" in the footer | `home-1440-first.png`, `home-1440-first.png` (eyebrow, 18 px) |
| Steve, Alex and wolf render | Sponsor card art | `/media/steve-alex-wolf.webp`, 575×450, served 640 w | Mojang key art; origin not stated; not verified | Third-party key art inside a hosting advertiser's card | `home-1440-full.png` y≈1310–1660 |
| Mojang Studios | Wordmark glyph in the footer beside the disclaimer | inline SVG, 24×24, `fill="currentColor"` at white/40 | Simple Icons `mojangstudios` | Recoloured to a 40 % white tint; `aria-hidden`, no text alternative — the adjacent sentence names Mojang | `home-1440-full.png` footer |
| Microsoft | Four-square mark in the footer | inline SVG 20×20, currentColor at white/40 | Simple Icons `microsoft` | Recoloured monochrome; Microsoft's guidance requires the four colours and a licence for logo use | same |
| Windows | Four-square glyph before "Windows" in the platform line and on `/releases` | inline SVG 14×14, `M2 2h9.5v9.5H2z…`, currentColor | Drawn by the site (not the Simple Icons path) | Monochrome lookalike of the Windows 11 mark | `home-1440-first.png` y≈546 |
| Linux | Tux before "Linux" | inline SVG 16×16, currentColor | Simple Icons `linux` (Larry Ewing's Tux outline) | Monochrome; Tux is not trademarked by the Linux Foundation; attribution to Larry Ewing is not given | same |
| macOS / Apple / Apple Silicon / Metal | Text only ("macOS on Apple Silicon", "Mac mini", "Apple M4 Pro") and a product photo of a Mac mini on `/benchmarks` | — | — | Word marks in referential use, which Apple permits; no Apple logo | `benchmarks-1440-first.png` |
| Rust | Gear-R mark before "Written in Rust" and in the footer line "Made in the open, in Rust." | inline SVG 16×16, currentColor | Simple Icons `rust` | Monochrome at white/65; the link goes to Wikipedia, not rust-lang.org | `home-1440-first.png` |
| GitHub | Mark in the header, two hero buttons, the "Source" card, the footer | inline SVG 16/17/20 px, currentColor (white, or #100a1f on the white card) | GitHub Octicons `mark-github` | GitHub's logo policy allows the mark to link to GitHub; the Octicon is the mark GitHub ships for this | `home-1440-first.png` |
| GitHub Sponsors | "Sponsor" card | Lucide-style dollar-sign stroke icon, 20 px | Drawn/generic glyph, not the GitHub Sponsors heart | Generic glyph standing in for a product | `home-1440-full.png` y≈2930 |
| Discord | "Discord" card, hub "Join the Discord" | inline SVG 20×20, Clyde, white on #5865f2 | Simple Icons `discord` | White on Discord's blurple #5865f2, hover #4752c4: matches Discord's permitted colourways; clear space not measured | same |
| Claude (Anthropic) | Hidden caption "…and Claude too" after "Built by 199 people." | inline SVG, 1.1 em, #d97757 | Simple Icons `claude` | Appears only on hover (opacity 0 → 1, 300 ms, `sm:` and up); Anthropic's terracotta | `home-1440.json` headings |
| Sculk Hosting | Sponsor card | `/media/sculk-wordmark.webp` 751×164, shown at 20–24 px tall, `alt="Sculk"` | The sponsor's own wordmark (a raster) | A wordmark supplied as webp rather than SVG; not verified against a brand page (none found) | `home-1440-full.png` |
| DezerX | Text link "DezerX" in the footer | — | — | — | footer |
| eBay | "Find one used on [eBay logo]" button on `/benchmarks` | inline SVG `viewBox 0 0 1000 400.75`, `role="img" aria-label="eBay"`, 18 px tall, four fills #f12c2d #0968f6 #ffbc13 #93c822 | SVG traced path (Inkscape-style coordinates); source not stated; not verified | Full colour on a white button, which matches eBay's "on white" rule; colours differ from eBay's published values (not verified which set is current) | `benchmarks-1440-full.png` |
| Lenovo, Intel, NVIDIA, Apple, Best Buy | Text buttons ("Lenovo's spec sheet", "RTX 4080 at Best Buy", "Buy on apple.com") and credited product photos | — | — | Word marks only; photos credited with author and CC licence | `benchmarks-1440-full.png` |
| Wikimedia Commons | Credit links | — | — | Link text names the file | same |
| Modrinth, CurseForge, Fabric, NeoForge, Forge, Quilt, Prism, MultiMC, ATLauncher | Text only, in docs and hub copy ("imports from Prism, MultiMC, the CurseForge app, the Modrinth App, ATLauncher"); one link to modrinth.com on `/docs/mods` | — | — | No logos shown | `docs-mods-1440-full.png` |
| Tailscale | Text only ("invites friends over Tailscale") | — | — | No logo | `docs-hosting-1440-full.png` |
| Vulkan, OpenGL, DirectX 12, Mesa | Text only | — | — | No logo | `docs-1440-full.png` "Supported systems" |
| Java / JDK / Adoptium | Text; link to adoptium.net on `/docs/install` | — | — | No logo | `docs-install-1440-full.png` |
| Blockbench | Text only on `/artists` | — | — | No logo | `artists-1440-full.png` |
| Geist, Bricolage Grotesque (fonts) | Named in the footer licence paragraph | — | — | — | footer |

Summary of the pattern: brand marks are used only where the brand is a destination (GitHub, Discord) or a platform badge (Windows, Linux, Rust), always monochrome in `currentColor` at the surrounding text's alpha, 14–20 px, `aria-hidden` with the brand name as adjacent text. Four exceptions break the pattern and should not be copied: the Microsoft and Mojang marks recoloured to 40 % white; the self-drawn Windows squares; a generic dollar glyph for GitHub Sponsors; and raster Mojang art (grass block, Steve/Alex) with no stated source.

### The rule this project inherits

> Every product, platform or technology named in the landing page, the docs or a link row is shown with its official logo from its own brand page or press kit, at the size and colour its guidelines allow, with the source URL recorded in site/BRAND-SOURCES.md. A placeholder icon, a generic glyph, a drawn lookalike or an emoji standing in for a real brand is a build failure. Where a brand forbids logo use, show its name as text in the site's own type and say so in BRAND-SOURCES.md.

### Starter table of official brand-asset pages

URLs below were returned by web search on 2026-11-03 (Firecrawl); the ones marked "scraped" were opened and read. Constraints are summarised from the pages' own text; read the page before use.

| Brand | Official brand / press page | Guideline page | Notes for a third-party site |
|---|---|---|---|
| Minecraft / Mojang Studios | not verified (no separate logo-download page found) | https://www.minecraft.net/en-us/usage-guidelines | Do not imply affiliation; do not use any part of the brand in your own logo; "Not an official Minecraft product" wording required |
| Microsoft | https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks | https://cdn-dynmedia-1.microsoft.com/is/content/microsoftcorp/microsoft/mscle/documents/legal/intellectualproperty/trademarks/Microsoft_logo_third_party_usage_guidance_.pdf | Licence generally required; clear space = symbol height; min 16 px; never single-colour |
| Windows | same as Microsoft | https://cdn-dynmedia-1.microsoft.com/is/content/microsoftcorp/microsoft/mscle/documents/presentations/Windows_Trademark_guidelines_2023.pdf | A trademark licence is required for any Windows logo; use the word |
| DirectX | not verified | Microsoft trademark page above | Use the word |
| Rust | https://foundation.rust-lang.org/policies/logo-policy-and-media-guide/ | https://rustfoundation.org/policy/rust-trademark-policy/ | Logo is CC-BY; must not look official or endorsed |
| GitHub, Octicons | https://brand.github.com/foundations/logo (zip: https://brand.github.com/GitHub_Logos.zip); octicons: https://primer.style/octicons/ | https://docs.github.com/en/site-policy/other-site-policies/github-logo-policy | Linking to GitHub and "social button" use permitted; do not alter or imply affiliation |
| GitHub Sponsors | not verified | GitHub logo policy above | — |
| Discord | https://discord.com/branding | same page, "Legal Brand Guidelines"; https://docs.discord.com/developers/discord-social-sdk/design-guidelines/branding-guidelines | Colour, black or white only; do not recolour or distort |
| Linux / Tux | https://www.linuxfoundation.org/legal/the-linux-mark | same | Tux by Larry Ewing, not owned by the Linux Foundation; "Linux" is Torvalds's mark |
| Apple / macOS / Metal | https://www.apple.com/legal/intellectual-property/ | https://www.apple.com/legal/intellectual-property/guidelinesfor3rdparties.html | No Apple logo without written licence; word marks in referential phrases only |
| Modrinth | not verified | not verified | Use the word until a brand page is found |
| CurseForge (Overwolf) | https://www.overwolf.com/brand-guidelines/ (Overwolf's page) | same | Not confirmed to cover CurseForge assets |
| Fabric | not verified | not verified | Use the word |
| NeoForge | not verified (https://neoforged.net/news/theproject/ mentions its own brand) | not verified | Use the word |
| Minecraft Forge | not verified | not verified | Use the word |
| Prism Launcher | https://prismlauncher.org/ (scraped: logo and assets CC BY-SA 4.0) | same | Attribution required |
| MultiMC | https://multimc.org/ (scraped: branding All Rights Reserved) | same | Use the word |
| ATLauncher | https://atlauncher.com/branding | same | Sizes given; licence terms not verified |
| Quilt | https://github.com/QuiltMC/art | repo licence | Licence not read |
| Tailscale | https://tailscale.com/press | not verified | Media kit zip on the page |
| Vulkan, OpenGL (Khronos) | https://www.khronos.org/legal/trademarks/ | https://www.khronos.org/legal/khronos-trademark-guidelines/ ; https://www.khronos.org/legal/logo-usage-guidelines | Logo packs with guidelines |
| Java (Oracle) | https://www.oracle.com/legal/logos/ | https://www.oracle.com/legal/trademarks/ ; https://www.oracle.com/a/ocom/docs/java-licensing-logo-guidelines-1908204.pdf | Licence needed for Java logo |
| Eclipse Adoptium | https://adoptium.net/docs/logo-styleguide | https://adoptium.net/docs/brand | — |
| Anthropic / Claude | https://brandfolder.com/anthropic ("Anthropic Official Brand Assets") | same page states usage guidelines apply | Link from anthropic.com not confirmed |
| eBay | https://playbook.ebay.com/foundations/logo | https://brandpermission.ebay.com/guidelines ; https://playbook.ebay.com/foundations/logo/using-our-logo | Off-platform: heritage logo on white, neutral-200 or black only |
| NVIDIA | https://www.nvidia.com/en-us/about-nvidia/legal-info/logo-brand-usage/ | https://images.nvidia.com/content/includes/gcr/pdf/NVIDIA-Trademark-and-Logo-Usage-Guidelines.pdf | Clear space; written permission for multiple logos |
| Intel | https://www.intel.com/content/www/us/en/support/articles/000015080/programs.html | https://www.intel.com/content/dam/www/legal/trademarks/brand_guidelines.pdf | Licence or written permission for logos |
| Lenovo | https://brandworld.lenovo.com/ ; https://news.lenovo.com/press-kits/ | https://www.lenovo.com/us/en/legal/copytrade/ | Written permission required |
| Best Buy | https://corporate.bestbuy.com/best-buy-logo-4/ (logo + trademark notice) | not verified | Use the word |
| Wikipedia / Wikimedia | https://wikimediafoundation.org/brand-stewardship/brand-use/ | https://foundation.wikimedia.org/wiki/Policy:Wikimedia_Foundation_Trademark_Policy ; https://foundation.wikimedia.org/wiki/Legal:Visual_identity_guidelines | Marks in hyperlinks may be resized, not modified |
| Steam (Valve) | https://partner.steamgames.com/doc/marketing/branding | https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/steamworks_docs/english/steam_brandGuidelines.pdf | Valve-approved artwork only |
| Xbox | https://learn.microsoft.com/en-us/gaming/game-publishing/resources/managed-support/how-to-access-branding-and-marketing-assets | https://www.xbox.com/en-US/legal/fansiteguidelines | Fan-site rules are non-commercial only |
| Mesa 3D | https://mesa3d.org/ (no brand page found) | not verified | Use the word |
| Simple Icons (the source of six marks on this site) | https://simpleicons.org/ | https://github.com/simple-icons/simple-icons/blob/develop/DISCLAIMER.md | The package disclaims responsibility; the brand's own permission still governs |
| Sculk Hosting, DezerX | not verified (sites exist, no brand pages found) | not verified | Ask the partner for an SVG and written permission |

---

## 8. Motion

Every animation found in the two stylesheets and the three canvas components. Durations and easings are quoted from `1954ozk_cgkuy.css` lines 4828–4985 unless stated.

| What moves | Trigger | Duration / easing | Mechanism | Reduced motion |
|---|---|---|---|---|
| Hero text, eyebrow, buttons, card ("rise") | page load, staggered by `--delay` per element (card slider at 2300 ms) | 0.9 s, `cubic-bezier(0.2, 0.7, 0.2, 1)`, `backwards` fill | `@keyframes rise { 0% { opacity: 0; transform: translateY(14px) } }` | `animation: none` (line 4968) |
| Sections below the fold ("reveal") | scroll position, CSS only | linear, `animation-range: entry 10% cover 30%` | `@supports (animation-timeline: view())`: the same `rise` keyframes bound to the element's view timeline (line 4904). Works without JavaScript (`home-1440-nojs-*.png`). | none |
| fps bars ("stat-bar") | load, then slider input | load: `var(--dur)` with the rise easing; input: `transition: clip-path 0.45s ease-out` | `clip-path: inset(0 calc(100% − var(--fill)) 0 0 round 2px)` | none |
| Slider fade | load | 0.4 s ease, delayed | `fade-in` keyframes | none |
| Logo float | continuous | 6 s ease-in-out infinite, ±8 px | `@keyframes float { 50% { translate: 0 -8px } }` on `.logo-float` — defined in the stylesheet; no element on the captured pages carries the class (grep of the served HTML: 0) | none |
| "overload-shake", "overload-flow" | benchmarks "Feel the difference" when the Java view stalls | 0.2 s / 1 s linear infinite | translate jitter by `--amp`; background-position scroll of a 160 px gradient | none |
| "race-fill", "race-time" | benchmarks loading-time race | `var(--dur)` linear `both`; time label fades at the end | `transform: scaleX(0) → 1` | none |
| Hero scene | always while on screen | ≤ 32 fps RAF loop (`e − L < 31` skip) | clouds 0.5–2.6 px/s, stars sin-twinkle, sun bands drift, fireflies sin³, sun glow pulse 0.9 rad/s, pointer parallax (clouds 8 px × depth, ridges 8 px × depth, lerp 3/s), scroll sinks the sun 0.9 r, shooting star ~every 9 s | one static frame; no pointer or scroll listeners attached (chunk line 1079–1083) |
| Aurora band | on screen | ≤ 32 fps | wave phases advance with `t`; stars blink; moon halo pulses 8 % | static |
| Ember field | on screen | ≤ 32 fps | not read in detail | static (chunk line 76) |
| Canvas fade-in | first frame drawn | 700 ms | `transition-opacity duration-700` + `data-ready` attribute | transition still runs (it is a Tailwind utility, not in the reduced-motion list) |
| Header background | scroll past top | 300 ms | `transition-[background-color,backdrop-filter] duration-300` | runs |
| Gallery figure zoom | hover | 700 ms ease-out, scale 1.03 | `motion-safe:group-hover:scale-[1.03]` | off (motion-safe) |
| Hidden "…and Claude too" | hover on the contributors h2 | 300 ms opacity + 4 px rise; icon rotates 90° over 700 ms | `group-hover:opacity-100 group-hover:translate-y-0` | runs |
| Link / button colours | hover, focus | 150 ms `cubic-bezier(0.4, 0, 0.2, 1)` (`--default-transition-duration`) | `transition-colors` | runs |
| Docs heading `#` anchor | hover / focus / `:target` | 150 ms opacity | `.heading-link::after` | runs |
| Code "Copy" button | hover on block | 150 ms opacity; always visible under `(hover: none)` | `.code-copy` | runs |
| Subsystem disclosure chevron | `<details>` open | 150 ms, rotate 180° | `group-open:rotate-180` | runs |
| `animate-pulse` | defined (`--animate-pulse`) | 2 s | not found applied on the pages captured | — |

**What does not move:** the header logo, the nav, the footer, headings after their entrance, the game screenshots (except the 3 % hover zoom), the contributor avatars, the subsystem list, numbers (fps readouts change value without a counting animation; the download count is static text). Nothing auto-scrolls, nothing loops in the UI layer; only the three scene canvases loop.

`@media (prefers-reduced-motion: reduce)` (line 4966) disables `.reveal, .logo-float, .rise, .stat-bar, .fps-range, .overload-shake, .overload-flow, .race-fill, .race-time` with `transition: none; animation: none`, and each canvas component checks `matchMedia("(prefers-reduced-motion: reduce)")` itself. Not covered: the 700 ms canvas fade-in, the 300 ms header transition, the 150 ms colour transitions, and the "…and Claude too" reveal.

---

## 9. Performance and craft

Lighthouse 13 (bundled with the `lighthouse` npm package on 2026-11-03), Chrome for Testing 153, headless, against `https://playhorizon.dev/`; raw JSON in `horizon/data/lh-home-mobile.json` and `horizon/data/lh-home-desktop.json`.

| | Mobile (default throttling) | Desktop preset |
|---|---|---|
| Performance | 73 | 92 |
| Accessibility | 100 | 100 |
| Best practices | 100 | 100 |
| SEO | 100 | 100 |
| First Contentful Paint | 1.3 s | 0.5 s |
| Largest Contentful Paint | 2.8 s | 0.9 s |
| Total Blocking Time | 960 ms | 170 ms |
| Cumulative Layout Shift | 0 | 0 |
| Speed Index | 3.4 s | 1.8 s |
| Time to Interactive | 4.5 s | 1.5 s |
| Requests | 21 | 205 |
| Transfer | 424 KiB | 948 KiB |
| Resource size | 1,262 KiB | 4,093 KiB |
| Server response | 130 ms | 140 ms |

The mobile TBT comes from hydration plus the two largest script chunks (`20box33a0_n53.js` 230 KB and `0-8-2pf2bywtz.js` 164 KB uncompressed; 157 KiB of script transferred on mobile, 218 KiB on desktop). The desktop run's 205 requests are the 169 contributor avatars (246 KiB) and 13 Next.js route prefetches (313 KiB, including a 1.5 MB uncompressed RSC payload for `/hub` and 986 KB for `/releases`) that fire on a wide viewport; the mobile run, which does not reach them, makes 21 requests.

- **Document:** 468 KB HTML, 60 KiB transferred (gzip). It contains the full page text, the JSON-LD graph, and the inline RSC payload; the home page is server-rendered with live GitHub data (download count, release list, contributors).
- **Fonts:** 3 woff2 files, 92 KiB total (Geist Latin 29 KB, Geist Mono Latin 23 KB, Bricolage Latin 41 KB), all `rel=preload`, `font-display: swap`, with metric-matched fallbacks (`size-adjust`, `ascent-override`) so swap causes no layout shift (CLS 0). Non-Latin subsets are declared but only load for their code points.
- **CSS:** two files, 104 KB raw, 18 KiB transferred.
- **Images:** every raster goes through `/_next/image` and arrives as webp regardless of source (`about-wallpaper.jpg` → `image/webp`, 17 KB at 640 w); `loading="lazy"` on everything below the hero; `srcset` with 1×/2× for the wordmark; explicit `width`/`height` on `<img>` so nothing reflows. No hero raster at all. Lighthouse's `uses-webp-images` and `uses-optimized-images` audits pass.
- **Third-party:** zero requests leave `playhorizon.dev` on the home page (41 of 41 responses in the DevTools log came from that host; avatars are proxied through the image optimizer). The footer's privacy sentence says the browser asks `api.github.com`, `raw.githubusercontent.com` and `avatars.githubusercontent.com`; on this load the server did those fetches instead. Not verified whether other pages (releases, hub) call GitHub from the browser.
- **Caching:** `_next/static` assets carry immutable hashed names.
- **Without JavaScript** (`home-1440-nojs-first.png`, `home-1440-nojs-community.png`, `docs-install-1440-nojs-first.png`): every page renders its full text, nav, footer, the benchmark card with its default numbers and the range input (which then does nothing), and the CSS gradient sky in place of the canvas; scroll reveals still work because they are CSS `animation-timeline`; the "9+" pill (client-fetched) is absent; the docs sidebar with 82 links renders. The mobile menu button exists but cannot open.
- **Craft details found while reading:** metric-compatible font fallbacks; `text-pretty` and `text-balance` on paragraphs and headings; `scroll-mt-14` on every section so anchor links clear the fixed header; `(opens in a new tab)` sr-only on text links that leave the site; `aria-valuetext="50 fps"` on the slider; `tabular-nums` on changing numbers; `[image-rendering:pixelated]` only on the low-res canvas, not the smooth one; `@media (hover: none)` rules that keep the code "Copy" button visible and widen hit targets on touch; `format-detection=telephone=no` to stop iOS auto-linking the version numbers; JSON-LD `WebSite`, `Organization`, `Person` and `SoftwareApplication` on the home page; `og:image` 1200×630 generated per page (`/opengraph-image`).

---

## 10. Accessibility

Measurements use the composited text colour (the alpha colour drawn over #100a1f and read back from a canvas) against #100a1f (page) and #1a1428 (the hero card's composited background).

| Text | Colour as rendered | Size / weight | Contrast on page | On card | WCAG |
|---|---|---|---|---|---|
| Headings, white | #ffffff | 32–60 px / 600 | 19.4 : 1 | 17.9 : 1 | AAA |
| Body default `--foreground` | #f4f1ff | 16 px | 17.4 : 1 | 16.1 : 1 | AAA |
| Hero lede white/70 | #b7b6bc | 18 px / 400 | 9.6 : 1 | 8.9 : 1 | AAA |
| Nav links white/65 | #aba9b1 | 14 px | 8.3 : 1 | — | AAA |
| Card lede, feature body white/60 | #9f9da5 | 15 px | 7.2 : 1 | 6.7 : 1 | AAA |
| Hints, footer paragraphs white/50 | #88858f | 12–14 px | 5.3 : 1 | 4.9 : 1 | AA (fails AAA for small text) |
| Accent gradient #a98fff (mid-stop) | #a98fff | 32–144 px | 7.4 : 1 | 6.9 : 1 | AAA |
| Docs links #c4b5ff | #c4b5ff | 16 px | 10.5 : 1 | — | AAA |
| White on primary button #6b4bff | #fff | 15 px / 500 | 5.2 : 1 | — | AA |
| Light panel ink #140d26 at 70 % on #f3f0fa | ≈#524e61 | 16 px | 7.1 : 1 | — | AAA |

Lighthouse's `color-contrast` audit passes on the home page; `image-alt` passes.

**Focus states.** Tabbing from the top (`home-measure.json` → `focus`): header links and hero buttons show the browser's `:focus-visible` outline (`outline: … auto 1px`, offset 1 px) in the link's own colour — no custom ring on plain links. Components with a custom ring use `focus-visible:ring-2 focus-visible:ring-[#a98fff]` (segmented controls, machine picker, benchmark tabs, docs code-copy button) and the slider thumb gets `outline: 2px solid #a98fff; outline-offset: 2px` (CSS lines 4887–4894; `home-1440-slider-focus.png` shows the ring on the 6×20 px thumb). `.fps-range:focus { outline: none }` removes the outline from the whole track so only the thumb is ringed. Docs heading anchors show their `#` on `:focus-visible` as well as hover.

**Alt text.** Illustration canvases: `aria-hidden="true"`, no alt (decorative). Game screenshots: full-sentence alts, e.g. "A torch-lit village under a square moon and a starry night sky". Contributor avatars: the username as alt for the grid, empty alt where the name is adjacent text. Brand SVGs: `aria-hidden` with the brand name as text beside them; the eBay SVG has `role="img" aria-label="eBay"`. The hero logo SVG is `aria-hidden` with "Horizon" as the link text. The grass-block `<img>` has `alt=""`. The Sculk wordmark has `alt="Sculk"`. The "Drawn by" mark has an `sr-only` "Horizon" after it.

**Benchmark slider by keyboard.** `<input type="range">` is reachable by Tab after the hero buttons and platform links, `aria-label="Standard launcher frame rate"`, `aria-valuetext` updates ("50 fps" → "100 fps"); Arrow keys step by 5 and PageUp by a larger step; the Horizon readout updates with each key (`home-measure.json` → `slider.after`: 303 fps at 100). An `aria-live` region exists on the page (empty at rest). The thumb's focus ring is visible (above).

**Other.** `lang="en"` is set. The nav has a visually hidden "9+, 40 new" text for the badge. Disclosures use native `<details>`. Touch: `(hover: none)` rules add width and visibility to controls that are hover-revealed on desktop. Not verified: screen-reader announcement order of the canvas-adjacent content, and the mobile menu's focus trapping.

---

## 11. The thirty rules

Each rule can be checked against a screenshot or a stylesheet.

1. The page background is #100a1f and `color-scheme` is dark; there is no light mode.
2. Headings and display numbers are set in Bricolage Grotesque at weight 600; everything else is Geist; code is Geist Mono.
3. The page h1 is 60 px at 1440 and 36 px at 390, line height ≤ 1.05, letter spacing −0.035 em.
4. Section h2s are 48 px at 1440 and 32 px at 390, line height 1.08, letter spacing −0.03 em.
5. Body text is 15–18 px Geist at line height 1.625; docs body is 16 px at 1.7.
6. No text element uses `text-transform: uppercase`; eyebrows are 14 px, weight 500, sentence case, white at 60 %.
7. Text colour steps are white at 100, 80, 70, 65, 60 and 50 % alpha; nothing below 50 % carries information.
8. The accent #a98fff appears on focus rings, pills, bar fills, links and the last words of a page h1; the primary button is #6b4bff and turns #7a5dff on hover.
9. Gradient text (`#ddd3ff → #a98fff → #8466ff`) is applied to at most one span per heading, and never to body text, labels or buttons.
10. Every length is a multiple of 4 px; the content column is 1152 px wide with 40 px gutters at ≥ 1024 and 24 px below.
11. The header is 72 px, fixed, transparent at scroll 0 and #100a1f at 75 % with a 12 px backdrop blur after scrolling.
12. Cards have no border and no drop shadow; their surface is white at 3–8 % alpha over the page colour, radius 16 px, padding 24–32 px.
13. Buttons are 48 px (hero), 40 px (page), 36 px (header) tall, radius 10 or 8 px, 15 or 14 px type at weight 500, with 17–18 px icons and a 10 px gap.
14. Numbers that change or align use `tabular-nums`; units follow the number in 14 px at 50 % white.
15. Every illustration is drawn into a canvas at runtime on a pixel grid of `clamp(round(viewport width / 340), 3, 6)` px and scaled with `image-rendering: pixelated`.
16. Scene canvases are `aria-hidden`, fade in over 700 ms, run at ≤ 32 fps only while on screen, and draw a single static frame under `prefers-reduced-motion: reduce`.
17. The hero scene sits in the bottom 230 px of the hero at 1440 and in a 253 px strip under the content at 390; no copy overlaps the sun or the hills.
18. No raster is used for decoration; every `<img>` is either a game screenshot with a full-sentence alt, a credited product photo, an avatar, or a partner's wordmark.
19. Section separators are the dithered edges of a scene canvas or a 1 px rule at 8 % white; there are no SVG wave masks and no gradient bridges between sections.
20. Entrance motion is one keyframe (`translateY(14px) → 0`, opacity 0 → 1) over 0.9 s with `cubic-bezier(0.2, 0.7, 0.2, 1)`, staggered by `--delay`, and bound to the view timeline below the fold.
21. Hover changes colour alpha over 150 ms; the only hover transforms are a 3 % image scale and a 4 px caption rise, both under `motion-safe`.
22. Every brand mark is an inline SVG in `currentColor` at 14–20 px, `aria-hidden`, with the brand's name as adjacent text, sourced from the brand's own kit and recorded in BRAND-SOURCES.md.
23. Every text link that leaves the site carries a visually hidden "(opens in a new tab)"; in body copy it also carries a 14–16 px arrow glyph.
24. Each page h1 is two lines with the last clause on its own line in the accent gradient; content h2s and docs headings are white.
25. Hero copy states a version number and a count; the lede is ≤ 25 words and contains at least two figures.
26. Social proof is a counted list of real people with links; there are no testimonials and no logo rows.
27. The footer is three paragraphs in sentences — who makes it, licences with links, privacy with the hostnames the page calls — plus a Mojang/Microsoft disclaimer.
28. Empty, loading and error states are sentences that say what is missing and since when ("No samples in this range yet: the history began on 31 October 2026.").
29. Body text at 50 % white or darker is never smaller than 12 px, and every text/background pair measures ≥ 4.9 : 1.
30. Pages render their full text, navigation and footer with JavaScript disabled, and the mobile transfer of the home page stays under 450 KiB with CLS 0.
