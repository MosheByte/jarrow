## What and why

<!-- One change, one reason. What does this PR do, and why now? -->

Milestone: <!-- M0 to M6 -->

## Proof

<!-- How do you know it works? Tests added, commands run, and for anything that claims a mod or a rule works: a link to the harness result. Without a harness result, a claim that something works gets the needs-proof label and does not merge. -->

## Site changes

<!-- Delete this section when the pull request does not touch site/ or crates/report/src/tokens.rs. -->

- Screenshots at 1440 x 900 and 390 x 844 of every page the change touches, embedded here. The `site-quality` workflow captures them and comments them beside the matching Horizon screenshot; for screenshots of your own, see [docs/site-quality.md](https://github.com/jarrow-dev/jarrow/blob/main/docs/site-quality.md).
- The thirty-rule checklist from [docs/design/thirty-rules.md](https://github.com/jarrow-dev/jarrow/blob/main/docs/design/thirty-rules.md#checklist-for-a-pull-request-that-touches-site), with only the rules the change verifiably meets ticked.
- `site-quality` passes: the rules, Lighthouse (95 mobile, 98 desktop, accessibility 100, CLS 0) and the design review.

## Checklist

- [ ] `py -3.12 tools/pr_ready.py` (or `python tools/pr_ready.py`) passes locally.
- [ ] Tests cover the change; the harness smoke set ran if `crates/` or `rules/` changed.
- [ ] No JAR, no decompiled source, no mapping file with restrictive terms, no secret in the diff.
- [ ] Rule changes describe API shapes only, cite `rules/SOURCES.md`, and copy no code from a mod, Sinytra Connector, Architectury or Forgified Fabric API.
- [ ] Nothing is described as working, green or supported without a harness result. Missing features say "not built yet (milestone Mx)".
- [ ] `AGENTS.md` and `docs/architecture.md` are updated if a crate, command or boundary changed.
- [ ] Any default the project leaves open has a `D-` entry in `DECISIONS.md`; any place where reality differs from the plan has an amendment in `docs/spec-amendments.md`.
- [ ] Agent commits carry `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
