# jarrow

The npm package of [Jarrow](https://github.com/jarrow-dev/jarrow), a converter from Fabric to NeoForge for Minecraft mod JARs.

**Today this package contains only Jarrow's design tokens.** It has no JavaScript and no converter. The WASM build of the Jarrow engine, which will run conversions in the browser, arrives in milestone M3 ([roadmap](https://github.com/jarrow-dev/jarrow/blob/main/ROADMAP.md)). Until then, Jarrow converts nothing.

## What is in it

| File | Contents |
| --- | --- |
| `tokens.css` | CSS custom properties named `--jw-*`: dark values on `:root`, light values on `[data-theme='light']` |
| `tokens.json` | The same tokens as JSON: for each one its name, CSS property, role, dark value, light value and a usage note |

Both files are generated when the package is published, by `jarrow tokens` and `jarrow tokens --json` from [`crates/report`](https://github.com/jarrow-dev/jarrow/tree/main/crates/report). The Jarrow site's stylesheet comes from the same source.

Two rules come with the colours: `--jw-result-*` (green, yellow, red and none) mark harness results only and are never decorative, and `--jw-accent` is for actions such as buttons and links.

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
