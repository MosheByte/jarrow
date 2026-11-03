---
title: Introduction
description: What Jarrow is, how it will work, and who builds it.
---

Jarrow is an open-source web service and command-line tool that will convert a Minecraft mod JAR built for Fabric into a JAR for NeoForge, and prove the result with a behaviour harness.

The goal in one sentence: drop a Minecraft mod JAR on a web page; get the same mod back for another loader, with a proof that it behaves the same.

:::note[Status]
Nothing converts yet. The project is in milestone M0, which sets up the repository, the CI and this site. See [Status](/docs/status/) for what exists today.
:::

## Why convert the JAR

Today a Fabric mod reaches NeoForge players in one of two ways: its author ports it, or players install a runtime compatibility layer such as Sinytra Connector with Forgified Fabric API. Jarrow takes a third route. It will rewrite the JAR itself, ahead of time, into a mod that NeoForge loads natively, and then test that the result behaves like the original.

## How it will work

1. **Classify.** Jarrow reads the mod's manifest and bytecode and puts it in a [tier](/docs/tiers/): A, B or unsupported, with the reasons.
2. **Convert.** Tier A mods convert in your browser (the engine is Rust compiled to WebAssembly), so the JAR never leaves your machine. Tier B mods need a deeper conversion that runs as a short-lived GitHub Actions job.
3. **Prove.** A parity harness starts the original mod on a Fabric server and the converted mod on a NeoForge server, dumps what each one registers (blocks, items, recipes, tags, loot tables and more) and compares the two. Only that comparison can mark a conversion green.

The first version covers one direction, Fabric to NeoForge, for Minecraft 1.21.1 and 1.21.11.

## Built by an AI agent, in the open

Jarrow is written by Claude, an AI agent made by Anthropic, working from a public specification ([BUILD-BOOK.md](https://github.com/MosheByte/jarrow/blob/main/BUILD-BOOK.md)). Every commit says so with a `Co-Authored-By` line, and every decision the agent takes is logged in [DECISIONS.md](https://github.com/MosheByte/jarrow/blob/main/DECISIONS.md). The project is meant to be a public example of agent-built software that has to prove its claims, so nothing on this site or in these docs is described as working unless a harness result backs it.

## Where to go next

- [Status](/docs/status/): what exists and what does not.
- [Tiers](/docs/tiers/): what A, B and unsupported will mean.
- [Legal and license policy](/docs/legal/): how Jarrow treats mod authors and licenses.
- [FAQ](/docs/faq/): "Is this legal?", "Why not Connector?", "Why is my mod red?"
- [Roadmap](/roadmap/): the milestones and their exit criteria.
