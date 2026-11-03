//! Design tokens: the single source of the site's colours, type, spacing and layout, and of
//! the badge colours.
//!
//! `site/tokens.css` is generated from [`css`] and checked against it by the `tokens_current`
//! test on every CI run. To change a token, edit [`TOKENS`] and run
//!
//! ```text
//! JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current
//! ```
//!
//! The values come from the measured analysis of playhorizon.dev in
//! `docs/design-research/horizon.md`, section 2. Jarrow substitutes four things: the page is
//! night blue ([`PAGE`]), the accent is amber ([`ACCENT`], [`PRIMARY`]), the scene is a sunrise
//! instead of a sunset, and the sky and sun ramps ([`SKY`], [`SUN`]) are amber. Every other
//! value (the white alpha text steps, the surface alphas, the type scale, the 4 px spacing, the
//! 1152 px container, the 72 px header, the radii, no borders, no shadows, the buttons) is
//! Horizon's, verbatim.
//!
//! The site is dark only. Green, amber and red ([`TIER_GREEN`], [`TIER_AMBER`], [`TIER_RED`])
//! mark result tiers (the board's green, amber and red, and the badges) and warnings, and are
//! never decorative. The brand amber ([`ACCENT`], `#ffb454`) and the tier amber ([`TIER_AMBER`],
//! `#f2c46b`) are different colours and are never swapped.
//!
//! [`json`] renders the same tokens as JSON. `jarrow tokens` prints the CSS and
//! `jarrow tokens --json` the JSON; the npm package `jarrow` ships both files.

use std::fmt;
use std::fmt::Write as _;

use crate::{Outcome, Tier};

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Builds a colour from a `0xRRGGBB` literal.
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xff) as u8,
            g: ((rgb >> 8) & 0xff) as u8,
            b: (rgb & 0xff) as u8,
        }
    }

    /// WCAG 2.x relative luminance, from 0.0 (black) to 1.0 (white).
    pub fn relative_luminance(self) -> f64 {
        fn channel(c: u8) -> f64 {
            let c = f64::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// This colour at `percent` opacity drawn over `background`, the way the report measures
    /// text: composited, then read back as an opaque colour (rounded per channel).
    pub fn over(self, percent: u8, background: Rgb) -> Rgb {
        let a = f64::from(percent.min(100)) / 100.0;
        let mix = |top: u8, bottom: u8| -> u8 {
            let value = f64::from(top) * a + f64::from(bottom) * (1.0 - a);
            // Clamped to 0..=255 first, so the cast cannot truncate.
            value.round().clamp(0.0, 255.0) as u8
        };
        Rgb {
            r: mix(self.r, background.r),
            g: mix(self.g, background.g),
            b: mix(self.b, background.b),
        }
    }
}

impl fmt::Display for Rgb {
    /// Lower-case `#rrggbb`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// WCAG 2.x contrast ratio between two colours, from 1.0 to 21.0.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.relative_luminance(), b.relative_luminance());
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

/// The smallest contrast any text and background pair on the site may have (rule 29 of the
/// report: every pair measures at least 4.9:1, composited the way the report measures it).
pub const MIN_TEXT_CONTRAST: f64 = 4.9;

/// One stop of a gradient: a colour and an optional position in percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stop {
    /// The stop's colour.
    pub color: Rgb,
    /// Its position, in percent of the gradient line; `None` lets CSS place it.
    pub at: Option<u8>,
}

const fn stop(color: Rgb, at: Option<u8>) -> Stop {
    Stop { color, at }
}

/// The value a token is written with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Value {
    /// An opaque colour, written `#rrggbb`.
    Color(Rgb),
    /// A colour at an opacity in whole percent, written `rgb(255 255 255 / 0.6)`.
    Alpha(Rgb, u8),
    /// A linear gradient: the direction (`90deg`, `to bottom`) and its stops.
    Gradient(&'static str, &'static [Stop]),
    /// Any other CSS value, verbatim: a length, a font stack, a duration, an easing.
    Css(&'static str),
}

impl Value {
    /// The opaque colour of a [`Value::Color`], if it is one.
    pub const fn color(self) -> Option<Rgb> {
        match self {
            Value::Color(rgb) => Some(rgb),
            _ => None,
        }
    }

    /// The colour and opacity of a colour value: 100 for [`Value::Color`].
    pub const fn color_with_alpha(self) -> Option<(Rgb, u8)> {
        match self {
            Value::Color(rgb) => Some((rgb, 100)),
            Value::Alpha(rgb, percent) => Some((rgb, percent)),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Color(rgb) => write!(f, "{rgb}"),
            Value::Alpha(rgb, percent) => {
                let alpha = f64::from(*percent) / 100.0;
                write!(f, "rgb({} {} {} / {alpha})", rgb.r, rgb.g, rgb.b)
            }
            Value::Gradient(direction, stops) => {
                write!(f, "linear-gradient({direction}")?;
                for s in *stops {
                    match s.at {
                        Some(at) => write!(f, ", {} {at}%", s.color)?,
                        None => write!(f, ", {}", s.color)?,
                    }
                }
                f.write_str(")")
            }
            Value::Css(css) => f.write_str(css),
        }
    }
}

/// The groups the tokens are written in, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    /// The page colour and its alpha.
    Page,
    /// White and its alpha steps for text, the floor of information at 50 %.
    Text,
    /// White at 3 to 20 % over the page: every surface, rule and hover step.
    Surface,
    /// Buttons, links, focus rings, pills, bar fills and the h1 accent.
    Action,
    /// Result tiers (green, amber, red) and warnings; never decorative.
    Tier,
    /// The scene canvases only: the 15-stop sky ramp and its no-JavaScript fallback.
    Sky,
    /// The scene canvases only: the 7-stop sun ramp.
    Sun,
    /// Families, sizes, weights, line heights and tracking.
    Type,
    /// The 4 px spacing scale.
    Space,
    /// Container, gutters, header and docs rails.
    Layout,
    /// Corner radii.
    Radius,
    /// Cards: no border, no shadow.
    Card,
    /// Button heights, paddings, radii and type.
    Button,
    /// Durations and easings.
    Motion,
}

impl Group {
    /// Every group, in the order they are written to the CSS.
    pub const ALL: [Group; 14] = [
        Group::Page,
        Group::Text,
        Group::Surface,
        Group::Action,
        Group::Tier,
        Group::Sky,
        Group::Sun,
        Group::Type,
        Group::Space,
        Group::Layout,
        Group::Radius,
        Group::Card,
        Group::Button,
        Group::Motion,
    ];

