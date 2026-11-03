//! Design tokens: the single source of the site's colours and the badge colours.
//!
//! `site/src/styles/tokens.css` is generated from [`css`] and checked against it by the
//! `tokens_current` test on every CI run. To change a colour, edit [`TOKENS`] and run
//!
//! ```text
//! JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current
//! ```
//!
//! The site is dark first: dark values sit on `:root`, light values on
//! `[data-theme='light']`. Green, amber and red (`--jw-result-*`) are reserved for harness
//! results and are never used decoratively; the accent is for actions only.
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

/// What a token may be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Backgrounds, surfaces and borders.
    Surface,
    /// Body and secondary text.
    Text,
    /// Buttons, links and focus: actions only, never a result.
    Action,
    /// Tier labels and badges.
    Tier,
    /// Harness results only; never decorative.
    Result,
}

impl Role {
    /// Lower-case name, as written to the JSON: `surface`, `text`, `action`, `tier`, `result`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Surface => "surface",
            Role::Text => "text",
            Role::Action => "action",
            Role::Tier => "tier",
            Role::Result => "result",
        }
    }
}

/// One design token: a CSS custom property with a dark and a light value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// Custom property name without the leading `--`, for example `jw-bg`.
    pub name: &'static str,
    /// Value in the dark theme (the default, on `:root`).
    pub dark: Rgb,
    /// Value in the light theme (`[data-theme='light']`).
    pub light: Rgb,
    /// What the token may be used for.
    pub role: Role,
    /// Short usage note, written next to the property in the CSS.
    pub usage: &'static str,
}

impl Token {
    /// The CSS custom property, for example `--jw-bg`.
    pub fn property(&self) -> String {
        format!("--{}", self.name)
    }

    /// The value for a theme.
    pub const fn value(&self, theme: Theme) -> Rgb {
        match theme {
            Theme::Dark => self.dark,
            Theme::Light => self.light,
        }
    }
}

/// A colour theme of the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Theme {
    /// The default theme, on `:root`.
    Dark,
    /// The light theme, on `[data-theme='light']`.
    Light,
}

impl Theme {
    /// Both themes, default first.
    pub const ALL: [Theme; 2] = [Theme::Dark, Theme::Light];

    /// Lower-case name, as written to the JSON: `dark` or `light`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    /// The CSS selector the theme's values are declared on.
    pub const fn selector(self) -> &'static str {
        match self {
            Theme::Dark => ":root",
            Theme::Light => "[data-theme='light']",
        }
    }

    /// The value of the CSS `color-scheme` property for the theme.
    pub const fn color_scheme(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }
}

const fn token(
    name: &'static str,
    dark: u32,
    light: u32,
    role: Role,
    usage: &'static str,
) -> Token {
    Token {
        name,
        dark: Rgb::hex(dark),
        light: Rgb::hex(light),
        role,
        usage,
    }
}

