//! Server logs: parsing, normalisation for noise matching, and classification of the reason
//! a run is red.
//!
//! Every reason id here is one of the board's closed `reason_kind` vocabulary
//! (`docs/board.md`). Classification is substring matching on hand-picked markers, applied
//! only to ERROR and FATAL events, to text written outside the logger (stderr), and to crash
//! reports. A harmless `ClassNotFoundException` logged at WARN or DEBUG, which loaders and
//! mods do on purpose when probing for optional classes, never makes a run red.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Why a harness result is not green, or why nothing ran. The ids are the board's
/// `reason_kind` values (`docs/board.md`); only the ones the harness produces are here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonKind {
    /// Not run: the Minecraft EULA gate is closed (D-013).
    HarnessNotRunEula,
    /// Not run: a pinned download failed or did not match its publisher hash.
    DownloadFailed,
    /// The mod (or the server around it) did not load, and no more specific cause was found.
    HarnessLoadFailure,
    /// A class or member was missing at runtime (`ClassNotFoundException`,
    /// `NoClassDefFoundError`, `NoSuchMethodError`, `NoSuchFieldError`).
    ClassNotFound,
    /// A mixin failed to apply.
    MixinFailure,
    /// A registry rejected an entry: duplicate, frozen or unbound.
    RegistryConflict,
    /// The server crashed.
    Crash,
    /// The server loaded, but the dumps differ.
    EntriesDiffer,
}

impl ReasonKind {
    /// Every kind, in declaration order.
    pub const ALL: [ReasonKind; 8] = [
        ReasonKind::HarnessNotRunEula,
        ReasonKind::DownloadFailed,
        ReasonKind::HarnessLoadFailure,
        ReasonKind::ClassNotFound,
        ReasonKind::MixinFailure,
        ReasonKind::RegistryConflict,
        ReasonKind::Crash,
        ReasonKind::EntriesDiffer,
    ];

    /// Log-derived kinds, most specific first. When a log matches several, the first wins:
    /// a mixin that fails because its target class is missing is a mixin failure, and a
    /// crash caused by a missing class is a missing class.
    pub const PRECEDENCE: [ReasonKind; 5] = [
        ReasonKind::MixinFailure,
        ReasonKind::ClassNotFound,
        ReasonKind::RegistryConflict,
        ReasonKind::HarnessLoadFailure,
        ReasonKind::Crash,
    ];

    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            ReasonKind::HarnessNotRunEula => "harness_not_run_eula",
            ReasonKind::DownloadFailed => "download_failed",
            ReasonKind::HarnessLoadFailure => "harness_load_failure",
            ReasonKind::ClassNotFound => "class_not_found",
            ReasonKind::MixinFailure => "mixin_failure",
            ReasonKind::RegistryConflict => "registry_conflict",
            ReasonKind::Crash => "crash",
            ReasonKind::EntriesDiffer => "entries_differ",
        }
    }

    /// One sentence for reports.
    pub const fn sentence(self) -> &'static str {
        match self {
            ReasonKind::HarnessNotRunEula => {
                "Not run: the Minecraft EULA has not been accepted for this harness."
            }
            ReasonKind::DownloadFailed => {
                "Not run: a pinned download failed or did not match its publisher hash."
            }
            ReasonKind::HarnessLoadFailure => "The mod did not load in the harness server.",
            ReasonKind::ClassNotFound => {
                "A class or member the mod needs was not found at runtime."
            }
            ReasonKind::MixinFailure => "A mixin failed to apply.",
            ReasonKind::RegistryConflict => {
                "A registry rejected an entry (duplicate, frozen or unbound)."
            }
            ReasonKind::Crash => "The server crashed.",
            ReasonKind::EntriesDiffer => {
                "The server loaded, but entries are missing, extra or changed."
            }
        }
    }
}

