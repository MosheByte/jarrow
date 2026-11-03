# jarrow

The npm package of [Jarrow](https://github.com/jarrow-dev/jarrow), a converter from Fabric to NeoForge for Minecraft mod JARs.

**Today this package contains only Jarrow's design tokens.** It has no JavaScript and no converter. The WASM build of the Jarrow engine, which will run conversions in the browser, arrives in milestone M3 ([roadmap](https://github.com/jarrow-dev/jarrow/blob/main/ROADMAP.md)). Until then, Jarrow converts nothing.

## What is in it

| File | Contents |
| --- | --- |
| `tokens.css` | CSS custom properties named `--jw-*`, all on `:root` with `color-scheme: dark` (the design is dark only): colours, the type scale, the 4 px spacing scale, layout, radii, card, button and motion values |
| `tokens.json` | The same tokens as JSON: for each one its name, CSS property, group, CSS value and a usage note |

Both files are generated when the package is published, by `jarrow tokens` and `jarrow tokens --json` from [`crates/report`](https://github.com/jarrow-dev/jarrow/tree/main/crates/report). The Jarrow site's `site/tokens.css` comes from the same source.

Two rules come with the colours: `--jw-tier-green`, `--jw-tier-amber` and `--jw-tier-red` mark result tiers and warnings only and are never decorative, and `--jw-accent` (amber, `#ffb454`) is for focus rings, links, pills, bar fills and the accent of a page heading. The values follow the measured design analysis in [`docs/design-research/horizon.md`](https://github.com/jarrow-dev/jarrow/blob/main/docs/design-research/horizon.md).

## Use

```sh
npm install jarrow@alpha
```

```css
@import "jarrow/tokens.css";
```

```js
import tokens from "jarrow/tokens.json" with { type: "json" };
```

This is a pre-release (0.1.0-alpha.1). Token names can change before 0.1.0.

MIT licensed.
