---
title: Tiers
description: Tier A, Tier B and unsupported, and what each will mean for a mod.
---

:::note
The tier classifier is not built yet (planned for milestone M1). This page describes what it will decide. No mod has a tier today.
:::

Every mod Jarrow sees will get exactly one tier, decided by a classifier that scans the JAR without converting it. The result page will name the tier and the reasons, in order.

| Tier | What it covers | Where it runs | Proof |
| --- | --- | --- | --- |
| A | Manifest, mappings, entrypoints, registries, resources, mixins, access wideners, and Fabric API calls covered by rule tables | Your browser (WebAssembly) or the CLI | Static checks, plus the harness in the nightly |
| B | Everything in A, plus API calls with no rule, event-model differences, networking, and rendering hooks the agent can port | A GitHub Actions job (JVM plus agent) | The full parity harness before the result is returned |
| Unsupported | Native libraries, coremod-style bytecode transformers, deep rendering pipelines, obfuscated or packed JARs, and mods whose license forbids modification | Nowhere | The page says so plainly and names the reason |

## In one sentence each

- **Tier A** covers what the rule tables translate mechanically, and it runs in your browser or the CLI.
- **Tier B** needs a deep conversion (decompile, rules, agent fixes, recompile) that runs in a GitHub Actions job and must pass the full parity harness before a result is returned.
- **Unsupported** means Jarrow will not convert the mod, and the reason is named.

## What the classifier will look at

The manifest of the loader, native libraries (`.so`, `.dll`, `.dylib`), custom language adapters, use of Fabric Loader internals, bytecode transformer registration, signs of obfuscation, the Fabric API symbols the mod calls (matched against the rule tables), and the license.

A tier is not a result. A Tier A mod can still come out red if the harness finds a difference. Results are green, yellow or red, and only the harness sets them.
