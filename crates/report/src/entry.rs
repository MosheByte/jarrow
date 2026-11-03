//! Structured report entries (see `docs/rules.md`).

use std::fmt;

use serde::{Deserialize, Serialize};

/// What happened at one place in the JAR.
///
/// Serialised in kebab-case: `"remapped"`, `"not-translated"`, ...
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    /// A class, member, descriptor or class-naming string was remapped through the mapping
    /// chain (intermediary to Mojang official names).
    Remapped,
    /// A rule rewrote a call site, a class reference or an API usage.
    Rewritten,
    /// A manifest or resource field was translated (for example `fabric.mod.json` to
    /// `neoforge.mods.toml`, or an access widener line to an access transformer line).
    Translated,
    /// A file or class was generated: an entrypoint `@Mod` class, a registration bridge, an
    /// adapter.
    Generated,
    /// Left as it was on purpose; the note says why.
    Untouched,
    /// Could not be translated; the note says what and why. The output never pretends.
    NotTranslated,
    /// Converted, but flagged for a human look (for example an `@Overwrite` of a vanilla method).
    Flagged,
    /// A reason the JAR is unsupported (a native library, a bytecode transformer, a license).
    Unsupported,
}

impl EntryKind {
    /// Every kind, in declaration order.
    pub const ALL: [EntryKind; 8] = [
        EntryKind::Remapped,
        EntryKind::Rewritten,
        EntryKind::Translated,
        EntryKind::Generated,
        EntryKind::Untouched,
        EntryKind::NotTranslated,
        EntryKind::Flagged,
        EntryKind::Unsupported,
    ];

    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            EntryKind::Remapped => "remapped",
            EntryKind::Rewritten => "rewritten",
            EntryKind::Translated => "translated",
            EntryKind::Generated => "generated",
            EntryKind::Untouched => "untouched",
            EntryKind::NotTranslated => "not-translated",
            EntryKind::Flagged => "flagged",
            EntryKind::Unsupported => "unsupported",
        }
    }

    /// Whether an entry of this kind must carry both `before` and `after`.
    pub const fn requires_before_and_after(self) -> bool {
        matches!(
            self,
            EntryKind::Remapped | EntryKind::Rewritten | EntryKind::Translated
        )
    }

    /// Whether an entry of this kind must name the rule that produced it.
    pub const fn requires_rule(self) -> bool {
        matches!(self, EntryKind::Rewritten | EntryKind::Generated)
    }

    /// Whether an entry of this kind must explain itself in `note`.
    pub const fn requires_note(self) -> bool {
        matches!(
            self,
            EntryKind::Untouched
                | EntryKind::NotTranslated
                | EntryKind::Flagged
                | EntryKind::Unsupported
        )
    }
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// How sure the engine is that a change preserves behaviour. The harness has the last word.
///
/// Serialised in kebab-case: `"exact"`, `"high"`, `"medium"`, `"low"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// Mechanical and equivalent by construction, such as a rename through the mapping chain.
    Exact,
    /// A rule whose documented semantics are the same on both loaders.
    High,
    /// A rule with a documented semantic difference (cancellation, phases, return values).
    Medium,
    /// A heuristic or agent-made change; only a harness result can back it.
    Low,
}

impl Confidence {
    /// Every confidence level, from most to least certain.
    pub const ALL: [Confidence; 4] = [
        Confidence::Exact,
        Confidence::High,
        Confidence::Medium,
        Confidence::Low,
    ];

    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            Confidence::Exact => "exact",
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Where in the JAR an entry applies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Location {
    /// Path of the file inside the JAR, `/`-separated (`com/example/Foo.class`,
    /// `fabric.mod.json`). Empty when the entry is about the JAR as a whole.
    pub path: String,
    /// Optional position inside that file: a member (`tick()V`), a JSON pointer
    /// (`/depends/fabric-api`), a TOML key or a line. `None` for the whole file.
    #[serde(default)]
    pub detail: Option<String>,
}

impl Location {
    /// The JAR as a whole.
    pub fn jar() -> Self {
        Self {
            path: String::new(),
            detail: None,
        }
    }

