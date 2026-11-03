---
title: Install
description: How the Jarrow CLI and npm package will be installed. Nothing is released yet.
---

:::note
The CLI is not released yet, and there is nothing useful to install today: no conversion code exists. The package name `jarrow` on crates.io and npm is pending: the claim waits for registry tokens that only the project owner can provide.
:::

## What is planned

The book plans nightly prereleases from milestone M1, a CLI nightly prerelease as an exit criterion of M2, and the first signed release, v0.1, in milestone M5. When releases exist, these will be the ways to get Jarrow:

| Channel | For | Planned command |
| --- | --- | --- |
| GitHub Releases | Prebuilt CLI binaries for Linux, macOS and Windows, signed with Sigstore, with an SBOM | Download from the releases page |
| crates.io | Building the CLI from source with Rust | `cargo install jarrow` |
| Homebrew, Scoop, winget, AUR | The CLI through your system's package manager | Published by the release workflow |
| npm | The WebAssembly engine, for using Jarrow from JavaScript and CI | `npm install jarrow` |

The commands above do not work yet. They are listed so you know what to look for.

## Planned CLI commands

- `jarrow convert`: convert a Fabric mod JAR into a NeoForge JAR, with a report.
- `jarrow check`: classify a JAR into a tier and list the reasons, without converting it.

Every public flag will be documented here when it exists.

## Use the site instead

The [converter page](/convert/) will run the same engine in your browser from milestone M3, with no install.
