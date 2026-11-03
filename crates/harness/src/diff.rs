//! Differences between two canonical dumps.
//!
//! The reference is the original mod on its own loader; the candidate is the converted mod on
//! the target loader. Every difference names a category, a kind and a path, so the same
//! difference learned from a baseline pair can be recognised and subtracted as loader noise
//! (see [`crate::noise`]).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canon::{canonical_eq, differing_pointers, first_difference};
use crate::dump::Dump;
use crate::logs::normalize_message;

/// Which part of the dump a difference is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// The loaded mod list (informational: loader mods differ by design).
    Mods,
    /// Registry entries and their order.
    Registries,
    /// Tag membership.
    Tags,
    /// Recipes.
    Recipes,
    /// Loot tables.
    LootTables,
    /// Advancements.
    Advancements,
    /// Worldgen block-state histogram.
    Worldgen,
    /// Server log events (path starts with `error` or `warning`).
    Log,
    /// Problems the probe reported about its own dump.
    ProbeErrors,
}

impl Category {
    /// Every category, in report order.
    pub const ALL: [Category; 9] = [
        Category::Mods,
        Category::Registries,
        Category::Tags,
        Category::Recipes,
        Category::LootTables,
        Category::Advancements,
        Category::Worldgen,
        Category::Log,
        Category::ProbeErrors,
    ];

    /// Whether differences in this category are measured entries (they count towards the
    /// missing, extra and changed totals of the score).
    pub const fn holds_entries(self) -> bool {
        matches!(
            self,
            Category::Registries
                | Category::Tags
                | Category::Recipes
                | Category::LootTables
                | Category::Advancements
                | Category::Worldgen
        )
    }

    /// Stable id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            Category::Mods => "mods",
            Category::Registries => "registries",
            Category::Tags => "tags",
            Category::Recipes => "recipes",
            Category::LootTables => "loot_tables",
            Category::Advancements => "advancements",
            Category::Worldgen => "worldgen",
            Category::Log => "log",
            Category::ProbeErrors => "probe_errors",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// What kind of difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    /// In the reference, not in the candidate.
    Missing,
    /// In the candidate, not in the reference.
    Extra,
    /// In both, with different content.
    Changed,
    /// The same entries in a different order.
    Order,
    /// A worldgen block-state count that moved by no more than the sample's run-to-run
    /// variation ([`within_worldgen_drift`]).
    Drift,
}

/// Worldgen counts may differ by this many blocks between runs of the same server.
pub const WORLDGEN_DRIFT_BLOCKS: u64 = 16;

/// Worldgen counts may differ by this share of the larger count between runs of the same
/// server.
pub const WORLDGEN_DRIFT_SHARE: f64 = 0.02;

/// Whether two counts of one block state differ by no more than the worldgen sample's
/// run-to-run variation: [`WORLDGEN_DRIFT_BLOCKS`] blocks or [`WORLDGEN_DRIFT_SHARE`] of the
/// larger count, whichever is more.
///
/// The same Fabric server with the same seed and probe, run twice in CI on 2026-11-03, gave
/// histograms that differed in 4 (1.21.1) and 15 (1.21.11) block states, by at most 32 blocks
/// (granite, 3768 against 3800) or 10 blocks on a small count (clay, 33 against 23), while
/// every other part of the dumps was identical. Counts inside this band are not evidence of a
/// conversion problem; counts outside it are.
pub fn within_worldgen_drift(a: u64, b: u64) -> bool {
    let larger = a.max(b);
    let difference = a.abs_diff(b);
    difference <= WORLDGEN_DRIFT_BLOCKS
        || (difference as f64) <= WORLDGEN_DRIFT_SHARE * larger as f64
}

