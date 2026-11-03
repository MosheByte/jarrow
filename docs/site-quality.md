# Site quality

Every pull request that changes `site/` or the design tokens in `crates/report/src/tokens.rs` is held to three things before it may merge: the thirty rules in [`docs/design/thirty-rules.md`](design/thirty-rules.md), the Lighthouse bars below, and a design review against the Horizon screenshots in [`docs/design-research/horizon/`](design-research/horizon/). The workflow is [`.github/workflows/site-quality.yml`](../.github/workflows/site-quality.yml); the checks are in [`site/tests/rules/`](../site/tests/rules/).

`site-quality-ok` is the aggregate check. It is not a required check yet: it becomes one once the rebuilt site meets the bars.

## What runs

| Job | What it does | Fails when |
| --- | --- | --- |
| `changes` | Lists the changed files and decides whether the site changed. A pull request that does not touch `site/` or `tokens.rs` skips everything else and passes. | never |
| `build-and-serve` | `npm ci` and `npm run build` in `site/`, then serves `dist/` with `wrangler pages dev` (the Pages runtime, with local KV, D1 and R2) and requests `/`, `/board/` and `/docs/`. Uploads `dist/` for the jobs below. | the build fails or a page does not answer 200 |
| `lighthouse (mobile)`, `lighthouse (desktop)` | Lighthouse CI, three runs each of `/`, `/board/` and `/docs/`, judged on the median run. Writes a table to the job summary and uploads the HTML and JSON reports. | a bar below is missed |
| `rules` | `npm run unit` (the helpers, and that `thirty-rules.md` matches `lib/rules.mjs`) and `npm test`: the stylesheet checks over the shipped CSS and the rendered checks over every built page at 1440 x 900 and 390 x 844. Writes a table of the thirty rules to the job summary. | any automated rule has a violation |
| `screenshots` | Playwright captures of `/`, `/board/`, `/docs/` and every page the change touches, at 1440 x 900 and 390 x 844, first screen and full page. | a capture fails |
| `screenshots-push` | Pushes the PNGs to the orphan branch `pr-assets` under `pr/<number>/<sha>/` as jarrow-bot, then posts one comment on the pull request, updated in place, that sets each screenshot beside its Horizon counterpart. | the push fails |
| `design-review` | A reviewer agent (`anthropics/claude-code-action`, Claude Opus 5.5) that has not seen the change reads the rules and the research report, looks at the screenshots, the paired Horizon screenshots and the built site, and returns one status per rule with evidence. The workflow posts its list as one comment, updated in place, and uploads `verdict.json`. | any rule is broken, or the reviewer returns no valid verdict |
| `site-quality-ok` | Reads every job's conclusion through the API, as `ci-ok` does. | any job failed or was cancelled |

In the merge queue the build, Lighthouse and the rules run again on the merge group; the screenshots are not retaken, and `design-review` passes only if the review of the pull request's head commit passed.

## The bars

| Measure | Bar | Where it is checked |
| --- | --- | --- |
| Performance, mobile (Lighthouse default throttling) | 95 or more on each page | `site/lighthouserc.json` |
| Performance, desktop preset | 98 or more on each page | `site/lighthouserc.desktop.json` |
| Accessibility | 100 on each page, both presets | both configs |
| Cumulative layout shift | 0 on each page, both presets | both configs |
| Transfer of `/`, mobile | 350 KiB or less (Lighthouse `total-byte-weight`) | `site/lighthouserc.json` |
| Third-party requests on `/` | none: every request goes to the site's own origin | `lighthouse-summary.mjs`, from the `network-requests` audit |

`wrangler pages dev` serves gzip; Cloudflare serves brotli in production, so the transfer measured in CI is an upper bound of what visitors download.

## Pages and their Horizon counterparts

The rendered checks run on every page in `dist/` and every server-rendered page in `dist/_routes.json`. The screenshots cover the always-captured pages plus the pages a change touches: a file under `src/pages/` or `src/content/docs/` maps to its route, and a component, layout, style or script maps to every page under `src/pages/` that imports it directly (`lib/routes.mjs`). Dynamic routes (`[id]`) are listed in the comment as not captured.