/// Page background.
pub const BG: Token = token(
    "jw-bg",
    0x0b0d12,
    0xfbfbfd,
    Role::Surface,
    "page background",
);
/// Cards, panels, the drop zone.
pub const SURFACE: Token = token(
    "jw-surface",
    0x141821,
    0xffffff,
    Role::Surface,
    "cards, panels, the drop zone",
);
/// Raised or hovered surfaces, table stripes.
pub const SURFACE_2: Token = token(
    "jw-surface-2",
    0x1c2230,
    0xf1f3f8,
    Role::Surface,
    "raised or hovered surfaces, table stripes",
);
/// Borders and dividers.
pub const BORDER: Token = token(
    "jw-border",
    0x2a3142,
    0xdde2ea,
    Role::Surface,
    "borders and dividers",
);
/// Body text.
pub const TEXT: Token = token("jw-text", 0xeef1f6, 0x10131a, Role::Text, "body text");
/// Secondary text.
pub const MUTED: Token = token("jw-muted", 0xa3adbd, 0x4b5563, Role::Text, "secondary text");
/// Actions only: buttons, links, focus rings.
pub const ACCENT: Token = token(
    "jw-accent",
    0xff5ca8,
    0xd6246e,
    Role::Action,
    "actions only: buttons, links, focus",
);
/// Hovered or pressed actions.
pub const ACCENT_STRONG: Token = token(
    "jw-accent-strong",
    0xff2e8f,
    0xb0185a,
    Role::Action,
    "hovered or pressed actions",
);
/// Text on accent backgrounds.
pub const ON_ACCENT: Token = token(
    "jw-on-accent",
    0x1a0010,
    0xffffff,
    Role::Action,
    "text on accent backgrounds",
);
/// Tier A.
pub const TIER_A: Token = token("jw-tier-a", 0x38bdf8, 0x0369a1, Role::Tier, "Tier A");
/// Tier B.
pub const TIER_B: Token = token("jw-tier-b", 0xa78bfa, 0x6d28d9, Role::Tier, "Tier B");
/// The unsupported tier.
pub const TIER_UNSUPPORTED: Token = token(
    "jw-tier-unsupported",
    0x94a3b8,
    0x475569,
    Role::Tier,
    "unsupported tier",
);
/// Harness result green.
pub const RESULT_GREEN: Token = token(
    "jw-result-green",
    0x22c55e,
    0x15803d,
    Role::Result,
    "result green; never decorative",
);
/// Harness result yellow.
pub const RESULT_YELLOW: Token = token(
    "jw-result-yellow",
    0xf59e0b,
    0xb45309,
    Role::Result,
    "result yellow; never decorative",
);
/// Harness result red.
pub const RESULT_RED: Token = token(
    "jw-result-red",
    0xef4444,
    0xb91c1c,
    Role::Result,
    "result red; never decorative",
);
/// No harness result yet.
pub const RESULT_NONE: Token = token(
    "jw-result-none",
    0x6b7280,
    0x4b5563,
    Role::Result,
    "no harness result yet",
);

/// Every design token, in the order they are written to the CSS.
pub const TOKENS: [Token; 16] = [
    BG,
    SURFACE,
    SURFACE_2,
    BORDER,
    TEXT,
    MUTED,
    ACCENT,
    ACCENT_STRONG,
    ON_ACCENT,
    TIER_A,
    TIER_B,
    TIER_UNSUPPORTED,
    RESULT_GREEN,
    RESULT_YELLOW,
    RESULT_RED,
    RESULT_NONE,
];

/// Looks a token up by name, with or without the leading `--`.
pub fn get(name: &str) -> Option<&'static Token> {
    let name = name.strip_prefix("--").unwrap_or(name);
    TOKENS.iter().find(|t| t.name == name)
}

/// The token that colours a tier.
pub const fn tier_token(tier: Tier) -> &'static Token {
    match tier {
        Tier::A => &TIER_A,
        Tier::B => &TIER_B,
        Tier::Unsupported => &TIER_UNSUPPORTED,
    }
}

/// The token that colours a harness outcome.
pub const fn result_token(outcome: Outcome) -> &'static Token {
    match outcome {
        Outcome::Green => &RESULT_GREEN,
        Outcome::Yellow => &RESULT_YELLOW,
        Outcome::Red => &RESULT_RED,
        Outcome::NotRun => &RESULT_NONE,
    }
}

/// Badge background colour for an outcome (shields.io endpoints, `/badge/<mod>.svg`).
///
/// Badges carry white text and sit on READMEs of either theme, so they use the light-theme
/// value of the result token, which keeps at least 4.5:1 contrast against white.
pub const fn badge_color(outcome: Outcome) -> Rgb {
    result_token(outcome).light
}

/// Renders `site/src/styles/tokens.css`: dark values on `:root`, light values on
/// `[data-theme='light']`.
pub fn css() -> String {
    let mut out = String::new();
    out.push_str(
        "/* Generated by crates/report (jarrow_report::tokens::css). Do not edit by hand.\n \
         * Regenerate: JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current\n \
         * Dark first: dark values on :root, light values on [data-theme='light'].\n \
         * --jw-result-* colours are reserved for harness results; --jw-accent is for actions only.\n \
         */\n",
    );
    for theme in Theme::ALL {
        out.push('\n');
        // Writing to a String cannot fail.
        let _ = writeln!(out, "{} {{", theme.selector());
        let _ = writeln!(out, "  color-scheme: {};", theme.color_scheme());
        for token in &TOKENS {
            let _ = match theme {
                Theme::Dark => writeln!(
                    out,
                    "  --{}: {}; /* {} */",
                    token.name, token.dark, token.usage
                ),
                Theme::Light => writeln!(out, "  --{}: {};", token.name, token.light),
            };
        }
        out.push_str("}\n");
    }
    out
}