impl fmt::Display for ReasonKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Lower-case markers per kind. A marker is a fixed substring, matched case-insensitively.
const MARKERS: &[(ReasonKind, &[&str])] = &[
    (
        ReasonKind::MixinFailure,
        &[
            "mixinapplyerror",
            "mixintransformererror",
            "mixinprepareerror",
            "mixinpreprocessorexception",
            "invalidmixinexception",
            "invalidinjectionexception",
            "invalidaccessorexception",
            "mixin apply failed",
            "mixin prepare failed",
            "critical injection failure",
            "org.spongepowered.asm.mixin.throwables",
            "org.spongepowered.asm.mixin.transformer.throwables",
            "org.spongepowered.asm.mixin.injection.throwables",
        ],
    ),
    (
        ReasonKind::ClassNotFound,
        &[
            "java.lang.classnotfoundexception",
            "java.lang.noclassdeffounderror",
            "java.lang.nosuchmethoderror",
            "java.lang.nosuchfielderror",
        ],
    ),
    (
        ReasonKind::RegistryConflict,
        &[
            "registry is already frozen",
            "adding duplicate key",
            "adding duplicate value",
            "unbound values in registry",
            "duplicate registration",
            "already registered",
        ],
    ),
    (
        ReasonKind::HarnessLoadFailure,
        &[
            "mod loading has failed",
            "mod loading failures have been detected",
            "loading errors encountered",
            "modloadingexception",
            "incompatible mods found",
            "incompatible mod set",
            "mod resolution failed",
            "which is missing!",
            "missing or unsupported mandatory dependencies",
            "failed to load mod",
        ],
    ),
    (
        ReasonKind::Crash,
        &[
            "---- minecraft crash report ----",
            "exception in server tick loop",
            "encountered an unexpected exception",
            "failed to start the minecraft server",
            "a fatal error has been detected by the java runtime environment",
            "java.lang.outofmemoryerror",
            "java.lang.stackoverflowerror",
            "exception in thread",
            "crash report saved to",
        ],
    ),
];

/// A log level, as log4j prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// TRACE.
    Trace,
    /// DEBUG.
    Debug,
    /// INFO.
    Info,
    /// WARN (and java.util.logging's WARNING).
    Warn,
    /// ERROR (and java.util.logging's SEVERE).
    Error,
    /// FATAL.
    Fatal,
}

impl Level {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "TRACE" => Level::Trace,
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" | "WARNING" => Level::Warn,
            "ERROR" | "SEVERE" => Level::Error,
            "FATAL" => Level::Fatal,
            _ => return None,
        })
    }
}

/// One log event: its first line, the lines that continue it (a stack trace), and its level
/// when the line carried one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// 1-based line number of the first line.
    pub line: usize,
    /// The level, or `None` for text written outside the logger.
    pub level: Option<Level>,
    /// The message without the timestamp, thread, level and logger prefix.
    pub message: String,
    /// Continuation lines, verbatim.
    pub continuation: Vec<String>,
}

impl LogRecord {
    /// Whether classification looks at this record: ERROR, FATAL, or written outside the
    /// logger.
    pub fn is_relevant(&self) -> bool {
        matches!(self.level, None | Some(Level::Error) | Some(Level::Fatal))
    }
}

/// Splits a log prefix such as `[12:00:00] [Server thread/INFO]: `,
/// `[12:00:00] [main/WARN] (FabricLoader) ` or
/// `[03Nov2026 12:00:00.123] [main/ERROR] [net.minecraft.server.Main/]: ` from the message.
/// Returns the level and the message, or `None` when the line has no such prefix.
pub fn split_prefix(line: &str) -> Option<(Level, &str)> {
    let rest = line.trim_start().strip_prefix('[')?;
    let rest = &rest[rest.find(']')? + 1..];
    let rest = rest.trim_start().strip_prefix('[')?;
    let end = rest.find(']')?;
    let (_, level) = rest[..end].rsplit_once('/')?;
    let level = Level::parse(level.trim())?;
    let mut rest = rest[end + 1..].trim_start();
    if let Some(inner) = rest.strip_prefix('[')
        && let Some(close) = inner.find(']')
    {
        rest = &inner[close + 1..];
    } else if let Some(inner) = rest.strip_prefix('(')
        && let Some(close) = inner.find(')')
    {
        rest = &inner[close + 1..];
    }
    let rest = rest.trim_start();
    Some((level, rest.strip_prefix(':').unwrap_or(rest).trim_start()))
}

const STANDALONE_STARTS: [&str; 5] = [
    "exception in thread",
    "#",
    "error:",
    "error occurred during initialization of vm",
    "---- minecraft crash report ----",
];

/// Parses captured server output into records. A line with a log prefix starts a record;
/// a line without one continues the previous record (log4j prints a throwable after its
/// event), except JVM and launcher output such as `Exception in thread "main"`, which starts
/// a record of its own with no level.
pub fn parse_log(text: &str) -> Vec<LogRecord> {
    let mut records: Vec<LogRecord> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if let Some((level, message)) = split_prefix(line) {
            records.push(LogRecord {
                line: index + 1,
                level: Some(level),
                message: message.to_owned(),
                continuation: Vec::new(),
            });
            continue;
        }
        let lower = line.trim_start().to_ascii_lowercase();
        let standalone = STANDALONE_STARTS.iter().any(|s| lower.starts_with(s));
        match records.last_mut() {
            Some(last) if !standalone => last.continuation.push(line.to_owned()),
            _ => records.push(LogRecord {
                line: index + 1,
                level: None,
                message: line.trim().to_owned(),
                continuation: Vec::new(),
            }),
        }
    }
    records
}

