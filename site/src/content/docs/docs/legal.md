---
title: Legal and license policy
description: How Jarrow treats mods, licenses and their authors, in plain words.
---

The people who can sink this project are mod authors, so this policy is written for them first. Nothing converts yet; this is the policy the converter, the board and author mode must follow from their first day, enforced in code and not only in text.

We are not lawyers. Read your own mod's license, and the licenses of the mods you convert.

## No redistribution, ever

Jarrow will never store, cache, host or link to a converted JAR.

- **Tier A** conversions happen in your browser. The JAR never leaves your machine, and the result exists only in your browser tab.
- **Tier B** conversions run in a short-lived job. The result is returned once, then the job deletes its workspace. Job logs must not contain decompiled source.
- **The site stores zero JAR bytes.** The public board links to each mod's own page and never to a converted JAR.

## Personal use by default

Converting a mod you downloaded, for your own game, is how the converter is presented. Publishing a conversion is the author's right. [Author mode](/authors/) exists to make that easy for the author and hard for everyone else.

## Licenses decide what you can do

The converter will read the license field of every mod's manifest:

| License | What Jarrow will do |
| --- | --- |
| Known permissive: MIT, Apache-2.0, LGPL, MPL, CC0, Unlicense | Convert |
| Known copyleft that allows modification: GPL | Convert, and the report says the result inherits the license |
| "All rights reserved", missing, or `SEE LICENSE IN` pointing at a restrictive file | Convert for personal use only, show a notice, and never offer the Gist or pull-request path unless the signed-in user owns the repository |

A license the converter cannot recognise gets the personal-use path only.

## Opt-out is absolute

An author who verifies ownership (GitHub sign-in matching the mod's repository, or a Modrinth-linked account) can mark their mod "do not port". The converter will refuse it in every direction, the board will hide it, and the refusal will explain why. This holds even when the license would allow conversion.

The verified flow arrives with author mode (milestone M4). Until then, authors can ask with the `author-opt-out` issue form on [GitHub](https://github.com/MosheByte/jarrow/issues/new/choose).

## Clean room

The engine learns each loader (Fabric, NeoForge, Forge and Quilt) from its public documentation and open-source API repositories. Rule tables describe API shapes, never mod code. No code from Sinytra Connector, Architectury, Forgified Fabric API or any mod is copied into the engine or the rule tables. Decompiled user code exists only inside a Tier B job and is never logged, stored, or sent anywhere outside that job.

## Trademarks

"Minecraft" belongs to Mojang. "Fabric", "NeoForge", "Forge" and "Quilt" belong to their projects. Jarrow is not an official Minecraft product, is not approved by or associated with Mojang or Microsoft, and is not affiliated with the Fabric, NeoForge, Forge or Quilt projects. The product name and logo do not use those names, and this site shows no loader logos.

## Licenses of this project

Jarrow's code is under the [MIT license](https://github.com/MosheByte/jarrow/blob/main/LICENSE). The rule tables are [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/), so anyone can reuse the knowledge in them.

## Reporting

- Security issues: see [SECURITY.md](https://github.com/MosheByte/jarrow/blob/main/SECURITY.md).
- A conversion that looks wrong: the `red-mod-report` issue form.
- An opt-out request: the `author-opt-out` issue form.

Both forms are on the [new issue page](https://github.com/MosheByte/jarrow/issues/new/choose).