/// Renders the tokens as JSON: the `tokens.json` of the npm package `jarrow`.
///
/// Top-level keys: `generated_by`, `default_theme` (`dark`), `selectors` (each theme's CSS
/// selector) and `tokens`, in [`TOKENS`] order. Each token has `name`, `property`, `role`
/// ([`Role::as_str`]), `dark`, `light` (lower-case `#rrggbb`) and `usage`.
pub fn json() -> String {
    let mut out = String::from("{\n  \"generated_by\": ");
    push_json_str(&mut out, "crates/report (jarrow_report::tokens::json)");
    out.push_str(",\n  \"default_theme\": ");
    push_json_str(&mut out, Theme::Dark.as_str());
    out.push_str(",\n  \"selectors\": {");
    for (i, theme) in Theme::ALL.iter().enumerate() {
        out.push_str(if i == 0 { " " } else { ", " });
        push_json_str(&mut out, theme.as_str());
        out.push_str(": ");
        push_json_str(&mut out, theme.selector());
    }
    out.push_str(" },\n  \"tokens\": [\n");
    for (i, token) in TOKENS.iter().enumerate() {
        out.push_str("    { \"name\": ");
        push_json_str(&mut out, token.name);
        out.push_str(", \"property\": ");
        push_json_str(&mut out, &token.property());
        out.push_str(", \"role\": ");
        push_json_str(&mut out, token.role.as_str());
        for theme in Theme::ALL {
            out.push_str(", ");
            push_json_str(&mut out, theme.as_str());
            out.push_str(": ");
            push_json_str(&mut out, &token.value(theme).to_string());
        }
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
    fn hex_and_display_round_trip() {
        let c = Rgb::hex(0x0b0d12);
        assert_eq!((c.r, c.g, c.b), (0x0b, 0x0d, 0x12));
        assert_eq!(c.to_string(), "#0b0d12");
    }

    #[test]
    fn names_are_unique_and_prefixed() {
        for (i, a) in TOKENS.iter().enumerate() {
            assert!(a.name.starts_with("jw-"), "{}", a.name);
            assert!(
                a.name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            );
            for b in &TOKENS[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn lookup_works_with_and_without_dashes() {
        assert_eq!(get("--jw-accent").map(|t| t.dark), Some(Rgb::hex(0xff5ca8)));
        assert_eq!(get("jw-accent").map(|t| t.light), Some(Rgb::hex(0xd6246e)));
        assert!(get("jw-nope").is_none());
    }

    #[test]
    fn every_tier_and_outcome_has_its_own_token() {
        for tier in Tier::ALL {
            assert_eq!(tier_token(tier).role, Role::Tier);
        }
        for outcome in Outcome::ALL {
            assert_eq!(result_token(outcome).role, Role::Result);
        }
        assert_eq!(tier_token(Tier::A).name, "jw-tier-a");
        assert_eq!(tier_token(Tier::B).name, "jw-tier-b");
        assert_eq!(tier_token(Tier::Unsupported).name, "jw-tier-unsupported");
        assert_eq!(result_token(Outcome::Green).name, "jw-result-green");
        assert_eq!(result_token(Outcome::Yellow).name, "jw-result-yellow");
        assert_eq!(result_token(Outcome::Red).name, "jw-result-red");
        assert_eq!(result_token(Outcome::NotRun).name, "jw-result-none");
    }

    #[test]
    fn badge_colors() {
        assert_eq!(badge_color(Outcome::Green).to_string(), "#15803d");
        assert_eq!(badge_color(Outcome::Yellow).to_string(), "#b45309");
        assert_eq!(badge_color(Outcome::Red).to_string(), "#b91c1c");
        assert_eq!(badge_color(Outcome::NotRun).to_string(), "#4b5563");
        let white = Rgb::hex(0xffffff);
        for outcome in Outcome::ALL {
            let ratio = contrast_ratio(badge_color(outcome), white);
            assert!(ratio >= 4.5, "{outcome}: {ratio:.2}");
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
    fn themes_meet_contrast_targets() {
        for theme in Theme::ALL {
            let bg = BG.value(theme);
            for back in [BG, SURFACE, SURFACE_2] {
                // Body text: WCAG AAA. Secondary text: AA.
                assert!(contrast_ratio(TEXT.value(theme), back.value(theme)) >= 7.0);
                assert!(contrast_ratio(MUTED.value(theme), back.value(theme)) >= 4.5);
            }
            // Text on actions: AA, in both the normal and the strong state.
            for action in [ACCENT, ACCENT_STRONG] {
                let ratio = contrast_ratio(ON_ACCENT.value(theme), action.value(theme));
                assert!(ratio >= 4.5, "{} {theme:?}: {ratio:.2}", action.name);
            }
            // Tier and result colours are UI components: at least 3:1 on the page.
            for token in TOKENS
                .iter()
                .filter(|t| matches!(t.role, Role::Tier | Role::Result))
            {
                let ratio = contrast_ratio(token.value(theme), bg);
                assert!(ratio >= 3.0, "{} {theme:?}: {ratio:.2}", token.name);
            }
        }
    }

    #[test]
    fn css_declares_every_token_once_per_theme() {
        let css = css();
        assert!(css.starts_with("/* Generated by crates/report"));
        assert!(css.ends_with("}\n"));
        let root = css.find(":root {").unwrap();
        let light = css.find("[data-theme='light'] {").unwrap();
        assert!(root < light, "dark block comes first");
        let (dark_block, light_block) = css.split_at(light);
        for token in &TOKENS {
            let dark_decl = format!("  --{}: {};", token.name, token.dark);
            let light_decl = format!("  --{}: {};", token.name, token.light);
            assert_eq!(dark_block.matches(&dark_decl).count(), 1, "{dark_decl}");
            assert_eq!(light_block.matches(&light_decl).count(), 1, "{light_decl}");
        }
        assert_eq!(css.matches("color-scheme: dark;").count(), 1);
        assert_eq!(css.matches("color-scheme: light;").count(), 1);
        assert!(!css.contains('\r'));
    }

    #[test]
    fn css_matches_the_fixed_palette() {
        let css = css();
        for expected in [
            "--jw-bg: #0b0d12;",
            "--jw-bg: #fbfbfd;",
            "--jw-accent: #ff5ca8;",
            "--jw-accent: #d6246e;",
            "--jw-on-accent: #1a0010;",
            "--jw-result-none: #6b7280;",
            "--jw-tier-unsupported: #475569;",
        ] {
            assert!(css.contains(expected), "{expected}");
        }
    }

    #[test]
    fn css_is_deterministic() {
        assert_eq!(css(), css());
    }

    #[test]
    fn json_lists_every_token_with_both_values() {
        let doc: serde_json::Value = serde_json::from_str(&json()).unwrap();
        assert_eq!(doc.as_object().unwrap().len(), 4);
        assert_eq!(doc["default_theme"], "dark");
        assert_eq!(doc["selectors"]["dark"], ":root");
        assert_eq!(doc["selectors"]["light"], "[data-theme='light']");
        let tokens = doc["tokens"].as_array().unwrap();
        assert_eq!(tokens.len(), TOKENS.len());
        for (entry, token) in tokens.iter().zip(&TOKENS) {
            assert_eq!(entry.as_object().unwrap().len(), 6, "{entry}");
            assert_eq!(entry["name"], token.name);
            assert_eq!(entry["property"], token.property());
            assert_eq!(entry["role"], token.role.as_str());
            assert_eq!(entry["dark"], token.dark.to_string());
            assert_eq!(entry["light"], token.light.to_string());
            assert_eq!(entry["usage"], token.usage);
        }
        assert_eq!(tokens[6]["name"], "jw-accent");
        assert_eq!(tokens[6]["dark"], "#ff5ca8");
        assert_eq!(tokens[12]["role"], "result");
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
}