/// One difference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Difference {
    /// Where.
    pub category: Category,
    /// What.
    pub kind: DiffKind,
    /// Identifies the entry within the category:
    /// registries `[registry]` or `[registry, entry]`; tags `[registry, tag]` or
    /// `[registry, tag, entry]`; recipes, loot tables and advancements `[id]`; worldgen
    /// `[block state]`; mods `[mod id]`; log `[level, normalised message]`; probe errors
    /// `[normalised message]`.
    pub path: Vec<String>,
    /// A human explanation (the first differing JSON pointer, two counts); not part of the
    /// identity used for noise matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// For a changed recipe, loot table or advancement: every JSON Pointer that differs, with
    /// array indices written `*` ([`crate::canon::differing_pointers`]). Noise matching uses
    /// it, so a loader that alters a vanilla entry does not hide a mod's own change to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pointers: Vec<String>,
}

impl Difference {
    fn new(category: Category, kind: DiffKind, path: Vec<String>) -> Self {
        Self {
            category,
            kind,
            path,
            detail: None,
            pointers: Vec::new(),
        }
    }

    fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }
}

/// Two dumps that cannot be compared, or a baseline that does not fit them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    /// The dumps were taken under different conditions (game version, seed, chunks).
    Incomparable(String),
    /// The baseline was learned for other loaders or versions.
    BaselineMismatch(String),
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareError::Incomparable(why) => write!(f, "the dumps cannot be compared: {why}"),
            CompareError::BaselineMismatch(why) => {
                write!(f, "the baseline does not fit these dumps: {why}")
            }
        }
    }
}

impl std::error::Error for CompareError {}

/// Checks that two dumps were taken under the same conditions.
pub fn check_comparable(reference: &Dump, candidate: &Dump) -> Result<(), CompareError> {
    if reference.game_version != candidate.game_version {
        return Err(CompareError::Incomparable(format!(
            "game version {} against {}",
            reference.game_version, candidate.game_version
        )));
    }
    // A side whose worldgen sampling failed has no seed; its histogram is then empty and the
    // probe's error says why, so the comparison goes on and shows the worldgen as missing.
    if let (Some(a), Some(b)) = (reference.worldgen.seed, candidate.worldgen.seed)
        && a != b
    {
        return Err(CompareError::Incomparable(format!(
            "worldgen seed {a} against {b}"
        )));
    }
    let chunks = |d: &Dump| {
        d.worldgen
            .chunks
            .iter()
            .copied()
            .collect::<BTreeSet<[i64; 2]>>()
    };
    if chunks(reference) != chunks(candidate) {
        return Err(CompareError::Incomparable(
            "the worldgen samples cover different chunks".to_owned(),
        ));
    }
    Ok(())
}

/// Every difference between two dumps, sorted. Fails when they are not comparable.
pub fn diff_dumps(reference: &Dump, candidate: &Dump) -> Result<Vec<Difference>, CompareError> {
    check_comparable(reference, candidate)?;
    let mut out = Vec::new();
    diff_mods(reference, candidate, &mut out);
    diff_registries(&reference.registries, &candidate.registries, &mut out);
    diff_tags(&reference.tags, &candidate.tags, &mut out);
    diff_json_maps(
        Category::Recipes,
        &reference.recipes,
        &candidate.recipes,
        &mut out,
    );
    diff_json_maps(
        Category::LootTables,
        &reference.loot_tables,
        &candidate.loot_tables,
        &mut out,
    );
    diff_json_maps(
        Category::Advancements,
        &reference.advancements,
        &candidate.advancements,
        &mut out,
    );
    diff_histogram(
        &reference.worldgen.block_histogram,
        &candidate.worldgen.block_histogram,
        &mut out,
    );
    diff_messages(
        Category::Log,
        Some("error"),
        &reference.log.errors,
        &candidate.log.errors,
        &mut out,
    );
    diff_messages(
        Category::Log,
        Some("warning"),
        &reference.log.warnings,
        &candidate.log.warnings,
        &mut out,
    );
    diff_messages(
        Category::ProbeErrors,
        None,
        &reference.errors,
        &candidate.errors,
        &mut out,
    );
    out.sort();
    out.dedup();
    Ok(out)
}