    /// Lower-case name, as written to the JSON: `page`, `text`, `surface`, ...
    pub const fn as_str(self) -> &'static str {
        match self {
            Group::Page => "page",
            Group::Text => "text",
            Group::Surface => "surface",
            Group::Action => "action",
            Group::Tier => "tier",
            Group::Sky => "sky",
            Group::Sun => "sun",
            Group::Type => "type",
            Group::Space => "space",
            Group::Layout => "layout",
            Group::Radius => "radius",
            Group::Card => "card",
            Group::Button => "button",
            Group::Motion => "motion",
        }
    }

    /// The comment line written above the group.
    pub const fn heading(self) -> &'static str {
        match self {
            Group::Page => "Colour: the page (dark only)",
            Group::Text => {
                "Colour: text, white and its alpha steps; nothing below 50 % carries information"
            }
            Group::Surface => "Colour: surfaces, white over the page; no borders, no shadows",
            Group::Action => {
                "Colour: actions (buttons, links, focus rings, pills, bar fills, the h1 accent)"
            }
            Group::Tier => "Colour: result tiers and warnings only; never decorative",
            Group::Sky => "Colour: the sky ramp, for the scene canvases only",
            Group::Sun => "Colour: the sun ramp, for the scene canvases only",
            Group::Type => {
                "Type: Bricolage Grotesque 600 for display, Geist for text, Geist Mono for code"
            }
            Group::Space => "Space: a 4 px unit; every length is a multiple of it",
            Group::Layout => "Layout",
            Group::Radius => "Radius",
            Group::Card => "Cards: surface alpha, no border, no shadow",
            Group::Button => "Buttons",
            Group::Motion => {
                "Motion: one entrance keyframe, colour-only hover, the scenes are the only loops"
            }
        }
    }
}

/// One design token: a CSS custom property, its value, its group and what it is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// Custom property name without the leading `--`, for example `jw-page`.
    pub name: &'static str,
    /// The value written to the CSS.
    pub value: Value,
    /// The group it is written in.
    pub group: Group,
    /// Short usage note, written next to the property in the CSS.
    pub usage: &'static str,
}

impl Token {
    /// The CSS custom property, for example `--jw-page`.
    pub fn property(&self) -> String {
        format!("--{}", self.name)
    }

    /// `var(--name)`.
    pub fn var(&self) -> String {
        format!("var(--{})", self.name)
    }
}

const fn token(name: &'static str, value: Value, group: Group, usage: &'static str) -> Token {
    Token {
        name,
        value,
        group,
        usage,
    }
}

const WHITE_RGB: Rgb = Rgb::hex(0xffffff);
const PAGE_RGB: Rgb = Rgb::hex(0x0b0f1f);
const ACCENT_RGB: Rgb = Rgb::hex(0xffb454);
const ACCENT_LIGHT_RGB: Rgb = Rgb::hex(0xffe3b0);
const ACCENT_DEEP_RGB: Rgb = Rgb::hex(0xff8a3d);

const fn white(name: &'static str, percent: u8, group: Group, usage: &'static str) -> Token {
    token(name, Value::Alpha(WHITE_RGB, percent), group, usage)
}

const fn color(name: &'static str, rgb: u32, group: Group, usage: &'static str) -> Token {
    token(name, Value::Color(Rgb::hex(rgb)), group, usage)
}

const fn css_value(
    name: &'static str,
    css: &'static str,
    group: Group,
    usage: &'static str,
) -> Token {
    token(name, Value::Css(css), group, usage)
}

/// The 15-stop sky ramp, top of the sky to the horizon.
pub const SKY: [Rgb; 15] = [
    Rgb::hex(0x0b0f1f),
    Rgb::hex(0x11173a),
    Rgb::hex(0x1a1e4a),
    Rgb::hex(0x2a2158),
    Rgb::hex(0x3d2a66),
    Rgb::hex(0x55326f),
    Rgb::hex(0x713b70),
    Rgb::hex(0x8f466c),
    Rgb::hex(0xad5262),
    Rgb::hex(0xc66055),
    Rgb::hex(0xdc7447),
    Rgb::hex(0xee8d44),
    Rgb::hex(0xf8a74a),
    Rgb::hex(0xffbf5a),
    Rgb::hex(0xffd27a),
];

/// The 7-stop sun ramp, the top of the disc to its bottom.
pub const SUN: [Rgb; 7] = [
    Rgb::hex(0xfff8dc),
    Rgb::hex(0xffeeb0),
    Rgb::hex(0xffdd88),
    Rgb::hex(0xffc862),
    Rgb::hex(0xffb454),
    Rgb::hex(0xff9a48),
    Rgb::hex(0xff7a3d),
];

/// The CSS sky behind the canvas, shown before the first frame and without JavaScript: the
/// stops Horizon's fallback takes from its ramp (0, 2, 5, 9 at 0, 38, 62, 80 %), ending in the
/// page colour at 86 %.
const SKY_FALLBACK_STOPS: [Stop; 5] = [
    stop(SKY[0], Some(0)),
    stop(SKY[2], Some(38)),
    stop(SKY[5], Some(62)),
    stop(SKY[9], Some(80)),
    stop(PAGE_RGB, Some(86)),
];

const ACCENT_GRADIENT_STOPS: [Stop; 3] = [
    stop(ACCENT_LIGHT_RGB, None),
    stop(ACCENT_RGB, Some(55)),
    stop(ACCENT_DEEP_RGB, None),
];

/// Page background, `theme-color`, text on the accent pill.
pub const PAGE: Token = token(
    "jw-page",
    Value::Color(PAGE_RGB),
    Group::Page,
    "page background, theme-color, text on light pills",
);
/// The scrolled header: the page at 75 % under a 12 px backdrop blur.
pub const PAGE_75: Token = token(
    "jw-page-75",
    Value::Alpha(PAGE_RGB, 75),
    Group::Page,
    "header after scrolling, under a 12 px backdrop blur; dialog backdrops",
);
/// Body text default (Horizon's `--foreground`).
pub const FG: Token = color(
    "jw-fg",
    0xf4f3ff,
    Group::Text,
    "body text default, code text",
);
/// Headings, primary labels, display numbers.
pub const WHITE: Token = token(
    "jw-white",
    Value::Color(WHITE_RGB),
    Group::Text,
    "headings, primary labels, display numbers",
);
/// White button hover.
pub const WHITE_90: Token = white("jw-white-90", 90, Group::Text, "white button hover");
/// Footer headings and the footer's disclaimer line.
pub const WHITE_80: Token = white(
    "jw-white-80",
    80,
    Group::Text,
    "footer headings and disclaimer, hamburger icon",
);
/// Docs body text.
pub const WHITE_72: Token = white("jw-white-72", 72, Group::Text, "docs body text");
/// Hero lede, footer links.
pub const WHITE_70: Token = white("jw-white-70", 70, Group::Text, "hero lede, footer links");
/// Nav links.
pub const WHITE_65: Token = white(
    "jw-white-65",
    65,
    Group::Text,
    "nav links, header GitHub link",
);
/// Card and feature body, eyebrows and labels.
pub const WHITE_60: Token = white(
    "jw-white-60",
    60,
    Group::Text,
    "card and feature body, eyebrows, labels",
);
/// Hints, footer paragraphs, units beside numbers: the lowest step that carries information.
pub const WHITE_50: Token = white(
    "jw-white-50",
    50,
    Group::Text,
    "hints, footer paragraphs, units; the lowest step that carries information",
);
/// Decorative text only.
pub const WHITE_45: Token = white(
    "jw-white-45",
    45,
    Group::Text,
    "decorative only, never information",
);
/// Decorative index numbers.
pub const WHITE_40: Token = white(
    "jw-white-40",
    40,
    Group::Text,
    "decorative only: index numbers beside a label",
);
/// Every text step that carries information, strongest first.
pub const TEXT_STEPS: [Token; 9] = [
    WHITE, FG, WHITE_90, WHITE_80, WHITE_72, WHITE_70, WHITE_65, WHITE_60, WHITE_50,
];