    /// A whole file inside the JAR.
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            detail: None,
        }
    }

    /// A position inside a file of the JAR.
    pub fn within(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            detail: Some(detail.into()),
        }
    }

    /// Whether this location is the JAR as a whole.
    pub fn is_jar(&self) -> bool {
        self.path.is_empty() && self.detail.is_none()
    }
}

impl fmt::Display for Location {
    /// `path`, `path#detail`, or `(jar)` for the JAR as a whole.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = if self.path.is_empty() {
            "(jar)"
        } else {
            &self.path
        };
        match &self.detail {
            Some(detail) => write!(f, "{path}#{detail}"),
            None => f.write_str(path),
        }
    }
}

/// One structured line of a conversion report.
///
/// The six fields every report entry carries (`kind`, `location`, `before`, `after`, `rule`,
/// `confidence`) plus `note`, the human explanation that untouched, untranslated, flagged
/// and unsupported entries must carry. Optional fields serialise as `null`, so every
/// consumer sees the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReportEntry {
    /// What happened.
    pub kind: EntryKind,
    /// Where it happened.
    pub location: Location,
    /// The original text or symbol, when there was one.
    #[serde(default)]
    pub before: Option<String>,
    /// The new text or symbol, when there is one.
    #[serde(default)]
    pub after: Option<String>,
    /// Id of the rule that fired, as written in the rule tables.
    #[serde(default)]
    pub rule: Option<String>,
    /// How sure the engine is that behaviour is preserved.
    pub confidence: Confidence,
    /// Why: required for untouched, not-translated, flagged and unsupported entries.
    #[serde(default)]
    pub note: Option<String>,
}

impl ReportEntry {
    /// An entry with only the required fields set; add the rest with the `with_*` methods.
    pub fn new(kind: EntryKind, location: Location, confidence: Confidence) -> Self {
        Self {
            kind,
            location,
            before: None,
            after: None,
            rule: None,
            confidence,
            note: None,
        }
    }

    /// Sets `before`.
    #[must_use]
    pub fn with_before(mut self, before: impl Into<String>) -> Self {
        self.before = Some(before.into());
        self
    }

    /// Sets `after`.
    #[must_use]
    pub fn with_after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    /// Sets `rule`.
    #[must_use]
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Sets `note`.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Checks that the entry carries the fields its kind needs, so a report can never say
    /// "not translated" without saying why.
    pub fn validate(&self) -> Result<(), EntryError> {
        let present =
            |field: &Option<String>| field.as_deref().is_some_and(|s| !s.trim().is_empty());
        if self.kind.requires_before_and_after() {
            if !present(&self.before) {
                return Err(EntryError::missing(self.kind, "before"));
            }
            if !present(&self.after) {
                return Err(EntryError::missing(self.kind, "after"));
            }
        }
        if self.kind.requires_rule() && !present(&self.rule) {
            return Err(EntryError::missing(self.kind, "rule"));
        }
        if self.kind.requires_note() && !present(&self.note) {
            return Err(EntryError::missing(self.kind, "note"));
        }
        Ok(())
    }
}

/// A [`ReportEntry`] that lacks a field its kind requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryError {
    /// The entry's kind.
    pub kind: EntryKind,
    /// The missing (or blank) field.
    pub field: &'static str,
}

impl EntryError {
    fn missing(kind: EntryKind, field: &'static str) -> Self {
        Self { kind, field }
    }
}

impl fmt::Display for EntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a {} entry needs a non-empty `{}`",
            self.kind, self.field
        )
    }
}