fn diff_mods(reference: &Dump, candidate: &Dump, out: &mut Vec<Difference>) {
    let a: BTreeMap<&str, &str> = reference
        .mods
        .iter()
        .map(|m| (m.id.as_str(), m.version.as_str()))
        .collect();
    let b: BTreeMap<&str, &str> = candidate
        .mods
        .iter()
        .map(|m| (m.id.as_str(), m.version.as_str()))
        .collect();
    for (id, version) in &a {
        match b.get(id) {
            None => out.push(Difference::new(
                Category::Mods,
                DiffKind::Missing,
                vec![(*id).to_owned()],
            )),
            Some(other) if other != version => out.push(
                Difference::new(Category::Mods, DiffKind::Changed, vec![(*id).to_owned()])
                    .with_detail(format!(
                        "version {version} in the reference, {other} in the candidate"
                    )),
            ),
            Some(_) => {}
        }
    }
    for id in b.keys().filter(|id| !a.contains_key(*id)) {
        out.push(Difference::new(
            Category::Mods,
            DiffKind::Extra,
            vec![(*id).to_owned()],
        ));
    }
}

fn diff_registries(
    a: &BTreeMap<String, Vec<String>>,
    b: &BTreeMap<String, Vec<String>>,
    out: &mut Vec<Difference>,
) {
    let ids: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for id in ids {
        let empty = Vec::new();
        let ea = a.get(id).unwrap_or(&empty);
        let eb = b.get(id).unwrap_or(&empty);
        if ea.is_empty() && eb.is_empty() {
            match (a.contains_key(id), b.contains_key(id)) {
                (true, false) => out.push(Difference::new(
                    Category::Registries,
                    DiffKind::Missing,
                    vec![id.clone()],
                )),
                (false, true) => out.push(Difference::new(
                    Category::Registries,
                    DiffKind::Extra,
                    vec![id.clone()],
                )),
                _ => {}
            }
            continue;
        }
        let sa: BTreeSet<&String> = ea.iter().collect();
        let sb: BTreeSet<&String> = eb.iter().collect();
        for entry in sa.difference(&sb) {
            out.push(Difference::new(
                Category::Registries,
                DiffKind::Missing,
                vec![id.clone(), (*entry).clone()],
            ));
        }
        for entry in sb.difference(&sa) {
            out.push(Difference::new(
                Category::Registries,
                DiffKind::Extra,
                vec![id.clone(), (*entry).clone()],
            ));
        }
        let common_a: Vec<&String> = ea.iter().filter(|e| sb.contains(e)).collect();
        let common_b: Vec<&String> = eb.iter().filter(|e| sa.contains(e)).collect();
        if common_a != common_b
            && let Some((i, (x, y))) = common_a
                .iter()
                .zip(common_b.iter())
                .enumerate()
                .find(|(_, (x, y))| x != y)
        {
            out.push(
                Difference::new(Category::Registries, DiffKind::Order, vec![id.clone()]).with_detail(format!(
                    "the shared entries first diverge at position {i}: {x} in the reference, {y} in the candidate"
                )),
            );
        }
    }
}

type Tags = BTreeMap<String, BTreeMap<String, Vec<String>>>;

fn diff_tags(a: &Tags, b: &Tags, out: &mut Vec<Difference>) {
    let empty = BTreeMap::new();
    let registries: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for registry in registries {
        let ta = a.get(registry).unwrap_or(&empty);
        let tb = b.get(registry).unwrap_or(&empty);
        let tags: BTreeSet<&String> = ta.keys().chain(tb.keys()).collect();
        for tag in tags {
            let none = Vec::new();
            let ma = ta.get(tag);
            let mb = tb.get(tag);
            let sa: BTreeSet<&String> = ma.unwrap_or(&none).iter().collect();
            let sb: BTreeSet<&String> = mb.unwrap_or(&none).iter().collect();
            if sa.is_empty() && sb.is_empty() {
                match (ma.is_some(), mb.is_some()) {
                    (true, false) => out.push(Difference::new(
                        Category::Tags,
                        DiffKind::Missing,
                        vec![registry.clone(), tag.clone()],
                    )),
                    (false, true) => out.push(Difference::new(
                        Category::Tags,
                        DiffKind::Extra,
                        vec![registry.clone(), tag.clone()],
                    )),
                    _ => {}
                }
                continue;
            }
            for entry in sa.difference(&sb) {
                out.push(Difference::new(
                    Category::Tags,
                    DiffKind::Missing,
                    vec![registry.clone(), tag.clone(), (*entry).clone()],
                ));
            }
            for entry in sb.difference(&sa) {
                out.push(Difference::new(
                    Category::Tags,
                    DiffKind::Extra,
                    vec![registry.clone(), tag.clone(), (*entry).clone()],
                ));
            }
        }
    }
}