| Jarrow | Horizon capture | Why |
| --- | --- | --- |
| `/` | `home` | the hero with the scene and the card |
| `/board/` | `stats` | the ember field hero and live counts |
| `/board/<id>/` | `mods-jei` | one item's page |
| `/convert/` | `benchmarks` | the interactive tool page |
| `/directions/` | `mods` | an index with filters |
| `/authors/`, `/roadmap/`, `/contributors/` | `hub` | the aurora band hero |
| `/releases/` | `releases` | the release list |
| `/status/` | `stats` | live figures |
| `/privacy/` | `privacy` | sentences naming hostnames |
| `/about/` and other short pages | `artists` | a short prose page |
| `/docs/` | `docs` | the docs index layout |
| `/docs/<page>/` | `docs-<page>` where Horizon has one, else `docs-install` | the docs article layout |
| `/404` | `players` | Horizon's 404, captured at `/players` |

## Markup the checks rely on

Most rules are read from computed styles alone. Where a rule needs to know what an element is, the checks look for these attributes; the rebuilt pages set them.

| Attribute | On | Rules |
| --- | --- | --- |
| `data-number` | a number that changes or aligns (counts, readouts) | 14: `tabular-nums` |
| `data-unit` | the unit after such a number | 14: 14 px at 50 % white |
| `data-card` | a card. Filled, rounded boxes in `<main>` are judged as cards even unmarked | 12 |
| `data-tier` | an element that shows a result tier, the only place the green, amber and red may appear | 8 |
| `data-brand-source` | the inline SVG of a brand mark, set to its source URL in `site/BRAND-SOURCES.md` | 22 |
| `data-hero` | the hero section, when it holds no scene canvas | 17, 25 |
| `data-docs-page` | a docs layout outside `/docs/` | 3, 5, 24 |
| `data-docs-prose` | docs prose outside Starlight's `.sl-markdown-content` | 5, 24 |

A visually hidden span is one that is absolutely positioned at 1 px or clipped (`clip: rect(0 0 0 0)` or `clip-path: inset(50%)`), like Tailwind's `sr-only`. Every page is measured with the system colour scheme set to dark; rule 1 loads each page once more set to light to prove there is no light mode.

## Secrets and pull requests from forks

`screenshots-push` uses `BOT_DEPLOY_KEY`: written to a 0600 file in `RUNNER_TEMP`, used only through `GIT_SSH_COMMAND`, removed by an exit trap, never printed. It pushes only to `pr-assets`, adds to it and never force-pushes; `main` is never touched. `design-review` uses `ANTHROPIC_API_KEY`. Neither job runs code from the pull request: the push job only moves the PNGs, and the reviewer works in a checkout of the base branch that holds only `docs/design/` and the research report, with the screenshots and the built pages beside it. It has the Read, Glob and Grep tools and nothing else, its permission rules deny it `/proc`, `/etc`, the runner's temp folder and the git configuration, and its list reaches the pull request through a workflow step that redacts anything shaped like a key or a token.

Pull requests from forks and runs started by bots get no secrets: they skip `screenshots-push` and `design-review` with a notice, and `site-quality-ok` treats the skip as a pass. A maintainer who has read the change can run both with `workflow_dispatch`.

## Running it by hand

`workflow_dispatch` takes `pr_number` (the pull request to test and comment on) and `ref` (a branch, tag or SHA to build instead of the pull request's head). The checks always come from the commit the workflow runs on, so a new check can be tried against `main`:

```sh
gh workflow run site-quality.yml --ref <branch with the checks> -f ref=main
gh workflow run site-quality.yml -f pr_number=42
```

Locally, from `site/tests/rules/`, after `npm run build` in `site/`:

```sh
npm ci
npx playwright install chromium          # or PW_CHANNEL=chrome to use an installed Chrome
node serve.mjs --background              # wrangler pages dev on http://127.0.0.1:8788
npm run unit && npm test                 # the rules; RULE_ROUTES=/,/board/ limits the pages
node rules-summary.mjs                   # the table of the thirty rules
node screenshots.mjs --out shots         # /, /board/ and /docs/ at both widths
npx lhci collect --config=../../lighthouserc.json && npx lhci upload --config=../../lighthouserc.json
node lighthouse-summary.mjs --preset mobile --config ../../lighthouserc.json
```

## Baseline

The site as it stood on `main` before the rebuild, measured by the first run of this workflow. It is expected to miss: the rebuild against the thirty rules is the next wave of work.

Baseline pending: filled in from the first run against `main`.
