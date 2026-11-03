# Button

Measured: `docs/design-research/horizon.md`, section 2.3 (the button table, from `home-measure.json`) and section 5 (tell 10). Rules 13 and 21. Screenshot: `docs/design-research/horizon/home-1440-first.png` (the two hero buttons, the header button).

## Anatomy

`[icon 17 px] [10 px gap] [label]`, one line, vertically centred. An icon is optional; when present it is a UI glyph (never a brand mark standing in for a brand name) or the official mark of the destination brand (`brand.md`).

## Variants

| Variant | Height | Padding x | Radius | Type | Background | Text | Hover |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Primary (hero) | `--jw-button-hero-height` 48 px | `--jw-button-hero-padding` 20 px | `--jw-button-hero-radius` 10 px | `--jw-button-hero-size` 15 px, 500 | `--jw-primary` | `--jw-on-primary` | `--jw-primary-hover` |
| Secondary (hero) | 48 px | 20 px | 10 px | 15 px, 500 | `--jw-white-8` | `--jw-white` | `--jw-white-12` |
| Page | `--jw-button-height` 40 px | `--jw-button-padding` 16 px | `--jw-button-radius` 8 px | `--jw-button-size` 14 px, 500 | `--jw-white-8` | `--jw-white` | `--jw-white-12` |
| White (page) | 40 px | 16 px | 8 px | 14 px, 500 | `--jw-white` | `--jw-page` | `--jw-white-90` |
| Header | `--jw-button-header-height` 36 px | 16 px | 8 px | 14 px, 500 | `--jw-white-10` | `--jw-white` | `--jw-white-15` |
| Header text link | 36 px | 12 px | none | 14 px, 400 | none | `--jw-white-65` | `--jw-white` |
| Pill toggle | `--jw-button-pill-height` 32 px | 12 px | 6 px | 14 px, 500 | `--jw-accent` when on, `--jw-white-8` when off | `--jw-on-primary` on, `--jw-white-80` off | off: `--jw-white-12` |
| Hamburger | 36 × 36 px | none | 8 px | icon at `--jw-white-80` | `--jw-white-6` | | `--jw-white-10` |

`site/src/styles/site.css` carries `.btn`, `.btn-primary` and `.btn-ghost` (the secondary) on these tokens.

## States

- **Default:** as the table. No border, no shadow, no gradient, no glow.
- **Hover:** the background (or text) colour changes, over `--jw-hover-duration` (150 ms) with `--jw-hover-ease`. Nothing moves: no translate, no scale, no shadow (rule 21).
- **Focus-visible:** `outline: 2px solid var(--jw-accent); outline-offset: 2px`. Never removed without a replacement.
- **Active:** the hover colour; no transform.
- **Disabled:** a disabled button is a sentence that says why it is not there ("The converter is not built yet (milestone M3)."), not a greyed control. If a control must stay visible, `aria-disabled="true"`, text at `--jw-white-50` on `--jw-white-4`, and the reason next to it.
- **Loading:** the label says what is happening ("Converting…"), the width does not jump (`min-width` of the idle label), and `aria-busy="true"` is on the region being updated.

## Interaction

- A button that navigates is an `<a>`; one that acts is a `<button type="button">`. Both reach 40 px or more in height for touch at 390; the 36 px header buttons sit in a 72 px header row.
- Keyboard: Enter (and Space for `<button>`) activates; the focus ring shows only on keyboard focus (`:focus-visible`).
- Buttons and brand-mark links (GitHub, Discord) do not carry the outbound arrow; text links in body copy do (rule 23). An outbound button still carries the visually hidden "(opens in a new tab)" when it opens one.

## Responsive

At 390 the hero buttons stack full width with a 12 px gap (`home-390-first.png`); the header keeps one 36 px button and the hamburger.

## Copy

Sentence case, a verb and an object: "Read the docs", "Open the board", "View on GitHub". Never "Learn more", "Get started", "Click here". No uppercase.