impl std::error::Error for EntryError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(kind: EntryKind) -> ReportEntry {
        let entry = ReportEntry::new(
            kind,
            Location::within("com/example/Mod.class", "onInitialize()V"),
            Confidence::High,
        );
        match kind {
            EntryKind::Remapped => entry
                .with_before("net/minecraft/class_1792")
                .with_after("net/minecraft/world/item/Item"),
            EntryKind::Rewritten => entry
                .with_before("ServerTickEvents.END_SERVER_TICK")
                .with_after("ServerTickEvent.Post")
                .with_rule("events/server-tick-end"),
            EntryKind::Translated => entry
                .with_before("\"environment\": \"client\"")
                .with_after("side = \"CLIENT\""),
            EntryKind::Generated => entry
                .with_after("com/example/jarrow/ExampleModNeoForge.class")
                .with_rule("entrypoints/main"),
            EntryKind::Untouched
            | EntryKind::NotTranslated
            | EntryKind::Flagged
            | EntryKind::Unsupported => entry.with_note("explained here"),
        }
    }

    #[test]
    fn every_kind_round_trips_through_json() {
        for kind in EntryKind::ALL {
            for confidence in Confidence::ALL {
                let mut entry = sample(kind);
                entry.confidence = confidence;
                entry.validate().unwrap();
                let json = serde_json::to_string(&entry).unwrap();
                let back: ReportEntry = serde_json::from_str(&json).unwrap();
                assert_eq!(back, entry, "{json}");
            }
        }
    }

    #[test]
    fn json_shape_is_stable() {
        let entry = ReportEntry::new(
            EntryKind::NotTranslated,
            Location::within("fabric.mod.json", "/entrypoints/modmenu"),
            Confidence::Exact,
        )
        .with_before("modmenu")
        .with_note("plugin entrypoint for `modmenu`, not translated");
        let value = serde_json::to_value(&entry).unwrap();
        let expected = serde_json::json!({
            "kind": "not-translated",
            "location": { "path": "fabric.mod.json", "detail": "/entrypoints/modmenu" },
            "before": "modmenu",
            "after": null,
            "rule": null,
            "confidence": "exact",
            "note": "plugin entrypoint for `modmenu`, not translated"
        });
        assert_eq!(value, expected);
    }

    #[test]
    fn optional_fields_may_be_omitted_on_input() {
        let json = r#"{"kind":"remapped","location":{"path":"a/B.class"},"confidence":"exact",
                       "before":"a/class_1","after":"a/B"}"#;
        let entry: ReportEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.location, Location::file("a/B.class"));
        assert_eq!(entry.rule, None);
        assert_eq!(entry.note, None);
        entry.validate().unwrap();
    }

    #[test]
    fn unknown_enum_values_are_rejected() {
        let json = r#"{"kind":"magic","location":{"path":""},"confidence":"exact"}"#;
        assert!(serde_json::from_str::<ReportEntry>(json).is_err());
        let json = r#"{"kind":"remapped","location":{"path":""},"confidence":"certain"}"#;
        assert!(serde_json::from_str::<ReportEntry>(json).is_err());
    }

    #[test]
    fn kind_ids_match_serde() {
        for kind in EntryKind::ALL {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.id())
            );
        }
        for confidence in Confidence::ALL {
            assert_eq!(
                serde_json::to_string(&confidence).unwrap(),
                format!("\"{}\"", confidence.id())
            );
        }
    }

    #[test]
    fn validation_requires_an_explanation() {
        let bare = ReportEntry::new(EntryKind::NotTranslated, Location::jar(), Confidence::Low);
        let err = bare.validate().unwrap_err();
        assert_eq!(err.field, "note");
        assert_eq!(
            err.to_string(),
            "a not-translated entry needs a non-empty `note`"
        );
        let blank = bare.clone().with_note("   ");
        assert!(blank.validate().is_err());
        assert!(
            bare.with_note("license forbids modification")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn validation_requires_before_after_and_rule() {
        let loc = Location::file("x.class");
        let remap = ReportEntry::new(EntryKind::Remapped, loc.clone(), Confidence::Exact);
        assert_eq!(remap.validate().unwrap_err().field, "before");
        assert_eq!(
            remap.clone().with_before("a").validate().unwrap_err().field,
            "after"
        );
        let rewrite = ReportEntry::new(EntryKind::Rewritten, loc, Confidence::Medium)
            .with_before("a")
            .with_after("b");
        assert_eq!(rewrite.validate().unwrap_err().field, "rule");
        assert!(rewrite.with_rule("events/x").validate().is_ok());
    }

    #[test]
    fn location_display() {
        assert_eq!(Location::jar().to_string(), "(jar)");
        assert!(Location::jar().is_jar());
        assert_eq!(Location::file("pack.mcmeta").to_string(), "pack.mcmeta");
        assert_eq!(
            Location::within("a/B.class", "tick()V").to_string(),
            "a/B.class#tick()V"
        );
        assert!(!Location::file("pack.mcmeta").is_jar());
    }
}