/// One classified line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// The reason it points to.
    pub kind: ReasonKind,
    /// Where it came from: `server.log`, `crash-report`, `dump.log.errors`, ...
    pub source: String,
    /// 1-based line number in that source, when it has lines.
    pub line: Option<usize>,
    /// The matching line, trimmed to 240 characters.
    pub excerpt: String,
}

/// At most this many findings are kept per run.
pub const MAX_FINDINGS: usize = 40;

fn marker_kind(line: &str) -> Vec<ReasonKind> {
    let lower = line.to_ascii_lowercase();
    MARKERS
        .iter()
        .filter(|(_, markers)| markers.iter().any(|m| lower.contains(m)))
        .map(|(kind, _)| *kind)
        .collect()
}

fn excerpt(line: &str) -> String {
    const LIMIT: usize = 240;
    let line = line.trim();
    if line.chars().count() <= LIMIT {
        line.to_owned()
    } else {
        let cut: String = line.chars().take(LIMIT).collect();
        format!("{cut}...")
    }
}

/// Classifies the relevant records of a parsed log. One finding per kind per record, at the
/// first line that matches it.
pub fn classify_records(records: &[LogRecord], source: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for record in records.iter().filter(|r| r.is_relevant()) {
        let mut seen: Vec<ReasonKind> = Vec::new();
        let lines = std::iter::once((record.line, record.message.as_str())).chain(
            record
                .continuation
                .iter()
                .enumerate()
                .map(|(i, l)| (record.line + i + 1, l.as_str())),
        );
        for (number, line) in lines {
            for kind in marker_kind(line) {
                if !seen.contains(&kind) {
                    seen.push(kind);
                    findings.push(Finding {
                        kind,
                        source: source.to_owned(),
                        line: Some(number),
                        excerpt: excerpt(line),
                    });
                }
            }
        }
        if findings.len() >= MAX_FINDINGS {
            findings.truncate(MAX_FINDINGS);
            break;
        }
    }
    findings
}

/// Classifies captured server output (see [`parse_log`] and [`classify_records`]).
pub fn classify_log(text: &str, source: &str) -> Vec<Finding> {
    classify_records(&parse_log(text), source)
}

/// Classifies a crash report: every line counts, and the file itself is evidence of a crash.
pub fn classify_crash_report(text: &str, source: &str) -> Vec<Finding> {
    let mut findings = vec![Finding {
        kind: ReasonKind::Crash,
        source: source.to_owned(),
        line: None,
        excerpt: "a crash report was written".to_owned(),
    }];
    let mut seen = vec![ReasonKind::Crash];
    for (i, line) in text.lines().enumerate() {
        for kind in marker_kind(line) {
            if !seen.contains(&kind) {
                seen.push(kind);
                findings.push(Finding {
                    kind,
                    source: source.to_owned(),
                    line: Some(i + 1),
                    excerpt: excerpt(line),
                });
            }
        }
    }
    findings
}

/// Classifies log messages the probe captured (each string is one ERROR or FATAL event).
pub fn classify_messages<'a>(
    messages: impl IntoIterator<Item = &'a str>,
    source: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (i, message) in messages.into_iter().enumerate() {
        let mut seen: Vec<ReasonKind> = Vec::new();
        for line in message.lines() {
            for kind in marker_kind(line) {
                if !seen.contains(&kind) {
                    seen.push(kind);
                    findings.push(Finding {
                        kind,
                        source: source.to_owned(),
                        line: Some(i + 1),
                        excerpt: excerpt(line),
                    });
                }
            }
        }
        if findings.len() >= MAX_FINDINGS {
            findings.truncate(MAX_FINDINGS);
            break;
        }
    }
    findings
}

/// The reason the findings point to, by [`ReasonKind::PRECEDENCE`].
pub fn primary_reason(findings: &[Finding]) -> Option<ReasonKind> {
    ReasonKind::PRECEDENCE
        .into_iter()
        .find(|kind| findings.iter().any(|f| f.kind == *kind))
}

