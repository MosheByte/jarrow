---
title: FAQ
description: Is this legal? Why not Connector? Why is my mod red?
---

## Is this legal?

Converting a mod you downloaded, for your own game, is the use Jarrow is built for. Jarrow does not redistribute anything: it never stores, hosts or links to a converted JAR, and in Tier A your JAR never leaves your machine. Whether you may share a converted mod depends on that mod's license, which is the author's call. The converter will read the license and limit what it offers ("personal use only" for restrictive or unknown licenses), and an author's opt-out is honoured even when the license would allow conversion.

We are not lawyers, and this is not legal advice. Read the license of every mod you convert, and the [license policy](/docs/legal/).

## Why not Connector?

[Sinytra Connector](https://github.com/Sinytra/Connector), together with Forgified Fabric API, already lets players run many Fabric mods on NeoForge, and it is impressive work. Connector is a runtime compatibility layer: you install it alongside the original Fabric JAR, and it adapts the mod while the game loads.

Jarrow takes a different approach. It will convert the JAR itself, ahead of time, into a mod NeoForge loads natively with no extra layer, and it will check each conversion with a parity harness. That makes a different trade-off, not a better one for everyone: Connector works with the original file and keeps up with the mod's updates on its own, while a Jarrow conversion is a new file that has to be converted again for each update.

Jarrow contains no Connector code. Where Connector's public issue tracker documents a known difference between the two loaders, the rule tables will credit it in their sources.

## Why is my mod red?

Today no mod has a result: the harness and the board are not built yet (milestone M1). When they are, a red result will always carry a reason that comes from the harness itself, for example a class that was not found, a mixin that failed to apply, a registry conflict or a crash, and the report will show the evidence. Until the converter exists (milestone M2), every mod on the board will be red with the reason "no converter yet".

If you think a result is wrong, the `red-mod-report` issue form on [GitHub](https://github.com/MosheByte/jarrow/issues/new/choose) asks for the mod's URL and the report hash.

## Does Jarrow collect data?

No, nothing yet. See [Telemetry](/docs/telemetry/).
