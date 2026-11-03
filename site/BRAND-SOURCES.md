# Brand and icon sources

Every icon, logo and typeface on the site, where it comes from, and its license. Jarrow's rule: no generated icons, logos, illustrations or screenshots.

## Icons

| What | Source | License | Where it is used |
| --- | --- | --- | --- |
| UI icons (arrows, book, map, sun, moon, construction, calendar, external link) | [Lucide](https://lucide.dev) via the `@lucide/astro` package | [ISC](https://lucide.dev/license) | Buttons, notices and the theme toggle on the site pages |
| Favicon | Lucide [`arrow-right-left`](https://lucide.dev/icons/arrow-right-left), paths unmodified, stroke width 2.5 for small sizes, coloured with `--jw-text` (`public/favicon.svg`) | ISC | Browser tab |
| GitHub mark | [Simple Icons](https://simpleicons.org) via the `simple-icons` package (`siGithub`) | Package [CC0-1.0](https://github.com/simple-icons/simple-icons/blob/develop/LICENSE.md); the mark itself is GitHub's, used under [GitHub's logo guidelines](https://github.com/logos) to label links to the repository | Header, docs header, home page button |
| Docs interface icons (search, menu, theme select, chevrons) | Built into [Starlight](https://starlight.astro.build) | MIT | Docs pages only. They are part of the docs framework, not drawn for this project |

Lucide removed brand icons from its set, so the GitHub mark comes from Simple Icons, the project's source for brand logos.

## Loader and game logos: none

The site shows no Fabric, NeoForge, Minecraft, Mojang or Microsoft logo. Fabric and NeoForge are named in plain text only (text wordmarks).

Why:

- The project allows a loader logo only under that loader's published brand guidelines. A web search on 2026-11-03 found no published logo or brand guideline from either project, so the fallback applies: a text wordmark.
- Simple Icons 16.36.0 has no Fabric, NeoForge or Minecraft icon to fall back on (checked against the package's exports).
- A loader logo next to "Jarrow" could suggest an affiliation that does not exist. The footer says plainly that the project is not affiliated with Fabric, NeoForge, Mojang or Microsoft ([docs/legal.md](../docs/legal.md#trademarks)).

If either project publishes guidelines that allow this use, add the logo here first, with a link to the guideline, then on the site.

## The Jarrow wordmark

"Jarrow" is set as text in the display typeface. There is no logo image.

## Typefaces

Self-hosted through [Fontsource](https://fontsource.org); no font is loaded from a third-party server.

| Typeface | Package | License | Use |
| --- | --- | --- | --- |
| Bricolage Grotesque (variable) | `@fontsource-variable/bricolage-grotesque` | SIL Open Font License 1.1 | Headings, the wordmark, large numbers |
| Atkinson Hyperlegible Next (variable) | `@fontsource-variable/atkinson-hyperlegible-next` | SIL Open Font License 1.1 | Body text |
| JetBrains Mono (variable) | `@fontsource-variable/jetbrains-mono` | SIL Open Font License 1.1 | Code, labels |

## Third-party images

One: the GitHub Actions status badge for `ci.yml` on the home page, loaded from github.com. It is the repository's own CI status. Everything else on the site is text and CSS.

Mod icons on the board, once it has rows, will be hot-linked from each mod's Modrinth page with attribution and a link back, never copied into the repository.