/// The surfaces a card sits on: white at 4 % over the page.
pub const WHITE_4: Token = white(
    "jw-white-4",
    4,
    Group::Surface,
    "cards (the hero card), code blocks, rows",
);
/// Secondary button, footer rule, rules between rows.
pub const WHITE_8: Token = white(
    "jw-white-8",
    8,
    Group::Surface,
    "secondary button, footer rule, rules between rows, selected items",
);

/// Every token, in the order they are written to the CSS.
pub const TOKENS: &[Token] = &[
    // Page.
    PAGE,
    PAGE_75,
    // Text.
    WHITE,
    FG,
    WHITE_90,
    WHITE_80,
    WHITE_72,
    WHITE_70,
    WHITE_65,
    WHITE_60,
    WHITE_50,
    WHITE_45,
    WHITE_40,
    // Surfaces.
    white(
        "jw-white-3",
        3,
        Group::Surface,
        "rows, the quietest surface",
    ),
    WHITE_4,
    white("jw-white-5", 5, Group::Surface, "surface step"),
    white("jw-white-6", 6, Group::Surface, "hamburger button"),
    white("jw-white-7", 7, Group::Surface, "inline code background"),
    WHITE_8,
    white("jw-white-9", 9, Group::Surface, "surface step"),
    white(
        "jw-white-10",
        10,
        Group::Surface,
        "header button, hamburger hover",
    ),
    white("jw-white-12", 12, Group::Surface, "secondary button hover"),
    white("jw-white-14", 14, Group::Surface, "surface step"),
    white(
        "jw-white-15",
        15,
        Group::Surface,
        "header button hover, blockquote rule",
    ),
    white(
        "jw-white-20",
        20,
        Group::Surface,
        "underline of footer links",
    ),
    // Actions.
    PRIMARY,
    PRIMARY_HOVER,
    ON_PRIMARY,
    ACCENT,
    token(
        "jw-accent-45",
        Value::Alpha(ACCENT_RGB, 45),
        Group::Action,
        "glow of the pill and live dots",
    ),
    token(
        "jw-accent-light",
        Value::Color(ACCENT_LIGHT_RGB),
        Group::Action,
        "first stop of the accent gradient",
    ),
    token(
        "jw-accent-deep",
        Value::Color(ACCENT_DEEP_RGB),
        Group::Action,
        "last stop of the accent gradient",
    ),
    ACCENT_GRADIENT,
    // Tiers.
    TIER_GREEN,
    TIER_AMBER,
    TIER_RED,
    // Sky.
    color("jw-sky-0", 0x0b0f1f, Group::Sky, "top of the sky"),
    color("jw-sky-1", 0x11173a, Group::Sky, "sky"),
    color("jw-sky-2", 0x1a1e4a, Group::Sky, "sky"),
    color("jw-sky-3", 0x2a2158, Group::Sky, "sky"),
    color("jw-sky-4", 0x3d2a66, Group::Sky, "sky"),
    color("jw-sky-5", 0x55326f, Group::Sky, "sky"),
    color("jw-sky-6", 0x713b70, Group::Sky, "sky"),
    color("jw-sky-7", 0x8f466c, Group::Sky, "sky"),
    color("jw-sky-8", 0xad5262, Group::Sky, "sky"),
    color("jw-sky-9", 0xc66055, Group::Sky, "sky"),
    color("jw-sky-10", 0xdc7447, Group::Sky, "sky"),
    color("jw-sky-11", 0xee8d44, Group::Sky, "sky"),
    color("jw-sky-12", 0xf8a74a, Group::Sky, "sky"),
    color("jw-sky-13", 0xffbf5a, Group::Sky, "sky"),
    color("jw-sky-14", 0xffd27a, Group::Sky, "the horizon"),
    token(
        "jw-sky-fallback",
        Value::Gradient("to bottom", &SKY_FALLBACK_STOPS),
        Group::Sky,
        "behind the canvas: before the first frame and without JavaScript",
    ),
    // Sun.
    color("jw-sun-0", 0xfff8dc, Group::Sun, "top of the disc"),
    color("jw-sun-1", 0xffeeb0, Group::Sun, "sun"),
    color("jw-sun-2", 0xffdd88, Group::Sun, "sun"),
    color("jw-sun-3", 0xffc862, Group::Sun, "sun"),
    color("jw-sun-4", 0xffb454, Group::Sun, "sun"),
    color("jw-sun-5", 0xff9a48, Group::Sun, "sun"),
    color("jw-sun-6", 0xff7a3d, Group::Sun, "bottom of the disc"),
    // Type: families.
    css_value(
        "jw-font-display",
        "'Bricolage Grotesque Variable', 'Bricolage Grotesque Fallback', sans-serif",
        Group::Type,
        "h1, h2, display numbers; weight 600",
    ),
    css_value(
        "jw-font-body",
        "'Geist Variable', 'Geist Fallback', sans-serif",
        Group::Type,
        "everything else",
    ),
    css_value(
        "jw-font-mono",
        "'Geist Mono Variable', 'Geist Mono Fallback', monospace",
        Group::Type,
        "code, pre, docs inline code",
    ),
    css_value(
        "jw-weight-display",
        "600",
        Group::Type,
        "every display heading and number",
    ),
    css_value(
        "jw-weight-medium",
        "500",
        Group::Type,
        "h3, labels, eyebrows, buttons",
    ),
    css_value(
        "jw-weight-strong",
        "600",
        Group::Type,
        "docs h3, pill counts",
    ),
    // Type: the page h1 (60 px at 1440, 36 px at 390, inside a 576 px inline-size container).
    css_value(
        "jw-h1-size",
        "clamp(2rem, 10.4cqi, 4.75rem)",
        Group::Type,
        "page h1; its parent is an inline-size container",
    ),
    css_value("jw-h1-leading", "1.02", Group::Type, "page h1"),
    css_value("jw-h1-tracking", "-0.035em", Group::Type, "page h1"),
    // Type: section h2 (48 px at 1440, 32 px at 390).
    css_value(
        "jw-h2-size",
        "clamp(2rem, 3.6vw, 3rem)",
        Group::Type,
        "section h2",
    ),
    css_value("jw-h2-leading", "1.08", Group::Type, "section h2"),
    css_value("jw-h2-tracking", "-0.03em", Group::Type, "section h2"),
    // Type: card h2.
    css_value("jw-h2-card-size", "2rem", Group::Type, "card h2"),
    css_value("jw-h2-card-leading", "1.1", Group::Type, "card h2"),
    css_value("jw-h2-card-tracking", "-0.02em", Group::Type, "card h2"),
    // Type: docs.
    css_value(
        "jw-docs-h1-size",
        "clamp(2rem, 4.5vw, 3rem)",
        Group::Type,
        "docs article h1: 48 px at 1440, 32 px at 390",
    ),
    css_value("jw-docs-h1-leading", "1.08", Group::Type, "docs article h1"),
    css_value(
        "jw-docs-h1-tracking",
        "-0.03em",
        Group::Type,
        "docs article h1",
    ),
    css_value("jw-docs-h2-size", "1.625rem", Group::Type, "docs h2"),
    css_value("jw-docs-h2-leading", "1.2", Group::Type, "docs h2"),
    css_value("jw-docs-h2-tracking", "-0.02em", Group::Type, "docs h2"),
    css_value(
        "jw-docs-h3-size",
        "1.1875rem",
        Group::Type,
        "docs h3, Geist 600",
    ),
    css_value("jw-docs-h3-leading", "1.3", Group::Type, "docs h3"),
    css_value("jw-docs-h3-tracking", "-0.01em", Group::Type, "docs h3"),
    css_value("jw-docs-body-size", "1rem", Group::Type, "docs body"),
    css_value("jw-docs-body-leading", "1.7", Group::Type, "docs body"),
    // Type: feature h3.
    css_value(
        "jw-h3-size",
        "1.125rem",
        Group::Type,
        "feature h3, Geist 500",
    ),
    css_value("jw-h3-leading", "1.75rem", Group::Type, "feature h3"),
    // Type: numbers.
    css_value(
        "jw-number-size",
        "2.5rem",
        Group::Type,
        "a number in a card",
    ),
    css_value(
        "jw-number-xl-size",
        "clamp(6rem, 18vw, 11rem)",
        Group::Type,
        "the one big number of a section",
    ),
    css_value("jw-number-leading", "1", Group::Type, "display numbers"),
    css_value(
        "jw-number-tracking",
        "-0.02em",
        Group::Type,
        "display numbers",
    ),
    // Type: text.
    css_value("jw-lede-size", "1.125rem", Group::Type, "hero lede"),
    css_value(
        "jw-body-size",
        "0.9375rem",
        Group::Type,
        "card and feature body",
    ),
    css_value(
        "jw-body-leading",
        "1.625",
        Group::Type,
        "lede, body, small, footer",
    ),
    css_value(
        "jw-label-size",
        "0.875rem",
        Group::Type,
        "labels, eyebrows, nav, footer; never uppercase",
    ),
    css_value(
        "jw-label-leading",
        "1.25rem",
        Group::Type,
        "labels, eyebrows, nav, footer headings",
    ),
    css_value(
        "jw-small-size",
        "0.75rem",
        Group::Type,
        "hints; nothing at 50 % white is smaller",
    ),
    css_value(
        "jw-rail-label-size",
        "0.8125rem",
        Group::Type,
        "docs rail group labels, weight 500, at 50 % white",
    ),
    css_value(
        "jw-pill-size",
        "0.625rem",
        Group::Type,
        "pill counts, weight 600, line height 1",
    ),
    // Type: code.
    css_value("jw-code-size", "0.84em", Group::Type, "inline code"),
    css_value(
        "jw-code-padding",
        "0.12em 0.38em",
        Group::Type,
        "inline code",
    ),
    css_value(
        "jw-pre-size",
        "0.84375rem",
        Group::Type,
        "code blocks (13.5 px)",
    ),
    css_value("jw-pre-leading", "1.65", Group::Type, "code blocks"),
    css_value("jw-pre-padding", "1em 1.15em", Group::Type, "code blocks"),
    // Space.
    css_value("jw-space", "0.25rem", Group::Space, "the unit: 4 px"),
    css_value("jw-space-1", "0.25rem", Group::Space, "4 px"),
    css_value("jw-space-2", "0.5rem", Group::Space, "8 px"),
    css_value("jw-space-3", "0.75rem", Group::Space, "12 px"),
    css_value("jw-space-4", "1rem", Group::Space, "16 px"),
    css_value("jw-space-5", "1.25rem", Group::Space, "20 px"),
    css_value("jw-space-6", "1.5rem", Group::Space, "24 px"),
    css_value("jw-space-8", "2rem", Group::Space, "32 px"),
    css_value("jw-space-10", "2.5rem", Group::Space, "40 px"),
    css_value("jw-space-12", "3rem", Group::Space, "48 px"),
    css_value("jw-space-16", "4rem", Group::Space, "64 px"),
    css_value("jw-space-18", "4.5rem", Group::Space, "72 px"),
    css_value("jw-space-24", "6rem", Group::Space, "96 px"),
    // Layout.
    css_value(
        "jw-container",
        "72rem",
        Group::Layout,
        "content column: 1152 px, centred",
    ),
    css_value(
        "jw-gutter",
        "1.5rem",
        Group::Layout,
        "24 px gutters below 1024 px",
    ),
    css_value(
        "jw-gutter-wide",
        "2.5rem",
        Group::Layout,
        "40 px gutters from 1024 px (64rem)",
    ),
    css_value(
        "jw-header-height",
        "4.5rem",
        Group::Layout,
        "72 px, fixed, transparent at the top",
    ),
    css_value(
        "jw-header-blur",
        "12px",
        Group::Layout,
        "backdrop blur of the scrolled header",
    ),
    css_value(
        "jw-card-blur",
        "24px",
        Group::Layout,
        "backdrop blur of the hero card",
    ),
    css_value(
        "jw-docs-rail",
        "13rem",
        Group::Layout,
        "docs left rail: 208 px",
    ),
    css_value(
        "jw-docs-article",
        "37.5rem",
        Group::Layout,
        "docs article column: 600 px",
    ),
    css_value(
        "jw-docs-toc",
        "11rem",
        Group::Layout,
        "docs \"On this page\" rail: 176 px",
    ),
    // Radius.
    css_value(
        "jw-radius-2",
        "2px",
        Group::Radius,
        "stat bars, slider thumb",
    ),
    css_value(
        "jw-radius-3",
        "3px",
        Group::Radius,
        "legend and score squares",
    ),
    css_value("jw-radius-5", "5px", Group::Radius, "inline code"),
    css_value(
        "jw-radius-6",
        "6px",
        Group::Radius,
        "code copy button, pill toggle",
    ),
    css_value(
        "jw-radius-8",
        "8px",
        Group::Radius,
        "header and 40 px buttons, hamburger",
    ),
    css_value("jw-radius-10", "10px", Group::Radius, "48 px hero buttons"),
    css_value(
        "jw-radius-12",
        "12px",
        Group::Radius,
        "code blocks, tables, dropdowns, blockquotes",
    ),
    css_value(
        "jw-radius-16",
        "16px",
        Group::Radius,
        "cards, figures, panels",
    ),
    css_value("jw-radius-24", "24px", Group::Radius, "large panels"),
    css_value("jw-radius-28", "28px", Group::Radius, "the one light panel"),
    css_value("jw-radius-full", "9999px", Group::Radius, "pills, avatars"),
    // Cards.
    css_value(
        "jw-card-surface",
        "var(--jw-white-4)",
        Group::Card,
        "white at 4 % over the page",
    ),
    css_value("jw-card-radius", "16px", Group::Card, "every card"),
    css_value(
        "jw-card-padding",
        "1.5rem",
        Group::Card,
        "24 px; 32 px on wide cards",
    ),
    css_value("jw-card-padding-wide", "2rem", Group::Card, "32 px"),
    css_value(
        "jw-card-border",
        "0",
        Group::Card,
        "no border: separation is by surface alpha",
    ),
    css_value("jw-card-shadow", "none", Group::Card, "no drop shadow"),
    // Buttons.
    css_value(
        "jw-button-hero-height",
        "3rem",
        Group::Button,
        "hero buttons: 48 px",
    ),
    css_value(
        "jw-button-hero-padding",
        "1.25rem",
        Group::Button,
        "20 px each side",
    ),
    css_value(
        "jw-button-hero-radius",
        "10px",
        Group::Button,
        "hero buttons",
    ),
    css_value(
        "jw-button-hero-size",
        "0.9375rem",
        Group::Button,
        "15 px, weight 500",
    ),
    css_value(
        "jw-button-height",
        "2.5rem",
        Group::Button,
        "page buttons: 40 px",
    ),
    css_value(
        "jw-button-padding",
        "1rem",
        Group::Button,
        "16 px each side",
    ),
    css_value(
        "jw-button-radius",
        "8px",
        Group::Button,
        "page and header buttons",
    ),
    css_value(
        "jw-button-size",
        "0.875rem",
        Group::Button,
        "14 px, weight 500",
    ),
    css_value(
        "jw-button-header-height",
        "2.25rem",
        Group::Button,
        "header buttons: 36 px",
    ),
    css_value(
        "jw-button-pill-height",
        "2rem",
        Group::Button,
        "pill toggles: 32 px, 12 px padding, radius 6 px",
    ),
    css_value(
        "jw-button-icon",
        "1.0625rem",
        Group::Button,
        "17 px icons (17 to 18 px)",
    ),
    css_value(
        "jw-button-gap",
        "0.625rem",
        Group::Button,
        "10 px between icon and label",
    ),
    // Motion.
    css_value(
        "jw-rise-duration",
        "0.9s",
        Group::Motion,
        "the one entrance keyframe",
    ),
    css_value(
        "jw-rise-ease",
        "cubic-bezier(0.2, 0.7, 0.2, 1)",
        Group::Motion,
        "the entrance keyframe",
    ),
    css_value(
        "jw-rise-distance",
        "14px",
        Group::Motion,
        "translateY(14px) to 0",
    ),
    css_value(
        "jw-hover-duration",
        "150ms",
        Group::Motion,
        "hover and focus: colour alpha only",
    ),
    css_value(
        "jw-hover-ease",
        "cubic-bezier(0.4, 0, 0.2, 1)",
        Group::Motion,
        "hover and focus",
    ),
    css_value(
        "jw-header-duration",
        "300ms",
        Group::Motion,
        "header background after scrolling",
    ),
    css_value(
        "jw-fade-duration",
        "700ms",
        Group::Motion,
        "scene canvas fade-in",
    ),
    css_value(
        "jw-bar-duration",
        "0.45s",
        Group::Motion,
        "bar fills (clip-path)",
    ),
];

