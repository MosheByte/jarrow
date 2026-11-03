# Legal and trust policy

This is how Jarrow treats mods, their authors and their licenses, in plain words. It is written for mod authors first, because they are the people this project has to earn trust from.

**We are not lawyers.** Neither the project owner nor the agent that builds Jarrow is a lawyer, and this page is not legal advice. If you are an author, read your own license: it decides what others may do with your mod. If a question of law decides whether a feature ships, the project stops and asks before shipping it.

Jarrow is pre-alpha (milestone M0). Nothing converts yet. The rules below are the policy the code will follow as each part is built. The license gate and the opt-out will be enforced in code, not only described here.

## What Jarrow does

Jarrow takes a Fabric mod JAR and produces a NeoForge JAR of the same mod, then checks with a harness that the two behave the same. Simple conversions (Tier A) run in your browser and your JAR never leaves your machine. Harder ones (Tier B) run in a short-lived GitHub Actions job for signed-in users.

## No redistribution

- The service never stores, caches, hosts or links to a converted JAR. Ever.
- Tier A conversions happen on your machine. The result exists only in your browser tab.
- Tier B conversions are ephemeral: your upload is deleted right after the job downloads it, the result is returned to you once, and the job's workspace is wiped. The Actions log never contains decompiled source.
- The public board links to each mod's original page. It never links to a converted JAR.
- The site stores zero JAR bytes.

## Personal use by default

Converting a mod you downloaded, for your own game, is how Jarrow is presented. Publishing a converted mod is the author's right, not ours and not yours. Author mode (milestone M4) exists to make publishing easy for authors and nobody else: an author signs in with GitHub, converts their own mod, and receives a pull request against their own repository.

## License policy

Jarrow will read the license field of every mod's manifest and act on it, in code (milestone M3 for the browser gate).

1. **Permissive licenses** (MIT, Apache-2.0, LGPL, MPL, CC0, Unlicense): Jarrow converts the mod.
2. **Copyleft licenses that allow modification** (GPL): Jarrow converts the mod, and the report says the result inherits the license.
3. **"All rights reserved", no license, or a pointer to a restrictive license file:** Jarrow converts for personal use only and shows a notice. It never offers to publish the result (as a Gist or a pull request) unless the signed-in user owns the mod's repository.
4. **A mod on the opt-out list** is refused everywhere, whatever its license.

## Opt-out is absolute

An author who proves they own a mod can choose one of two things:

- **Do not port.** The converter refuses the mod, the board hides it, and the refusal explains why. This is honoured even when the license would allow conversion.
- **Official port exists.** The board links to the author's own NeoForge version instead of showing a result.

How to opt out:

- **Now:** fill in the [author opt-out form](https://github.com/MosheByte/jarrow/issues/new?template=author-opt-out.yml) with your mod's page and proof that you own it, for example a link from the mod page or repository to the issue you opened. Opt-outs filed before the board exists are applied from its first run.
- **From milestone M4:** sign in on the site with GitHub. If your account matches the mod's repository, or your Modrinth-linked account, you can opt out in one click.

## Clean room

- The engine learns the Fabric and NeoForge APIs from their public documentation and their open-source API repositories, under their licenses.
- Rule tables describe API shapes (class names, methods, event timing, return values) and cite their sources. They never contain code from a mod.
- Jarrow does not copy code from Sinytra Connector, Architectury or Forgified Fabric API. Their public issue trackers may inform a rule about a known difference, and then the rule credits them.
- Decompiled code of a user's mod exists only inside a Tier B job. It is never logged, stored, or sent to a model outside that job's own context.

## Mojang's mappings and the Minecraft EULA

- Mojang's official mappings are downloaded from Mojang's version manifest when needed, under Mojang's license. They are never committed to this repository, and any cache of them stays private, never served publicly.
- Running a Minecraft server means accepting the Minecraft EULA. Only the project owner can accept it for the project, so the harness refuses to start a server until the owner has done so (see [spec-amendments.md](spec-amendments.md), A-6).

## Trademarks

"Minecraft" is a trademark of Mojang and Microsoft. "Fabric" and "NeoForge" belong to their projects. The name Jarrow, its logo and its domain do not use any of them. The site footer and the README carry this notice:

> Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft. Not affiliated with the Fabric or NeoForge projects.

Loader logos appear only where their own brand guidelines allow it; otherwise the site uses a text name.

## Agent authorship

Jarrow is built by Claude, an AI agent made by Anthropic, for the project owner. Every commit says so in a `Co-Authored-By` trailer, and so do the README and the site's About page. Hiding it would be dishonest.

## Licenses of this project

- The code is MIT ([LICENSE](../LICENSE)).
- The rule tables under `rules/` are CC0-1.0: anyone can reuse what they describe about the APIs.
- Fixture mods in `tests/fixtures/` are vendored only when their own license allows it, each with its LICENSE file beside it.

## Reporting

- Vulnerabilities: [SECURITY.md](../SECURITY.md).
- A conversion that looks wrong: the [red mod report](https://github.com/MosheByte/jarrow/issues/new?template=red-mod-report.yml) form.
- An author who wants their mod left alone: the [author opt-out](https://github.com/MosheByte/jarrow/issues/new?template=author-opt-out.yml) form.
