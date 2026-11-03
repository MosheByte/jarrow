//! End-to-end tests of the `jarrow-harness` binary. No network, no Java, no server.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

fn harness(args: &[&str], dir: &Path) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_jarrow-harness"))
        .args(args)
        .current_dir(dir)
        .env_remove("MINECRAFT_EULA_ACCEPTED")
        .env_remove("MINECRAFT_EULA")
        .env("JAVA_HOME", dir.join("no-jdk-here"))
        .output()?)
}

fn temp(tag: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!("jarrow-harness-cli-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A minimal valid dump.
fn dump(
    loader: &str,
    loader_version: &str,
    extra_registry: Option<(&str, &str)>,
    mods: &[&str],
) -> Value {
    let mut registries = json!({
        "minecraft:item": ["minecraft:air", "minecraft:stick"],
        "minecraft:block": ["minecraft:air", "minecraft:stone"]
    });
    if let Some((registry, entry)) = extra_registry {
        registries[registry] = json!([entry]);
    }
    let mut mod_list: Vec<Value> = mods
        .iter()
        .map(|id| json!({"id": id, "version": "1.0.0"}))
        .collect();
    mod_list.sort_by_key(|m| m["id"].as_str().unwrap_or_default().to_owned());
    json!({
        "schema": 1,
        "loader": loader,
        "loader_version": loader_version,
        "game_version": "1.21.1",
        "mods": mod_list,
        "registries": registries,
        "tags": {"minecraft:item": {"c:rods/wooden": ["minecraft:stick"]}},
        "recipes": {"minecraft:stick": {"type": "minecraft:crafting_shaped", "result": {"id": "minecraft:stick", "count": 4}}},
        "loot_tables": {},
        "advancements": {},
        "worldgen": {"seed": 20261103, "chunks": [[0, 0]], "block_histogram": {"minecraft:stone": 10}},
        "log": {"errors": [], "warnings": []},
        "errors": []
    })
}

fn write(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

#[test]
fn help_lists_the_commands_and_the_eula_gate() -> Result<(), Box<dyn Error>> {
    let dir = temp("help")?;
    let out = harness(&["--help"], &dir)?;
    assert!(out.status.success());
    let text = stdout(&out);
    for command in [
        "provision",
        "run",
        "baseline",
        "diff",
        "validate",
        "MINECRAFT_EULA_ACCEPTED",
    ] {
        assert!(text.contains(command), "{command} missing from {text}");
    }
    assert!(!text.to_lowercase().contains("guarantee"));
    let out = harness(&["run", "--help"], &dir)?;
    for flag in [
        "--loader",
        "--game-version",
        "--probe",
        "--mods",
        "--out",
        "--record",
        "--work",
        "--timeout-secs",
        "--xmx",
        "--cache",
        "--lock",
    ] {
        assert!(stdout(&out).contains(flag), "{flag}");
    }
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn validate_accepts_a_good_dump_and_lists_problems_in_a_bad_one() -> Result<(), Box<dyn Error>> {
    let dir = temp("validate")?;
    write(
        &dir.join("good.json"),
        &dump("fabric", "0.20.1", None, &["minecraft"]),
    )?;
    let out = harness(&["validate", "good.json"], &dir)?;
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("valid"));

    let mut bad = dump("fabric", "0.20.1", None, &["minecraft"]);
    bad["loader"] = json!("rift");
    bad.as_object_mut().ok_or("not an object")?.remove("tags");
    write(&dir.join("bad.json"), &bad)?;
    let out = harness(&["validate", "bad.json"], &dir)?;
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(
        text.contains("/loader") && text.contains("\"tags\""),
        "{text}"
    );
    fs::write(dir.join("junk.json"), "{")?;
    assert_eq!(
        harness(&["validate", "junk.json"], &dir)?.status.code(),
        Some(1)
    );
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn baseline_from_dumps_then_diff_scores_a_conversion() -> Result<(), Box<dyn Error>> {
    let dir = temp("diff")?;
    let fabric_mods = ["fabric-api", "fabricloader", "jarrow_probe", "minecraft"];
    let neo_mods = ["jarrow_probe", "minecraft", "neoforge"];
    write(
        &dir.join("base-fabric.json"),
        &dump("fabric", "0.20.1", None, &fabric_mods),
    )?;
    write(
        &dir.join("base-neoforge.json"),
        &dump(
            "neoforge",
            "21.1.265",
            Some(("neoforge:fluid_type", "neoforge:empty")),
            &neo_mods,
        ),
    )?;
    let out = harness(
        &[
            "baseline",
            "--game-version",
            "1.21.1",
            "--reference-dump",
            "base-fabric.json",
            "--candidate-dump",
            "base-neoforge.json",
            "--out",
            "noise.json",
        ],
        &dir,
    )?;
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    let noise: Value = serde_json::from_str(&fs::read_to_string(dir.join("noise.json"))?)?;
    assert_eq!(noise["record"], "jarrow-harness-noise");

    let with_mod = |mut v: Value, id: &str| -> Value {
        if let Some(items) = v["registries"]["minecraft:item"].as_array_mut() {
            items.push(json!("examplemod:widget"));
        }
        if let Some(mods) = v["mods"].as_array_mut() {
            mods.push(json!({"id": id, "version": "1.0.0"}));
        }
        v
    };
    write(
        &dir.join("original.json"),
        &with_mod(dump("fabric", "0.20.1", None, &fabric_mods), "examplemod"),
    )?;
    write(
        &dir.join("converted.json"),
        &with_mod(
            dump(
                "neoforge",
                "21.1.265",
                Some(("neoforge:fluid_type", "neoforge:empty")),
                &neo_mods,
            ),
            "examplemod",
        ),
    )?;
    let out = harness(
        &[
            "diff",
            "original.json",
            "converted.json",
            "--baseline",
            "noise.json",
            "--out",
            "report.json",
        ],
        &dir,
    )?;
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).starts_with("score 1 (green): Verified by the harness"),
        "{}",
        stdout(&out)
    );
    let report: Value = serde_json::from_str(&fs::read_to_string(dir.join("report.json"))?)?;
    assert_eq!(report["score"], json!(1.0));
    assert_eq!(report["outcome"], "green");
    assert_eq!(
        report["baseline"]["sha256"].as_str().map(str::len),
        Some(64)
    );

    // The same converted dump without its new item is red.
    let mut broken = with_mod(
        dump(
            "neoforge",
            "21.1.265",
            Some(("neoforge:fluid_type", "neoforge:empty")),
            &neo_mods,
        ),
        "examplemod",
    );
    broken["registries"]["minecraft:item"] = json!(["minecraft:air", "minecraft:stick"]);
    write(&dir.join("broken.json"), &broken)?;
    let out = harness(
        &[
            "diff",
            "original.json",
            "broken.json",
            "--baseline",
            "noise.json",
            "--out",
            "report2.json",
        ],
        &dir,
    )?;
    assert!(out.status.success());
    let report: Value = serde_json::from_str(&fs::read_to_string(dir.join("report2.json"))?)?;
    assert_eq!(report["score"], json!(0.25));
    assert_eq!(report["reason_kind"], "entries_differ");

    // A baseline for other loaders is refused rather than misapplied.
    let out = harness(
        &[
            "diff",
            "converted.json",
            "original.json",
            "--baseline",
            "noise.json",
            "--out",
            "x.json",
        ],
        &dir,
    )?;
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("baseline"), "{}", stderr(&out));
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn run_with_the_gate_closed_is_not_run_and_starts_nothing() -> Result<(), Box<dyn Error>> {
    let dir = temp("eula")?;
    fs::write(dir.join("probe.jar"), b"probe")?;
    fs::write(dir.join("mod.jar"), b"mod")?;
    let args = [
        "run",
        "--loader",
        "neoforge",
        "--game-version",
        "1.21.1",
        "--probe",
        "probe.jar",
        "--mods",
        "mod.jar",
        "--out",
        "out/dump.json",
        "--cache",
        "cache",
    ];
    let out = harness(&args, &dir)?;
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("not run [harness_not_run_eula]"),
        "{}",
        stdout(&out)
    );
    let record: Value = serde_json::from_str(&fs::read_to_string(dir.join("out/dump.run.json"))?)?;
    assert_eq!(record["status"], "not-run");
    assert_eq!(record["reason_kind"], "harness_not_run_eula");
    assert_eq!(record["loader_version"], "21.1.265");
    assert!(!dir.join("out/dump.json").exists());
    assert!(
        !dir.join("cache").exists(),
        "nothing was downloaded or installed"
    );

    // diff turns the not-run record into a not-run report, never a score.
    write(
        &dir.join("original.json"),
        &dump("fabric", "0.20.1", None, &["minecraft"]),
    )?;
    let out = harness(
        &[
            "diff",
            "original.json",
            "out/dump.run.json",
            "--out",
            "report.json",
        ],
        &dir,
    )?;
    assert!(out.status.success(), "{}", stderr(&out));
    let report: Value = serde_json::from_str(&fs::read_to_string(dir.join("report.json"))?)?;
    assert_eq!(report["outcome"], "not-run");
    assert_eq!(report["score"], Value::Null);
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn forge_and_quilt_say_they_are_not_built_yet() -> Result<(), Box<dyn Error>> {
    let dir = temp("forge")?;
    for loader in ["forge", "quilt"] {
        let out = harness(
            &[
                "provision",
                "--loader",
                loader,
                "--game-version",
                "1.21.1",
                "--cache",
                "cache",
            ],
            &dir,
        )?;
        assert_eq!(out.status.code(), Some(2));
        assert!(stderr(&out).contains("not built yet"), "{}", stderr(&out));
    }
    let out = harness(
        &[
            "provision",
            "--loader",
            "fabric",
            "--game-version",
            "1.7.10",
            "--cache",
            "cache",
        ],
        &dir,
    )?;
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("game_versions.1.7.10"),
        "{}",
        stderr(&out)
    );
    fs::remove_dir_all(&dir)?;
    Ok(())
}

/// `harness/probe/ci/baseline.sh` checks `baseline --help` for `--game-version`, `--out` and
/// `--probe-dir`, then runs `baseline --game-version <v> --out <dir>/<v> --probe-dir <probes>`.
#[test]
fn baseline_takes_the_interface_the_probe_ci_script_uses() -> Result<(), Box<dyn Error>> {
    let dir = temp("probe-ci")?;
    let out = harness(&["baseline", "--help"], &dir)?;
    let help = stdout(&out);
    for flag in ["--game-version ", "--out ", "--probe-dir "] {
        assert!(help.contains(flag), "{flag} missing from {help}");
    }
    let probes = dir.join("probes/probes-1.21.1");
    fs::create_dir_all(&probes)?;
    fs::write(
        probes.join("jarrow-probe-fabric-1.21.1-0.1.0.jar"),
        b"fabric probe",
    )?;
    fs::write(
        probes.join("jarrow-probe-neoforge-1.21.1-0.1.0.jar"),
        b"neoforge probe",
    )?;
    fs::write(probes.join("SHA256SUMS"), b"")?;
    fs::create_dir_all(dir.join("baseline/1.21.1"))?;
    let args = [
        "baseline",
        "--game-version",
        "1.21.1",
        "--out",
        "baseline/1.21.1",
        "--probe-dir",
        "probes/probes-1.21.1",
        "--cache",
        "cache",
    ];
    // The gate is closed here, so both sides are recorded as not run and nothing starts.
    let out = harness(&args, &dir)?;
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    for loader in ["fabric", "neoforge"] {
        let path = dir.join(format!("baseline/1.21.1/noise.{loader}.run.json"));
        let record: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        assert_eq!(record["status"], "not-run");
        assert_eq!(
            record["inputs"][0]["file"],
            format!("jarrow-probe-{loader}-1.21.1-0.1.0.jar")
        );
    }
    assert!(!dir.join("baseline/1.21.1/noise.json").exists());
    assert!(!dir.join("cache").exists());
    let out = harness(
        &[
            "baseline",
            "--game-version",
            "1.21.11",
            "--probe-dir",
            "probes/probes-1.21.1",
        ],
        &dir,
    )?;
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("no jarrow-probe-fabric-1.21.11-*.jar"),
        "{}",
        stderr(&out)
    );
    fs::remove_dir_all(&dir)?;
    Ok(())
}
