# Range input (the board card's filter)

Measured: `docs/design-research/horizon.md`, section 5 (the benchmark card: a real `<input type="range">` whose value sets the comparison), section 8 (bar fills), section 10 (the slider by keyboard; its focus ring, `home-1440-slider-focus.png`). Data: `docs/design-research/horizon/data/home-1440.json` (`sliders`) and `home-measure.json` (`slider`).

## What it is

A native `<input type="range">` in the board card that filters last night's counts by conversion direction (loader to loader). It is a real control: moving it changes the numbers and the bars. Never a decorative slider, never a custom div that imitates one.

## Markup

```html
<label class="sr-only" for="direction">Conversion direction</label>
<input id="direction" type="range" min="0" max="5" step="1" value="0"
       aria-valuetext="All directions">
<output for="direction" aria-live="polite"><!-- the count for the direction --> mods</output>
```

- `aria-valuetext` is the human value ("Fabric to NeoForge"), updated on every input; the `<output>` (or an `aria-live="polite"` region) announces the new count.
- The default value renders on the server, so without JavaScript the card shows the default counts and the input is present but inert (rule 30).

## Tokens

| Part | Value |
| --- | --- |
| Track | the bar itself, as in the reference, where the range input is drawn over the bar it sets (`home-1440-slider-focus.png`): 8 px tall, empty part `--jw-white-8`, radius `--jw-radius-2` |
| Filled part | `--jw-accent` for the filter's own bar; a count bar fills in its tier colour |
| Thumb | 6 × 20 px, `--jw-white`, radius `--jw-radius-2` (report section 10: the 6×20 px thumb) |
| Hit area | the input box 24 px tall (`h-6`), the full card width |
| Bars | `clip-path: inset(0 calc(100% - var(--fill)) 0 0 round 2px)`, transition `clip-path var(--jw-bar-duration) ease-out`; colour is the tier colour of the count |
| Readout | `--jw-number-size`, display 600, `tabular-nums`, unit in 14 px at white 50 % |
| Hint | 12 px, white 50 %: what the number is and what it is not |

## States

- **Focus-visible:** the thumb, not the track, gets `outline: 2px solid var(--jw-accent); outline-offset: 2px`; the track's own outline is removed (`:focus { outline: none }` on the input, the ring on `::-webkit-slider-thumb` and `::-moz-range-thumb` under `:focus-visible`).
- **Hover:** the thumb stays white; the cursor is `pointer`. No scale.
- **Disabled:** when there is no board yet the input is not rendered; the card says why in a sentence.
- **Reduced motion:** bar transitions are instant.

## Keyboard and touch

Tab reaches it after the hero buttons. Arrow keys step by one direction; Home and End go to the ends; each step updates `aria-valuetext`, the readout and the bars. On touch the whole 24 px box is the hit area.

## Copy

The hint says what the numbers are, from which run, and what they are not ("Last night's harness results for the top 1,000 mods. A green result is behaviour parity in the harness, not a review of the mod."). No promise words (`brand.md`).