/// Primary button background.
pub const PRIMARY: Token = token(
    "jw-primary",
    Value::Color(ACCENT_RGB),
    Group::Action,
    "primary button background, ::selection",
);
/// Primary button hover.
pub const PRIMARY_HOVER: Token = color(
    "jw-primary-hover",
    0xffc26b,
    Group::Action,
    "primary button hover, link hover",
);
/// Text on the primary button and on the accent.
pub const ON_PRIMARY: Token = color(
    "jw-on-primary",
    0x1a1200,
    Group::Action,
    "text on the primary button and on accent fills",
);
/// The accent: focus rings, pills, bar fills, links, the last clause of a page h1.
pub const ACCENT: Token = token(
    "jw-accent",
    Value::Color(ACCENT_RGB),
    Group::Action,
    "focus rings, pills, bar fills, links",
);
/// The heading accent: gradient text on the last clause of a page h1 and on a section number.
pub const ACCENT_GRADIENT: Token = token(
    "jw-accent-gradient",
    Value::Gradient("90deg", &ACCENT_GRADIENT_STOPS),
    Group::Action,
    "background-clip: text on one span per heading at most",
);
/// Result tier green.
pub const TIER_GREEN: Token = color(
    "jw-tier-green",
    0x8be0b4,
    Group::Tier,
    "green results and success text only",
);
/// Result tier amber (not the brand amber).
pub const TIER_AMBER: Token = color(
    "jw-tier-amber",
    0xf2c46b,
    Group::Tier,
    "amber results and warnings only; not the brand amber",
);
/// Result tier red.
pub const TIER_RED: Token = color(
    "jw-tier-red",
    0xff8a8a,
    Group::Tier,
    "red results and errors only",
);