/// Normalises one log message for noise matching: only the first line, without the log
/// prefix, with numbers and long hexadecimal ids replaced by `#`, and whitespace collapsed.
/// Two runs that log "the same" message (differing only in timestamps, thread ids, counts or
/// object hashes) normalise to the same string.
pub fn normalize_message(message: &str) -> String {
    let first = message.lines().next().unwrap_or_default();
    let body = split_prefix(first).map_or(first, |(_, m)| m);
    let mut out = String::with_capacity(body.len());
    let mut token = String::new();
    for c in body.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_alphanumeric() {
            token.push(c);
            continue;
        }
        flush_token(&mut token, &mut out);
        if c.is_whitespace() {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out.trim_end().to_owned()
}

fn flush_token(token: &mut String, out: &mut String) {
    if token.is_empty() {
        return;
    }
    let has_digit = token.bytes().any(|b| b.is_ascii_digit());
    let hex = token.len() >= 8 && token.bytes().all(|b| b.is_ascii_hexdigit());
    let prefixed_hex = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit()));
    if (has_digit && hex) || prefixed_hex {
        out.push('#');
    } else {
        let mut in_digits = false;
        for c in token.chars() {
            if c.is_ascii_digit() {
                if !in_digits {
                    out.push('#');
                }
                in_digits = true;
            } else {
                out.push(c);
                in_digits = false;
            }
        }
    }
    token.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-written excerpts in the shape of real server logs. They are not copied from any
    // mod or loader log.

    const FABRIC_MISSING_CLASS: &str = "\
[17:40:01] [main/INFO] (FabricLoader) Loading 52 mods:
[17:40:02] [main/WARN] (FabricLoader) Optional integration class com.example.compat.Hook not present
java.lang.ClassNotFoundException: com.example.compat.Hook
\tat java.base/java.lang.ClassLoader.loadClass(ClassLoader.java:525)
[17:40:05] [main/ERROR] (Minecraft) Failed to start the minecraft server
java.lang.RuntimeException: Could not execute entrypoint stage 'main' due to errors, provided by 'examplemod'!
\tat net.fabricmc.loader.impl.FabricLoaderImpl.invokeEntrypoints(FabricLoaderImpl.java:403)
Caused by: java.lang.NoClassDefFoundError: net/fabricmc/fabric/api/example/v1/ExampleEvents
\t... 12 more
";

    const NEOFORGE_MIXIN: &str = "\
[03Nov2026 17:40:01.123] [main/INFO] [net.neoforged.fml.loading.FMLLoader/CORE]: Loading mods
[03Nov2026 17:40:09.456] [main/ERROR] [mixin/]: Mixin apply failed examplemod.mixins.json:ServerLevelMixin -> net.minecraft.server.level.ServerLevel: org.spongepowered.asm.mixin.injection.throwables.InvalidInjectionException Critical injection failure: @Inject annotation on onTick could not find any targets matching 'tick'
Caused by: java.lang.ClassNotFoundException: net.minecraft.class_3218
[03Nov2026 17:40:09.789] [main/FATAL] [net.minecraft.server.Main/FATAL]: Failed to start the minecraft server
";

    const REGISTRY_FROZEN: &str = "\
[17:40:03] [Server thread/ERROR]: Encountered an unexpected exception
java.lang.IllegalStateException: Registry is already frozen (trying to add key ResourceKey[minecraft:item / examplemod:widget])
\tat net.minecraft.core.MappedRegistry.register(MappedRegistry.java:120)
";

    const FABRIC_MISSING_DEPENDENCY: &str = "\
[17:40:01] [main/ERROR] (FabricLoader) Incompatible mods found!
net.fabricmc.loader.impl.FormattedException: Some of your mods are incompatible with the game or each other!
A potential solution has been determined, this may resolve your problem:
\t - Install cloth-config, any version.
";

    const JVM_STDERR: &str = "\
Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space
\tat java.base/java.util.Arrays.copyOf(Arrays.java:3537)
";

    #[test]
    fn prefixes_of_every_format_are_split() {
        assert_eq!(
            split_prefix("[17:40:01] [Server thread/INFO]: Starting minecraft server"),
            Some((Level::Info, "Starting minecraft server"))
        );
        assert_eq!(
            split_prefix("[17:40:01] [main/WARN] (FabricLoader) Mod x uses a deprecated api"),
            Some((Level::Warn, "Mod x uses a deprecated api"))
        );
        assert_eq!(
            split_prefix(
                "[03Nov2026 17:40:01.123] [main/ERROR] [net.minecraft.server.Main/]: boom"
            ),
            Some((Level::Error, "boom"))
        );
        assert_eq!(split_prefix("\tat java.base/Foo.bar(Foo.java:1)"), None);
        assert_eq!(split_prefix("[not] [a/LEVEL] line"), None);
        assert_eq!(split_prefix("[x"), None);
    }

    #[test]
    fn throwables_attach_to_their_event() {
        let records = parse_log(FABRIC_MISSING_CLASS);
        assert_eq!(records.len(), 3);
        assert_eq!(records[1].level, Some(Level::Warn));
        assert_eq!(records[1].continuation.len(), 2);
        assert_eq!(records[2].line, 5);
        assert_eq!(records[2].continuation.len(), 4);
        let stderr = parse_log(JVM_STDERR);
        assert_eq!(stderr.len(), 1);
        assert_eq!(stderr[0].level, None);
    }

    #[test]
    fn a_missing_class_at_error_is_class_not_found_and_the_warn_is_ignored() {
        let findings = classify_log(FABRIC_MISSING_CLASS, "server.log");
        assert_eq!(primary_reason(&findings), Some(ReasonKind::ClassNotFound));
        let cnf: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.kind == ReasonKind::ClassNotFound)
            .collect();
        assert_eq!(cnf.len(), 1);
        assert_eq!(
            cnf[0].line,
            Some(8),
            "the NoClassDefFoundError, not the WARN on line 3"
        );
        assert!(findings.iter().any(|f| f.kind == ReasonKind::Crash));
    }

    #[test]
    fn a_mixin_failure_wins_over_the_missing_class_behind_it() {
        let findings = classify_log(NEOFORGE_MIXIN, "server.log");
        assert_eq!(primary_reason(&findings), Some(ReasonKind::MixinFailure));
        assert!(findings.iter().any(|f| f.kind == ReasonKind::ClassNotFound));
    }

    #[test]
    fn registry_freezes_and_missing_dependencies_and_jvm_errors() {
        assert_eq!(
            primary_reason(&classify_log(REGISTRY_FROZEN, "server.log")),
            Some(ReasonKind::RegistryConflict)
        );
        assert_eq!(
            primary_reason(&classify_log(FABRIC_MISSING_DEPENDENCY, "server.log")),
            Some(ReasonKind::HarnessLoadFailure)
        );
        assert_eq!(
            primary_reason(&classify_log(JVM_STDERR, "server.log")),
            Some(ReasonKind::Crash)
        );
    }

    #[test]
    fn a_clean_log_has_no_reason() {
        let log = "\
[17:40:01] [Server thread/INFO]: Done (3.2s)! For help, type \"help\"
[17:40:02] [Server thread/WARN]: Can't keep up! Is the server overloaded?
[17:40:03] [main/DEBUG] (Mixin) java.lang.ClassNotFoundException probing optional target
";
        assert!(classify_log(log, "server.log").is_empty());
        assert_eq!(primary_reason(&[]), None);
    }

    #[test]
    fn crash_reports_always_count_as_a_crash() {
        let report = "---- Minecraft Crash Report ----\nDescription: Ticking entity\n\njava.lang.NullPointerException: x\n";
        let findings = classify_crash_report(report, "crash-reports/crash.txt");
        assert_eq!(primary_reason(&findings), Some(ReasonKind::Crash));
        let report = "---- Minecraft Crash Report ----\nCaused by: java.lang.NoSuchMethodError: 'void a.b()'\n";
        assert_eq!(
            primary_reason(&classify_crash_report(report, "c")),
            Some(ReasonKind::ClassNotFound)
        );
    }

    #[test]
    fn probe_messages_are_classified_one_event_each() {
        let messages = [
            "examplemod: Duplicate registration examplemod:widget",
            "something else",
        ];
        let findings = classify_messages(messages, "dump.log.errors");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, ReasonKind::RegistryConflict);
        assert_eq!(findings[0].line, Some(1));
    }

    #[test]
    fn normalisation_drops_what_varies_between_runs() {
        let a = normalize_message(
            "[17:40:01] [Worker-Main-3/WARN]: Took 1234 ms to load 56 chunks @ 0x7f3a2b1c",
        );
        let b = normalize_message(
            "[09:12:44] [Worker-Main-7/WARN]: Took 98 ms to load 7 chunks @ 0x1a2b3c4d",
        );
        assert_eq!(a, b);
        assert_eq!(a, "Took # ms to load # chunks @ #");
        assert_eq!(
            normalize_message("Missing entity 1a2b3c4d5e6f7a8b in level\n\tat x"),
            "Missing entity # in level"
        );
        assert_eq!(
            normalize_message("  many   spaces\there "),
            "many spaces here"
        );
        assert_ne!(
            normalize_message("Failed to load recipe examplemod:widget"),
            normalize_message("Failed to load recipe examplemod:gadget")
        );
    }

    #[test]
    fn ids_match_serde() {
        for kind in ReasonKind::ALL {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.id())
            );
            assert!(kind.sentence().ends_with('.'));
            assert!(!kind.sentence().to_lowercase().contains("guarantee"));
        }
    }
}
