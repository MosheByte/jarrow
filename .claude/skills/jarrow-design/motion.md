# Motion and the scenes

Measurements: `docs/design-research/horizon.md`, section 8 (every animation in the reference) and section 3 (the canvas generators). Tokens: the "Motion", "Sky" and "Sun" groups in `site/tokens.css`. The keyframe and its classes: `site/src/styles/global.css`.

## What moves

| What | How | Tokens |
| --- | --- | --- |
| Hero text, eyebrow, buttons, card | `.jw-rise`: the `jw-rise` keyframe on load, staggered by `--delay` per element | `--jw-rise-duration` 0.9 s, `--jw-rise-ease` `cubic-bezier(0.2, 0.7, 0.2, 1)`, `--jw-rise-distance` 14 px |
| Sections below the fold | `.jw-reveal`: the same keyframe on the view timeline (`animation-range: entry 10% cover 30%`), CSS only, so it works without JavaScript | same |
| Bar fills | `clip-path: inset(0 calc(100% - var(--fill)) 0 0 round 2px)`, transition on input | `--jw-bar-duration` 0.45 s, ease-out |
| Links and buttons | colour or background alpha only | `--jw-hover-duration` 150 ms, `--jw-hover-ease` |
| The header | background and backdrop blur after scrolling | `--jw-header-duration` 300 ms |
| Scene canvases | fade in once the first frame is drawn, then loop at 32 fps or less while on screen | `--jw-fade-duration` 700 ms |
| A drop on the drop zone | the board card's bars refill; nothing else moves | `--jw-bar-duration` |

**What does not move:** the logo, the nav, the footer, headings after their entrance, numbers (a readout changes value without counting up), avatars, lists. Nothing auto-scrolls. Only the scene canvases loop.

## Rules

1. **One entrance keyframe.** `translateY(14px)` to 0 with opacity 0 to 1, 0.9 s, `cubic-bezier(0.2, 0.7, 0.2, 1)`, `backwards` fill, staggered by `--delay` (rule 20). No other entrance, no bounce, no scale-in, no slide from the side.
2. **Hover is colour.** Hover changes colour alpha over 150 ms (rule 21). The only hover transforms on the whole site: a 3 % scale on a game screenshot inside its rounded figure and a 4 px caption rise, both under `@media (prefers-reduced-motion: no-preference)`. A card never lifts.
3. **Reduced motion.** Under `prefers-reduced-motion: reduce`, every keyframe is off (`global.css` does it for `.jw-rise` and `.jw-reveal` and shortens every transition to nothing), and every canvas draws one static frame and attaches no pointer or scroll listener.

## The scenes

All illustration is drawn at runtime into `<canvas>` by Jarrow's own seeded generators, from the report's description in section 3 (never from the reference's code, its chunk or its assets):

- **Two layers.** Underneath, a smooth canvas at device scale: the sky gradient from `--jw-sky-*`, a radial glow around the sun, round stars. On top, a low-resolution `ImageData` buffer scaled up with `image-rendering: pixelated`: the sun disc from `--jw-sun-*`, ridges, clouds, sprites.
- **The pixel grid.** `px = clamp(round(width / 340), 3, 6)`: 3 px at 390, 4 px at 1440, 6 px from 2040 (rule 15). Cells stay whole; nothing is scaled to a fraction.
- **Seeded.** One seed, so every visitor sees the same picture; a `ResizeObserver` rebuilds the scene at the new size instead of scaling a bitmap.
- **Frame budget.** At most 32 fps (skip a frame when less than 31 ms have passed), only while the canvas is on screen (an `IntersectionObserver` stops the loop off screen).
- **Accessible.** Every canvas is `aria-hidden="true"`, fades in over 700 ms once its first frame is drawn (`data-ready`), and is static under reduced motion (rule 16). Without JavaScript the hero shows `--jw-sky-fallback`.
- **Colours.** Only the sky and sun ramps, the page colour, white and the tier colours where a scene shows a result. A generator never invents a colour; it may mix between two ramp stops.

The three scenes, each the Jarrow counterpart of one the report reads (section 3): the **sunrise** for the home hero and the releases page (the reference's hero scene turned to dawn: the sun at 63 % of the width under the hero card at 1440 and at 50 % at 390, a scanlined disc whose gaps drift, block-stepped value-noise ridges, stepped clouds lit from the sun); the **ember field** for the board (the reference's `PixelFire`: a dithered field densest at the bottom, on the amber ramp); the **aurora band** for the authors page and the community section (two wave bands with an 8×8 Bayer ordered dither, in the night end of the sky ramp). Section edges are the scene's own dithered falloff; there are no SVG wave dividers (rule 19). If your working copy has the private build specification, its section 8 has the composition of each scene.