/// Looks a token up by name, with or without the leading `--`.
pub fn get(name: &str) -> Option<&'static Token> {
    let name = name.strip_prefix("--").unwrap_or(name);
    TOKENS.iter().find(|t| t.name == name)
}

/// The token that colours a tier label.
///
/// Tiers are a classification, not a harness result, so they are never green, amber or red
/// (nothing is green without a harness result behind it). Every tier label is the same
/// neutral text; the words say which tier it is.
pub const fn tier_token(tier: Tier) -> &'static Token {
    match tier {
        Tier::A | Tier::B | Tier::Unsupported => &WHITE_80,
    }
}

/// The token that colours a harness outcome. No result yet is neutral text at the 50 % floor.
pub const fn result_token(outcome: Outcome) -> &'static Token {
    match outcome {
        Outcome::Green => &TIER_GREEN,
        Outcome::Yellow => &TIER_AMBER,
        Outcome::Red => &TIER_RED,
        Outcome::NotRun => &WHITE_50,
    }
}

/// Badge background colour for an outcome (shields.io endpoints, `/badge/<mod>.svg`).
///
/// Green, amber and red badges use the tier colours with [`badge_text_color`] (the page colour)
/// as text. A mod with no harness result yet gets a page-coloured badge with light text, so it
/// cannot be mistaken for a result.
pub const fn badge_color(outcome: Outcome) -> Rgb {
    match outcome {
        Outcome::Green => Rgb::hex(0x8be0b4),
        Outcome::Yellow => Rgb::hex(0xf2c46b),
        Outcome::Red => Rgb::hex(0xff8a8a),
        Outcome::NotRun => PAGE_RGB,
    }
}

