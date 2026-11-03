# Card

Measured: `docs/design-research/horizon.md`, section 2.3 (surfaces, borders, shadows, backdrop blur), section 5 (the benchmark card in the first screen; tell 8), section 10 (contrast on the card). Rule 12. Screenshots: `home-1440-first.png`, `home-1440-zoom-benchmark-card.png`.

## Anatomy

A rounded surface (`--jw-card-radius`, 16 px) with `--jw-card-padding` (24 px) or `--jw-card-padding-wide` (32 px), holding, top to bottom: an optional label (14 px, 500, white 60 %), a card h2 (`--jw-h2-card-size` 32 px, display 600, line height 1.1, −0.02 em), a body paragraph (15 px, white 60 %, 1.625), then the content (numbers, bars, a list, a control), then an optional hint (12 px, white 50 %).

## Tokens

| Part | Token |
| --- | --- |
| Surface | `--jw-card-surface` (white at 4 %); a quieter card `--jw-white-3`, a raised one up to `--jw-white-8` |
| Edge | `--jw-card-border: 0`; `--jw-card-shadow: none` |
| Radius | `--jw-card-radius` 16 px; panels 24 px (`--jw-radius-24`) |
| Padding | 24 px, 32 px on wide cards |
| Hero card blur | `backdrop-filter: blur(var(--jw-card-blur))` (24 px), hero card only |
| Numbers | `--jw-number-size` 40 px, display 600, `tabular-nums`, unit in 14 px at white 50 % |

## The hero card (the board card)

The home page's first screen has one card at the right of the copy, over the scene's sky: it is the board card. It holds last night's green, amber and red counts with their bars and a real range input that filters the counts (`components/range-input.md`), and the drop zone. Its blurred backdrop picks up the sky behind it; it ends above the sun (rule 17). It is `.jw-rise` with the latest `--delay` of the hero.

- Counts are figures with units (a count followed by "mods"), `tabular-nums`, with the bar in the tier colour (`--jw-tier-green`, `-amber`, `-red`) and the label in text, so colour is never the only signal.
- When there is no nightly yet, the card says so in a sentence and shows no bars ("No nightly run yet: the board fills after the first one."). It never shows sample numbers.

## States

- **Hover:** a card that is a link changes its surface alpha by one step (4 % to 8 %) over 150 ms. It never lifts, scales or gains a shadow (rule 21). A card that is not a link has no hover.
- **Focus-visible:** a link card shows the 2 px accent ring, offset 2 px, around the whole card.
- **Empty, loading, error:** a sentence in the card's body style that says what is missing and since when. Loading text is a sentence too ("Reading last night's board…"), with the server-rendered last value or a dash in its place, so the layout does not move.

## Do not

- A 1 px border, an outline, a drop shadow, a gradient fill or a coloured surface.
- Icons in tinted circles at the top of a card; three identical cards in a row as a feature grid (report section 5, tell 1).
- A card inside a card.
