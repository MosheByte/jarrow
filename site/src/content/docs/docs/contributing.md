---
title: Contributing
description: How to contribute to Jarrow.
---

Jarrow is built in the open, mostly by Claude, an AI agent made by Anthropic; every commit it makes is co-authored and disclosed. Contributions from people are welcome, and they go through the same path as the agent's: a pull request that CI checks before it can merge.

Start with [CONTRIBUTING.md](https://github.com/MosheByte/jarrow/blob/main/CONTRIBUTING.md) on GitHub. It covers how to build and test, and what a pull request needs.

## Good places to start

- **Questions and ideas:** [GitHub Discussions](https://github.com/MosheByte/jarrow/discussions).
- **Bugs and wrong-looking conversions:** the issue forms on the [new issue page](https://github.com/MosheByte/jarrow/issues/new/choose).
- **Rule tables:** from milestone M2, rule tables under `rules/` describe how one loader's API calls map to another's, one directory per direction, starting with Fabric to NeoForge. They are CC0, and a rule proposal is a good first contribution once the format is documented.
- **The site and these docs:** the source is in `site/` of the repository. Every docs page has an edit link at the bottom.

## Ground rules

- Be polite to everyone, including the Sinytra, Fabric, NeoForge, Forge and Quilt maintainers. Their work is the ecosystem this project lives in. The project follows the [Contributor Covenant](https://github.com/MosheByte/jarrow/blob/main/CODE_OF_CONDUCT.md).
- Never commit a mod JAR, decompiled source or any user upload.
- Never copy code from Sinytra Connector, Architectury, Forgified Fabric API or any mod.
- Nothing is called "working" without a harness result.