/// Text colour on a badge: the page colour on the tier colours, the body text colour on the
/// page-coloured "not run" badge.
pub const fn badge_text_color(outcome: Outcome) -> Rgb {
    match outcome {
        Outcome::Green | Outcome::Yellow | Outcome::Red => PAGE_RGB,
        Outcome::NotRun => Rgb::hex(0xf4f3ff),
    }
}

/// Renders `site/tokens.css`: every token on `:root`, grouped, with `color-scheme: dark`.
pub fn css() -> String {
    let mut out = String::new();
    out.push_str(
        "/* Generated by crates/report (jarrow_report::tokens::css). Do not edit by hand.\n \
         * Regenerate: JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current\n \
         * Values: docs/design-research/horizon.md, section 2, with Jarrow's four substitutions\n \
         * (night-blue page, amber accent, a sunrise, amber sky and sun ramps). Dark only.\n \
         * site/scripts/check-colors.mjs fails the build on any colour in dist/ that is not one of\n \
         * these values or an alpha step declared here.\n \
         */\n\n",
    );
    // Writing to a String cannot fail.
    let _ = writeln!(out, ":root {{");
    out.push_str("  color-scheme: dark;\n");
    for group in Group::ALL {
        let _ = write!(out, "\n  /* {} */\n", group.heading());
        for t in TOKENS.iter().filter(|t| t.group == group) {
            let _ = writeln!(out, "  --{}: {}; /* {} */", t.name, t.value, t.usage);
        }
    }
    out.push_str("}\n");
    out
}

/// Renders the tokens as JSON: the `tokens.json` of the npm package `jarrow`.
///
/// Top-level keys: `generated_by`, `color_scheme` (`dark`; there is no light theme),
/// `selector` (`:root`, where `site/tokens.css` declares every token) and `tokens`, in
/// [`TOKENS`] order. Each token has `name`, `property`, `group` ([`Group::as_str`]), `value`
/// (the CSS value, exactly as `site/tokens.css` writes it) and `usage`.
pub fn json() -> String {
    let mut out = String::from("{\n  \"generated_by\": ");
    push_json_str(&mut out, "crates/report (jarrow_report::tokens::json)");
    out.push_str(",\n  \"color_scheme\": ");
    push_json_str(&mut out, "dark");
    out.push_str(",\n  \"selector\": ");
    push_json_str(&mut out, ":root");
    out.push_str(",\n  \"tokens\": [\n");
    for (i, token) in TOKENS.iter().enumerate() {
        out.push_str("    { \"name\": ");
        push_json_str(&mut out, token.name);
        out.push_str(", \"property\": ");
        push_json_str(&mut out, &token.property());
        out.push_str(", \"group\": ");
        push_json_str(&mut out, token.group.as_str());
        out.push_str(", \"value\": ");
        push_json_str(&mut out, &token.value.to_string());
        out.push_str(", \"usage\": ");
        push_json_str(&mut out, token.usage);
        out.push_str(if i + 1 < TOKENS.len() {
            " },\n"
        } else {
            " }\n"
        });
    }
    out.push_str("  ]\n}\n");
    out
}