fn diff_json_maps(
    category: Category,
    a: &BTreeMap<String, Value>,
    b: &BTreeMap<String, Value>,
    out: &mut Vec<Difference>,
) {
    for (id, va) in a {
        match b.get(id) {
            None => out.push(Difference::new(
                category,
                DiffKind::Missing,
                vec![id.clone()],
            )),
            Some(vb) if !canonical_eq(va, vb) => {
                let mut d = Difference::new(category, DiffKind::Changed, vec![id.clone()]);
                if let Some(detail) = first_difference(va, vb) {
                    d = d.with_detail(detail);
                }
                d.pointers = differing_pointers(va, vb);
                out.push(d);
            }
            Some(_) => {}
        }
    }
    for id in b.keys().filter(|id| !a.contains_key(*id)) {
        out.push(Difference::new(category, DiffKind::Extra, vec![id.clone()]));
    }
}

/// Block-state counts. A count that moved within the sample's run-to-run variation is
/// [`DiffKind::Drift`], also when it is absent on one side (a handful of mushrooms); beyond it,
/// a state on one side only is missing or extra and a different count is changed.
fn diff_histogram(a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>, out: &mut Vec<Difference>) {
    let states: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for state in states {
        let (ca, cb) = (a.get(state).copied(), b.get(state).copied());
        let (x, y) = (ca.unwrap_or(0), cb.unwrap_or(0));
        if ca == cb {
            continue;
        }
        let detail = format!("{x} in the reference, {y} in the candidate");
        let kind = if within_worldgen_drift(x, y) {
            DiffKind::Drift
        } else if cb.is_none() {
            DiffKind::Missing
        } else if ca.is_none() {
            DiffKind::Extra
        } else {
            DiffKind::Changed
        };
        out.push(
            Difference::new(Category::Worldgen, kind, vec![state.clone()]).with_detail(detail),
        );
    }
}

/// Messages the candidate logged that the reference did not, compared after
/// [`normalize_message`]. Fewer messages in the candidate are never a difference.
fn diff_messages(
    category: Category,
    level: Option<&str>,
    reference: &[String],
    candidate: &[String],
    out: &mut Vec<Difference>,
) {
    let seen: BTreeSet<String> = reference.iter().map(|m| normalize_message(m)).collect();
    let mut added = BTreeSet::new();
    for message in candidate {
        let normalized = normalize_message(message);
        if !seen.contains(&normalized) && added.insert(normalized.clone()) {
            let mut path: Vec<String> = level.map(str::to_owned).into_iter().collect();
            path.push(normalized);
            out.push(
                Difference::new(category, DiffKind::Extra, path).with_detail(first_line(message)),
            );
        }
    }
}

fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or_default().trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump::tests::sample;
    use crate::loader::Loader;
    use serde_json::json;

    fn dump(value: Value) -> Dump {
        Dump::from_value(&value).unwrap()
    }

    fn kinds(diffs: &[Difference]) -> Vec<(Category, DiffKind, Vec<&str>)> {
        diffs
            .iter()
            .map(|d| {
                (
                    d.category,
                    d.kind,
                    d.path.iter().map(String::as_str).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn identical_dumps_have_no_differences() {
        let a = dump(sample(Loader::Fabric));
        assert!(diff_dumps(&a, &a.clone()).unwrap().is_empty());
    }

    #[test]
    fn registries_report_missing_extra_and_order_of_shared_entries() {
        let mut b = sample(Loader::Fabric);
        b["registries"]["minecraft:item"] =
            json!(["minecraft:stick", "minecraft:air", "examplemod:widget"]);
        b["registries"]["neoforge:fluid_type"] = json!(["neoforge:empty"]);
        let a = dump(sample(Loader::Fabric));
        let diffs = diff_dumps(&a, &dump(b)).unwrap();
        assert_eq!(
            kinds(&diffs),
            vec![
                (
                    Category::Registries,
                    DiffKind::Missing,
                    vec!["minecraft:item", "minecraft:stone"]
                ),
                (
                    Category::Registries,
                    DiffKind::Extra,
                    vec!["minecraft:item", "examplemod:widget"]
                ),
                (
                    Category::Registries,
                    DiffKind::Extra,
                    vec!["neoforge:fluid_type", "neoforge:empty"]
                ),
                (
                    Category::Registries,
                    DiffKind::Order,
                    vec!["minecraft:item"]
                ),
            ]
        );
        let order = &diffs[3];
        assert_eq!(
            order.detail.as_deref(),
            Some(
                "the shared entries first diverge at position 0: minecraft:air in the reference, minecraft:stick in the candidate"
            )
        );
    }

    #[test]
    fn appended_entries_are_not_an_order_difference() {
        let mut b = sample(Loader::Fabric);
        b["registries"]["minecraft:item"] = json!([
            "neoforge:bucket",
            "minecraft:air",
            "minecraft:stone",
            "minecraft:stick"
        ]);
        let diffs = diff_dumps(&dump(sample(Loader::Fabric)), &dump(b)).unwrap();
        assert_eq!(
            kinds(&diffs),
            vec![(
                Category::Registries,
                DiffKind::Extra,
                vec!["minecraft:item", "neoforge:bucket"]
            )]
        );
    }

    #[test]
    fn tags_json_maps_and_worldgen() {
        let mut b = sample(Loader::Fabric);
        b["tags"]["minecraft:item"]["minecraft:logs"] =
            json!(["minecraft:oak_log", "minecraft:spruce_log"]);
        b["tags"]["minecraft:block"] = json!({"c:ores": []});
        b["recipes"]["minecraft:stick"]["result"]["count"] = json!(2);
        b["recipes"]["examplemod:widget"] = json!({"type": "minecraft:crafting_shapeless"});
        b["loot_tables"] = json!({});
        b["worldgen"]["block_histogram"]["minecraft:stone"] = json!(20);
        b["worldgen"]["block_histogram"]["minecraft:air"] = json!(110);
        b["worldgen"]["block_histogram"]["examplemod:ore"] = json!(40);
        let diffs = diff_dumps(&dump(sample(Loader::Fabric)), &dump(b)).unwrap();
        assert_eq!(
            kinds(&diffs),
            vec![
                (
                    Category::Tags,
                    DiffKind::Missing,
                    vec!["minecraft:item", "minecraft:logs", "minecraft:birch_log"]
                ),
                (
                    Category::Tags,
                    DiffKind::Extra,
                    vec!["minecraft:block", "c:ores"]
                ),
                (
                    Category::Tags,
                    DiffKind::Extra,
                    vec!["minecraft:item", "minecraft:logs", "minecraft:spruce_log"]
                ),
                (
                    Category::Recipes,
                    DiffKind::Extra,
                    vec!["examplemod:widget"]
                ),
                (
                    Category::Recipes,
                    DiffKind::Changed,
                    vec!["minecraft:stick"]
                ),
                (
                    Category::LootTables,
                    DiffKind::Missing,
                    vec!["minecraft:blocks/stone"]
                ),
                (Category::Worldgen, DiffKind::Extra, vec!["examplemod:ore"]),
                (
                    Category::Worldgen,
                    DiffKind::Changed,
                    vec!["minecraft:stone"]
                ),
                (Category::Worldgen, DiffKind::Drift, vec!["minecraft:air"]),
            ]
        );
        let changed = diffs
            .iter()
            .find(|d| d.category == Category::Recipes && d.kind == DiffKind::Changed)
            .unwrap();
        assert_eq!(
            changed.detail.as_deref(),
            Some("/result/count: 4 in the reference, 2 in the candidate")
        );
        assert_eq!(changed.pointers, ["/result/count"]);
    }

    #[test]
    fn worldgen_drift_follows_the_measured_run_to_run_variation() {
        // Pairs seen between two runs of the same server (docs/harness.md).
        for (a, b) in [(3768, 3800), (33, 23), (0, 6), (262_805, 262_825), (4, 3)] {
            assert!(within_worldgen_drift(a, b), "{a} {b}");
        }
        // A mod's ore that is gone, or vanilla stone that halved, is not drift.
        for (a, b) in [(40, 0), (0, 17), (50, 20), (10_000, 9_700)] {
            assert!(!within_worldgen_drift(a, b), "{a} {b}");
        }
    }

    #[test]
    fn only_new_log_messages_count_after_normalisation() {
        let mut a = sample(Loader::Fabric);
        a["log"]["warnings"] = json!(["[10:00:00] [main/WARN]: Took 12 ms"]);
        a["log"]["errors"] = json!(["only in the reference"]);
        let mut b = sample(Loader::NeoForge);
        b["log"]["warnings"] = json!(["Took 340 ms", "Config file examplemod.toml was rewritten"]);
        b["errors"] = json!(["could not encode recipe examplemod:widget"]);
        let diffs = diff_dumps(&dump(a), &dump(b)).unwrap();
        let log: Vec<_> = kinds(&diffs)
            .into_iter()
            .filter(|(c, _, _)| *c != Category::Mods)
            .collect();
        assert_eq!(
            log,
            vec![
                (
                    Category::Log,
                    DiffKind::Extra,
                    vec!["warning", "Config file examplemod.toml was rewritten"]
                ),
                (
                    Category::ProbeErrors,
                    DiffKind::Extra,
                    vec!["could not encode recipe examplemod:widget"]
                ),
            ]
        );
    }

    #[test]
    fn mods_differ_by_id_and_version() {
        let diffs = diff_dumps(
            &dump(sample(Loader::Fabric)),
            &dump(sample(Loader::NeoForge)),
        )
        .unwrap();
        let mods: Vec<_> = kinds(&diffs)
            .into_iter()
            .filter(|(c, _, _)| *c == Category::Mods)
            .collect();
        assert_eq!(
            mods,
            vec![
                (Category::Mods, DiffKind::Missing, vec!["fabric-api"]),
                (Category::Mods, DiffKind::Missing, vec!["fabricloader"]),
                (Category::Mods, DiffKind::Extra, vec!["neoforge"]),
            ]
        );
    }

    #[test]
    fn different_conditions_are_refused() {
        let a = dump(sample(Loader::Fabric));
        let mut b = sample(Loader::NeoForge);
        b["worldgen"]["seed"] = json!(1);
        assert!(matches!(
            diff_dumps(&a, &dump(b)),
            Err(CompareError::Incomparable(_))
        ));
        let mut b = sample(Loader::NeoForge);
        b["game_version"] = json!("1.21.11");
        assert!(diff_dumps(&a, &dump(b)).is_err());
        let mut b = sample(Loader::NeoForge);
        b["worldgen"]["chunks"] = json!([[0, 0]]);
        assert!(diff_dumps(&a, &dump(b)).is_err());
        let mut b = sample(Loader::NeoForge);
        b["worldgen"]["chunks"] = json!([[1, 0], [0, 0]]);
        assert!(
            diff_dumps(&a, &dump(b)).is_ok(),
            "chunk order does not matter"
        );
    }

    #[test]
    fn a_failed_worldgen_sample_shows_as_missing_entries() {
        let a = dump(sample(Loader::Fabric));
        let mut b = sample(Loader::NeoForge);
        b["worldgen"]["seed"] = Value::Null;
        b["worldgen"]["block_histogram"] = json!({});
        b["errors"] = json!(["worldgen: java.lang.RuntimeException: boom"]);
        let diffs = diff_dumps(&a, &dump(b)).unwrap();
        let missing = diffs
            .iter()
            .filter(|d| d.category == Category::Worldgen && d.kind == DiffKind::Missing)
            .count();
        assert_eq!(missing, 2, "air and stone; three oak logs are within drift");
        assert!(diffs.iter().any(|d| d.category == Category::ProbeErrors));
    }
}