/// Appends `s` as a JSON string literal (RFC 8259), quotes included.
fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Writing to a String cannot fail.
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_lists_every_token_with_its_css_value() {
        let doc: serde_json::Value = serde_json::from_str(&json()).unwrap();
        assert_eq!(doc.as_object().unwrap().len(), 4);
        assert_eq!(doc["color_scheme"], "dark");
        assert_eq!(doc["selector"], ":root");
        let tokens = doc["tokens"].as_array().unwrap();
        assert_eq!(tokens.len(), TOKENS.len());
        let css = css();
        for (entry, token) in tokens.iter().zip(TOKENS) {
            assert_eq!(entry.as_object().unwrap().len(), 5, "{entry}");
            assert_eq!(entry["name"], token.name);
            assert_eq!(entry["property"], token.property());
            assert_eq!(entry["group"], token.group.as_str());
            assert_eq!(entry["value"], token.value.to_string());
            assert_eq!(entry["usage"], token.usage);
            // The JSON and the CSS carry the same value.
            let decl = format!("  {}: {};", token.property(), token.value);
            assert!(css.contains(&decl), "{decl}");
        }
        assert_eq!(tokens[0]["name"], "jw-page");
        assert_eq!(tokens[0]["value"], "#0b0f1f");
        let accent = tokens.iter().find(|t| t["name"] == "jw-accent").unwrap();
        assert_eq!(accent["value"], "#ffb454");
        assert_eq!(accent["group"], "action");
    }

    #[test]
    fn json_is_deterministic() {
        let json = json();
        assert_eq!(json, super::json());
        assert!(json.ends_with("}\n"));
        assert!(!json.contains('\r'));
    }

    #[test]
    fn json_strings_are_escaped() {
        let mut out = String::new();
        push_json_str(&mut out, "a\"b\\c\nd\u{1}é");
        assert_eq!(out, r#""a\"b\\c\nd\u0001é""#);
        let back: String = serde_json::from_str(&out).unwrap();
        assert_eq!(back, "a\"b\\c\nd\u{1}é");
    }

    #[test]
    fn group_names_are_unique_lower_case_words() {
        for (i, a) in Group::ALL.iter().enumerate() {
            assert!(a.as_str().chars().all(|c| c.is_ascii_lowercase()), "{a:?}");
            for b in &Group::ALL[i + 1..] {
                assert_ne!(a.as_str(), b.as_str());
            }
        }
    }

    fn page() -> Rgb {
        PAGE_RGB
    }

    /// The hero card's surface as rendered: white at 4 % over the page.
    fn card() -> Rgb {
        WHITE_RGB.over(4, PAGE_RGB)
    }

    fn rendered(t: &Token, background: Rgb) -> Rgb {
        match t.value.color_with_alpha() {
            Some((rgb, percent)) => rgb.over(percent, background),
            None => panic!("{} is not a colour", t.name),
        }
    }

    #[test]
    fn hex_and_display_round_trip() {
        let c = Rgb::hex(0x0b0f1f);
        assert_eq!((c.r, c.g, c.b), (0x0b, 0x0f, 0x1f));
        assert_eq!(c.to_string(), "#0b0f1f");
    }

    #[test]
    fn values_are_written_as_css() {
        assert_eq!(
            Value::Alpha(WHITE_RGB, 60).to_string(),
            "rgb(255 255 255 / 0.6)"
        );
        assert_eq!(
            Value::Alpha(WHITE_RGB, 4).to_string(),
            "rgb(255 255 255 / 0.04)"
        );
        assert_eq!(
            Value::Alpha(PAGE_RGB, 75).to_string(),
            "rgb(11 15 31 / 0.75)"
        );
        assert_eq!(
            ACCENT_GRADIENT.value.to_string(),
            "linear-gradient(90deg, #ffe3b0, #ffb454 55%, #ff8a3d)"
        );
        assert_eq!(
            get("jw-sky-fallback").map(|t| t.value.to_string()),
            Some(
                "linear-gradient(to bottom, #0b0f1f 0%, #1a1e4a 38%, #55326f 62%, #c66055 80%, \
                 #0b0f1f 86%)"
                    .to_owned()
            )
        );
    }

    #[test]
    fn compositing_matches_the_report() {
        // The report composites Horizon's white/60 over #100a1f and reads back #9f9da5.
        assert_eq!(WHITE_RGB.over(60, Rgb::hex(0x100a1f)), Rgb::hex(0x9f9da5));
        // And white/4 over the same page as the card, #1a1428.
        assert_eq!(WHITE_RGB.over(4, Rgb::hex(0x100a1f)), Rgb::hex(0x1a1428));
        assert_eq!(WHITE_RGB.over(100, PAGE_RGB), WHITE_RGB);
        assert_eq!(WHITE_RGB.over(0, PAGE_RGB), PAGE_RGB);
    }

    #[test]
    fn names_are_unique_and_prefixed() {
        for (i, a) in TOKENS.iter().enumerate() {
            assert!(a.name.starts_with("jw-"), "{}", a.name);
            assert!(
                a.name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{}",
                a.name
            );
            for b in &TOKENS[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn every_named_constant_is_in_the_table() {
        for t in [
            PAGE,
            PAGE_75,
            FG,
            WHITE,
            WHITE_90,
            WHITE_80,
            WHITE_72,
            WHITE_70,
            WHITE_65,
            WHITE_60,
            WHITE_50,
            WHITE_45,
            WHITE_40,
            WHITE_4,
            WHITE_8,
            PRIMARY,
            PRIMARY_HOVER,
            ON_PRIMARY,
            ACCENT,
            ACCENT_GRADIENT,
            TIER_GREEN,
            TIER_AMBER,
            TIER_RED,
        ] {
            assert_eq!(get(t.name), Some(&t), "{}", t.name);
        }
    }

    #[test]
    fn lookup_works_with_and_without_dashes() {
        assert_eq!(
            get("--jw-accent").and_then(|t| t.value.color()),
            Some(ACCENT_RGB)
        );
        assert_eq!(get("jw-page").and_then(|t| t.value.color()), Some(PAGE_RGB));
        assert!(get("jw-nope").is_none());
    }

    #[test]
    fn the_four_substitutions_are_exact() {
        let hex = |name: &str| get(name).map(|t| t.value.to_string());
        assert_eq!(hex("jw-page").as_deref(), Some("#0b0f1f"));
        assert_eq!(hex("jw-fg").as_deref(), Some("#f4f3ff"));
        assert_eq!(hex("jw-primary").as_deref(), Some("#ffb454"));
        assert_eq!(hex("jw-primary-hover").as_deref(), Some("#ffc26b"));
        assert_eq!(hex("jw-on-primary").as_deref(), Some("#1a1200"));
        assert_eq!(hex("jw-accent").as_deref(), Some("#ffb454"));
        assert_eq!(hex("jw-tier-green").as_deref(), Some("#8be0b4"));
        assert_eq!(hex("jw-tier-amber").as_deref(), Some("#f2c46b"));
        assert_eq!(hex("jw-tier-red").as_deref(), Some("#ff8a8a"));
        let sky: Vec<String> = SKY.iter().map(Rgb::to_string).collect();
        assert_eq!(
            sky.join(" "),
            "#0b0f1f #11173a #1a1e4a #2a2158 #3d2a66 #55326f #713b70 #8f466c #ad5262 #c66055 \
             #dc7447 #ee8d44 #f8a74a #ffbf5a #ffd27a"
        );
        let sun: Vec<String> = SUN.iter().map(Rgb::to_string).collect();
        assert_eq!(
            sun.join(" "),
            "#fff8dc #ffeeb0 #ffdd88 #ffc862 #ffb454 #ff9a48 #ff7a3d"
        );
        for (i, rgb) in SKY.iter().enumerate() {
            let t = get(&format!("jw-sky-{i}"));
            assert_eq!(t.and_then(|t| t.value.color()), Some(*rgb), "sky {i}");
        }
        for (i, rgb) in SUN.iter().enumerate() {
            let t = get(&format!("jw-sun-{i}"));
            assert_eq!(t.and_then(|t| t.value.color()), Some(*rgb), "sun {i}");
        }
        // The brand amber and the tier amber are different colours.
        assert_ne!(ACCENT.value, TIER_AMBER.value);
    }

    #[test]
    fn horizons_measured_values_are_kept_verbatim() {
        let css = |name: &str| get(name).map(|t| t.value.to_string());
        for (name, value) in [
            ("jw-h1-size", "clamp(2rem, 10.4cqi, 4.75rem)"),
            ("jw-h1-leading", "1.02"),
            ("jw-h1-tracking", "-0.035em"),
            ("jw-h2-size", "clamp(2rem, 3.6vw, 3rem)"),
            ("jw-h2-leading", "1.08"),
            ("jw-h2-tracking", "-0.03em"),
            ("jw-docs-body-leading", "1.7"),
            ("jw-body-leading", "1.625"),
            ("jw-space", "0.25rem"),
            ("jw-container", "72rem"),
            ("jw-gutter", "1.5rem"),
            ("jw-gutter-wide", "2.5rem"),
            ("jw-header-height", "4.5rem"),
            ("jw-card-radius", "16px"),
            ("jw-card-border", "0"),
            ("jw-card-shadow", "none"),
            ("jw-button-hero-height", "3rem"),
            ("jw-button-hero-radius", "10px"),
            ("jw-button-height", "2.5rem"),
            ("jw-button-header-height", "2.25rem"),
            ("jw-rise-ease", "cubic-bezier(0.2, 0.7, 0.2, 1)"),
            ("jw-hover-duration", "150ms"),
        ] {
            assert_eq!(css(name).as_deref(), Some(value), "{name}");
        }
        // The white alpha steps the report lists for text and surfaces.
        let alphas: Vec<u8> = TOKENS
            .iter()
            .filter_map(|t| match t.value {
                Value::Alpha(rgb, percent) if rgb == WHITE_RGB => Some(percent),
                _ => None,
            })
            .collect();
        for step in [80, 70, 65, 60, 50, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14] {
            assert!(alphas.contains(&step), "white/{step}");
        }
    }

    #[test]
    fn spacing_and_layout_lengths_are_multiples_of_4px() {
        // Rule 10: every margin, padding, height and column is on the 4 px grid. Type sizes,
        // radii, icon sizes and blurs are not lengths of the grid.
        let on_grid = |t: &Token| match t.group {
            Group::Space | Group::Layout => !t.name.ends_with("-blur"),
            Group::Card | Group::Button => t.name.contains("height") || t.name.contains("padding"),
            _ => false,
        };
        let mut checked = 0;
        for t in TOKENS.iter().filter(|t| on_grid(t)) {
            let Value::Css(css) = t.value else { continue };
            let px = if let Some(rem) = css.strip_suffix("rem") {
                rem.parse::<f64>().map(|r| r * 16.0)
            } else if let Some(px) = css.strip_suffix("px") {
                px.parse::<f64>()
            } else {
                panic!("{} = {css} is not a length", t.name);
            };
            let px = px.unwrap_or_else(|err| panic!("{} = {css}: {err}", t.name));
            assert!(
                (px / 4.0).fract().abs() < 1e-9,
                "{} = {css} ({px} px)",
                t.name
            );
            checked += 1;
        }
        assert!(checked >= 25, "only {checked} lengths checked");
    }

    #[test]
    fn every_text_step_meets_the_contrast_floor() {
        for t in &TEXT_STEPS {
            for background in [page(), card()] {
                let ratio = contrast_ratio(rendered(t, background), background);
                assert!(
                    ratio >= MIN_TEXT_CONTRAST,
                    "{} on {background}: {ratio:.2}",
                    t.name
                );
            }
        }
    }

    #[test]
    fn actions_and_tiers_meet_the_contrast_floor() {
        for t in [ACCENT, TIER_GREEN, TIER_AMBER, TIER_RED] {
            for background in [page(), card()] {
                let ratio = contrast_ratio(rendered(&t, background), background);
                assert!(ratio >= MIN_TEXT_CONTRAST, "{}: {ratio:.2}", t.name);
            }
        }
        let on = rendered(&ON_PRIMARY, page());
        for t in [PRIMARY, PRIMARY_HOVER] {
            let ratio = contrast_ratio(on, rendered(&t, page()));
            assert!(ratio >= MIN_TEXT_CONTRAST, "{}: {ratio:.2}", t.name);
        }
        // The page colour on the accent pill.
        assert!(contrast_ratio(page(), ACCENT_RGB) >= MIN_TEXT_CONTRAST);
    }

    #[test]
    fn decorative_steps_sit_below_the_floor_of_information() {
        // 45 % and 40 % are decorative only; they are not in TEXT_STEPS.
        for t in [WHITE_45, WHITE_40] {
            assert!(!TEXT_STEPS.contains(&t), "{}", t.name);
        }
    }

    #[test]
    fn tiers_are_neutral_and_results_use_the_tier_colours() {
        for tier in Tier::ALL {
            assert_eq!(tier_token(tier).group, Group::Text);
        }
        assert_eq!(result_token(Outcome::Green).name, "jw-tier-green");
        assert_eq!(result_token(Outcome::Yellow).name, "jw-tier-amber");
        assert_eq!(result_token(Outcome::Red).name, "jw-tier-red");
        assert_eq!(result_token(Outcome::NotRun).name, "jw-white-50");
    }

    #[test]
    fn badge_colors_are_the_tier_colours() {
        assert_eq!(badge_color(Outcome::Green).to_string(), "#8be0b4");
        assert_eq!(badge_color(Outcome::Yellow).to_string(), "#f2c46b");
        assert_eq!(badge_color(Outcome::Red).to_string(), "#ff8a8a");
        assert_eq!(badge_color(Outcome::NotRun).to_string(), "#0b0f1f");
        for outcome in [Outcome::Green, Outcome::Yellow, Outcome::Red] {
            assert_eq!(
                Some(badge_color(outcome)),
                result_token(outcome).value.color(),
                "{outcome}"
            );
        }
        for outcome in Outcome::ALL {
            let ratio = contrast_ratio(badge_color(outcome), badge_text_color(outcome));
            assert!(ratio >= MIN_TEXT_CONTRAST, "{outcome}: {ratio:.2}");
        }
    }

    #[test]
    fn contrast_ratio_matches_wcag_reference_points() {
        let black = Rgb::hex(0x000000);
        let white = Rgb::hex(0xffffff);
        assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-9);
        assert!((contrast_ratio(white, white) - 1.0).abs() < 1e-9);
        // #777777 on white is the textbook 4.48:1.
        let grey = contrast_ratio(Rgb::hex(0x777777), white);
        assert!((grey - 4.48).abs() < 0.01, "{grey}");
    }

    #[test]
    fn css_declares_every_token_once_and_is_dark_only() {
        let css = css();
        assert!(css.starts_with("/* Generated by crates/report"));
        assert!(css.ends_with("}\n"));
        for t in TOKENS {
            let decl = format!("  --{}: {};", t.name, t.value);
            assert_eq!(css.matches(&decl).count(), 1, "{decl}");
        }
        assert_eq!(css.matches("color-scheme: dark;").count(), 1);
        assert!(!css.contains("color-scheme: light"), "no light theme");
        assert!(!css.contains("data-theme"), "no theme switch");
        assert!(!css.contains("prefers-color-scheme"));
        assert!(!css.contains('\r'));
    }

    #[test]
    fn css_writes_groups_in_order() {
        let css = css();
        let mut last = 0;
        for group in Group::ALL {
            let at = css
                .find(&format!("/* {} */", group.heading()))
                .unwrap_or_else(|| panic!("{group:?} heading"));
            assert!(at > last, "{group:?} out of order");
            last = at;
            assert!(
                TOKENS.iter().any(|t| t.group == group),
                "{group:?} is empty"
            );
        }
    }

    #[test]
    fn gradients_use_only_declared_colours() {
        let colours: Vec<Rgb> = TOKENS.iter().filter_map(|t| t.value.color()).collect();
        for t in TOKENS {
            if let Value::Gradient(_, stops) = t.value {
                for s in stops {
                    assert!(colours.contains(&s.color), "{}: {}", t.name, s.color);
                }
            }
        }
    }

    #[test]
    fn css_is_deterministic() {
        assert_eq!(css(), css());
    }
}
