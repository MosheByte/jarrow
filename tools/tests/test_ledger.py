"""Tests for tools/ledger.py: the no-input ledger, rendering, validation and determinism."""
from __future__ import annotations

import copy
import json
import re
from pathlib import Path
from typing import Any

import pytest

import ledger

SHA = "0123456789abcdef0123456789abcdef01234567"

BOARD: dict[str, Any] = {
    "schema": 1,
    "run": "nightly-2026-11-04",
    "date": "2026-11-04",
    "commit": SHA,
    "game_versions": ["1.21.1", "1.21.11"],
    "mods": 1000,
    "green": 612,
    "yellow": 231,
    "red": 157,
    "not_run": 0,
    "harness": {"completed": 990, "attempted": 1000},
    "areas": [
        {"area": "registries", "measured": 800, "green": 700, "unsupported": None},
        {"area": "manifest", "measured": 1000, "green": 960, "unsupported": None},
        {"area": "rendering", "measured": 0, "green": 0, "unsupported": "deep rendering pipelines are out of v1"},
        {"area": "networking", "measured": 0, "green": 0, "unsupported": None},
    ],
    "red_reasons": [
        {"reason": "registry conflict", "count": 40},
        {"reason": "mixin target not found", "count": 61},
        {"reason": "class not found", "count": 40},
    ],
}
COVERAGE: dict[str, Any] = {
    "schema": 1,
    "date": "2026-11-04",
    "versions": [
        {
            "game_version": "1.21.1",
            "fabric_api": "0.116.7+1.21.1",
            "symbols": 5321,
            "with_rule": 1200,
            "tier_b": 300,
            "unsupported": 100,
            "weighted": 0.7412,
        }
    ],
}
WASM: dict[str, Any] = {
    "schema": 1,
    "commit": SHA,
    "raw_bytes": 5_012_345,
    "gzip_bytes": 1_834_567,
    "budget_gzip_bytes": 4_194_304,
}
PERF: dict[str, Any] = {
    "schema": 1,
    "date": "2026-11-04",
    "surfaces": [
        {"surface": "wasm", "machine": "2020 laptop", "samples": 25, "classify_ms_median": 120, "convert_ms_median": 6400},
        {"surface": "cli", "machine": "GitHub ubuntu-latest", "samples": 1000, "classify_ms_median": 41, "convert_ms_median": 1830},
    ],
}
FILES = {"board-summary.json": BOARD, "coverage.json": COVERAGE, "wasm.json": WASM, "perf.json": PERF}


def write_inputs(root: Path, files: dict[str, Any], indent: int | None = 2, sort: bool = False) -> None:
    inputs = root / "ledger" / "inputs"
    inputs.mkdir(parents=True, exist_ok=True)
    for name, data in files.items():
        (inputs / name).write_text(json.dumps(data, indent=indent, sort_keys=sort), encoding="utf-8")


def section(text: str, title: str) -> str:
    match = re.search(rf"^## {re.escape(title)}\n(.*?)(?=^## |\Z)", text, re.MULTILINE | re.DOTALL)
    assert match, f"missing section {title!r}"
    return match.group(1)


SECTIONS = ["What works", "Partial", "Unsupported", "Numbers from the last nightly", "Open red reasons"]


# ------------------------------------------------------------------------ no input


def test_no_inputs_says_no_nightly_everywhere(tmp_path: Path) -> None:
    text = ledger.render(tmp_path)
    assert text.splitlines()[0] == ledger.HEADER
    assert text.endswith("\n") and not text.endswith("\n\n")
    assert "\r" not in text
    for title in SECTIONS:
        assert "No nightly has run yet." in section(text, title), title
    numbers = section(text, "Numbers from the last nightly")
    for milestone in ("M1", "M2", "M3"):
        assert milestone in numbers
    assert numbers.count("not measured yet") == 6
    assert "guarantee" not in text.lower()


def test_no_inputs_matches_the_committed_ledger() -> None:
    root = Path(__file__).resolve().parents[2]
    committed = root / "LEDGER.md"
    if not (root / "ledger" / "inputs").exists() and committed.exists():
        assert committed.read_text(encoding="utf-8").replace("\r\n", "\n") == ledger.render(root)


def test_empty_inputs_dir_is_the_same_as_none(tmp_path: Path) -> None:
    (tmp_path / "ledger" / "inputs").mkdir(parents=True)
    assert ledger.render(tmp_path) == ledger.render(tmp_path / "nowhere")


# --------------------------------------------------------------------- determinism


def test_render_is_deterministic(tmp_path: Path) -> None:
    write_inputs(tmp_path, FILES)
    first = ledger.render(tmp_path)
    assert first == ledger.render(tmp_path)


def test_formatting_of_inputs_does_not_matter(tmp_path: Path) -> None:
    a, b = tmp_path / "a", tmp_path / "b"
    write_inputs(a, FILES, indent=2)
    shuffled = copy.deepcopy(FILES)
    shuffled["board-summary.json"]["areas"].reverse()
    shuffled["board-summary.json"]["red_reasons"].reverse()
    shuffled["perf.json"]["surfaces"].reverse()
    write_inputs(b, shuffled, indent=None, sort=True)
    assert ledger.render(a) == ledger.render(b)


def test_write_and_stdout_agree(tmp_path: Path, capsysbinary: pytest.CaptureFixture[bytes]) -> None:
    write_inputs(tmp_path, FILES)
    assert ledger.main(["--root", str(tmp_path)]) == 0
    written = (tmp_path / "LEDGER.md").read_bytes()
    assert ledger.main(["--root", str(tmp_path), "--stdout"]) == 0
    assert capsysbinary.readouterr().out == written
    assert b"\r\n" not in written


# -------------------------------------------------------------------------- render


def test_full_inputs_render_every_number(tmp_path: Path) -> None:
    write_inputs(tmp_path, FILES)
    text = ledger.render(tmp_path)
    assert "No nightly has run yet" not in text
    assert "Last nightly: `nightly-2026-11-04` on 2026-11-04, commit `0123456789ab`" in text
    numbers = section(text, "Numbers from the last nightly")
    assert "612 green (61.2%), 231 yellow (23.1%), 157 red (15.7%), 0 not run, of 1,000 mods" in numbers
    assert "99.0% (990 of 1,000 runs produced a score)" in numbers
    assert "74.1% weighted; 1,200 of 5,321 symbols have a rule" in numbers
    assert "1,830 ms convert, 41 ms classify (median of 1,000, GitHub ubuntu-latest)" in numbers
    assert "6,400 ms convert" in numbers
    assert "1.75 MiB gzipped (1,834,567 bytes; 4.78 MiB raw), within the 4.00 MiB budget" in numbers
    assert "not measured yet" not in numbers


def test_areas_are_classified_by_harness_numbers(tmp_path: Path) -> None:
    write_inputs(tmp_path, {"board-summary.json": BOARD})
    text = ledger.render(tmp_path)
    works = section(text, "What works")
    partial = section(text, "Partial")
    unsupported = section(text, "Unsupported")
    assert "| manifest | 960 of 1,000 | 96.0% |" in works
    assert "registries" not in works
    assert "| registries | 700 of 800 | 87.5% |" in partial
    assert "| networking | not measured | n/a |" in partial
    assert "| rendering | deep rendering pipelines are out of v1 |" in unsupported


@pytest.mark.parametrize(("green", "status"), [(95, "What works"), (94, "Partial")])
def test_works_threshold_is_95_percent(tmp_path: Path, green: int, status: str) -> None:
    board = copy.deepcopy(BOARD)
    board["areas"] = [{"area": "manifest", "measured": 100, "green": green, "unsupported": None}]
    write_inputs(tmp_path, {"board-summary.json": board})
    assert "| manifest |" in section(ledger.render(tmp_path), status)


def test_red_reasons_are_sorted_and_counted(tmp_path: Path) -> None:
    write_inputs(tmp_path, {"board-summary.json": BOARD})
    reasons = section(ledger.render(tmp_path), "Open red reasons")
    rows = [line for line in reasons.splitlines() if line.startswith("| ") and "---" not in line][1:]
    assert rows == [
        "| mixin target not found | 61 |",
        "| class not found | 40 |",
        "| registry conflict | 40 |",
    ]
    assert "16 red mods have no classified reason yet." in reasons


def test_long_reason_lists_are_summarised(tmp_path: Path) -> None:
    board = copy.deepcopy(BOARD)
    board["red_reasons"] = [{"reason": f"reason {i:02d}", "count": 2} for i in range(60)]
    board["red"] = 120
    board["yellow"] = 268
    write_inputs(tmp_path, {"board-summary.json": board})
    reasons = section(ledger.render(tmp_path), "Open red reasons")
    assert reasons.count("| reason ") == ledger.MAX_RED_REASONS
    assert "And 10 more reasons covering 20 red mods." in reasons


def test_markdown_in_values_is_escaped(tmp_path: Path) -> None:
    board = copy.deepcopy(BOARD)
    board["red_reasons"] = [{"reason": "a | b", "count": 1}]
    write_inputs(tmp_path, {"board-summary.json": board})
    assert "| a \\| b | 1 |" in ledger.render(tmp_path)


def test_wasm_over_budget_is_said(tmp_path: Path) -> None:
    wasm = dict(WASM, gzip_bytes=5_000_000)
    write_inputs(tmp_path, {"wasm.json": wasm})
    assert "OVER the 4.00 MiB budget" in ledger.render(tmp_path)


def test_partial_inputs_keep_missing_numbers_honest(tmp_path: Path) -> None:
    write_inputs(tmp_path, {"wasm.json": WASM})
    text = ledger.render(tmp_path)
    numbers = section(text, "Numbers from the last nightly")
    assert "| Board totals | not measured yet | M1" in numbers
    assert "within the 4.00 MiB budget" in numbers
    assert "No nightly has run yet." in section(text, "What works")


# ---------------------------------------------------------------------- validation


def mutate(path: list[Any], value: Any) -> dict[str, Any]:
    board = copy.deepcopy(BOARD)
    target: Any = board
    for key in path[:-1]:
        target = target[key]
    if value is KeyError:
        del target[path[-1]]
    else:
        target[path[-1]] = value
    return board


@pytest.mark.parametrize(
    ("board", "message"),
    [
        (mutate(["schema"], 2), "schema must be 1"),
        (mutate(["green"], 611), "green + yellow + red + not_run is 999, but mods is 1000"),
        (mutate(["green"], -1), "green must be a non-negative integer"),
        (mutate(["green"], True), "green must be a non-negative integer"),
        (mutate(["date"], "2026-11-4"), "date must match"),
        (mutate(["commit"], "abc"), "commit must match"),
        (mutate(["run"], KeyError), "run is required"),
        (mutate(["surprise"], 1), "has unknown keys: surprise"),
        (mutate(["harness", "completed"], 1001), "completed exceeds attempted"),
        (mutate(["areas", 0, "green"], 900), "areas[0] green exceeds measured"),
        (mutate(["areas", 1, "area"], "registries"), "repeats area 'registries'"),
        (mutate(["red_reasons", 0, "count"], 100), "red_reasons count 201 red mods, but red is 157"),
        (mutate(["game_versions"], ["1.21.1", ""]), "game_versions must be a list of non-empty strings"),
        (mutate(["areas", 0, "unsupported"], ""), "unsupported must be null or a non-empty string"),
    ],
)
def test_invalid_board_is_rejected(tmp_path: Path, board: dict[str, Any], message: str) -> None:
    write_inputs(tmp_path, {"board-summary.json": board})
    with pytest.raises(ledger.LedgerInputError) as err:
        ledger.render(tmp_path)
    assert str(err.value).startswith("ledger/inputs/board-summary.json")
    assert message in str(err.value)


def test_invalid_json_is_rejected(tmp_path: Path) -> None:
    inputs = tmp_path / "ledger" / "inputs"
    inputs.mkdir(parents=True)
    (inputs / "coverage.json").write_text("{not json", encoding="utf-8")
    with pytest.raises(ledger.LedgerInputError, match="not valid UTF-8 JSON"):
        ledger.render(tmp_path)


def test_other_inputs_are_validated(tmp_path: Path) -> None:
    cases = [
        ({"coverage.json": dict(COVERAGE, versions=[dict(COVERAGE["versions"][0], weighted=1.5)])}, "weighted must be a number from 0 to 1"),
        ({"coverage.json": dict(COVERAGE, versions=[dict(COVERAGE["versions"][0], tier_b=9999)])}, "above symbols"),
        ({"wasm.json": dict(WASM, gzip_bytes="big")}, "gzip_bytes must be a non-negative integer"),
        ({"perf.json": dict(PERF, surfaces=[dict(PERF["surfaces"][0], surface="gpu")])}, 'surface "cli" or "wasm"'),
        ({"perf.json": [1, 2]}, "must be a JSON object"),
    ]
    for i, (files, message) in enumerate(cases):
        root = tmp_path / str(i)
        write_inputs(root, files)
        with pytest.raises(ledger.LedgerInputError) as err:
            ledger.render(root)
        assert message in str(err.value), (files, str(err.value))


def test_main_reports_errors(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    write_inputs(tmp_path, {"wasm.json": {"schema": 1}})
    assert ledger.main(["--root", str(tmp_path)]) == 1
    assert "ledger: ledger/inputs/wasm.json" in capsys.readouterr().err
    assert not (tmp_path / "LEDGER.md").exists()


# -------------------------------------------------------------------- contributors

CONTRIBUTORS: dict[str, Any] = {
    "schema": 1,
    "as_of": "2026-11-03",
    "window_days": 30,
    "sources": ["git", "github-pulls"],
    "contributors_30d": {
        "people": {"count": 2, "logins": ["MosheByte", "newbie"]},
        "agents": {"count": 1, "names": ["Claude Opus 5.5"]},
    },
    "first_time_contributors_30d": {"count": 1, "logins": ["newbie"]},
}


def rows(text: str, title: str) -> list[str]:
    return [line for line in section(text, title).splitlines() if line.startswith("| ") and "---" not in line]


def test_contributors_without_input_say_no_data_yet(tmp_path: Path) -> None:
    text = ledger.render(tmp_path)
    assert rows(text, "Contributors") == [
        "| Number | Value |",
        "| Contributors (30 days) | No data yet |",
        "| First-time contributors (30 days) | No data yet |",
    ]
    assert "`tools/contributors.py --ledger`" in section(text, "Contributors")
    assert text.endswith("\n") and not text.endswith("\n\n")


def test_contributors_render_counts_and_logins(tmp_path: Path) -> None:
    write_inputs(tmp_path, {"contributors.json": CONTRIBUTORS})
    text = ledger.render(tmp_path)
    contrib = section(text, "Contributors")
    assert "the 30 days to 2026-11-03 (UTC)" in contrib
    assert "merged pull requests (GitHub) and commit authors and co-authors (git)" in contrib
    assert "Bots are not counted." in contrib
    assert rows(text, "Contributors")[1:] == [
        "| Contributors (30 days) | 2 people (MosheByte, newbie); 1 agent (Claude Opus 5.5) |",
        "| First-time contributors (30 days) | 1 person (newbie) |",
    ]
    # Contributor numbers alone are not a nightly: the board sections still say none has run.
    assert "No nightly has run yet. Each number below" in section(text, "Numbers from the last nightly")
    assert "No nightly has run yet." in section(text, "What works")


def test_contributors_zero_git_only_and_long_lists(tmp_path: Path) -> None:
    data = copy.deepcopy(CONTRIBUTORS)
    data["sources"] = ["git"]
    data["first_time_contributors_30d"] = None
    data["contributors_30d"]["people"] = {"count": 0, "logins": []}
    write_inputs(tmp_path / "a", {"contributors.json": data})
    text = ledger.render(tmp_path / "a")
    assert "from commit authors and co-authors (git) only" in section(text, "Contributors")
    assert rows(text, "Contributors")[1:] == [
        "| Contributors (30 days) | 0 people; 1 agent (Claude Opus 5.5) |",
        "| First-time contributors (30 days) | not measured: the merged pull request lookup was unavailable |",
    ]
    many = [f"user{i:02d}" for i in range(35)]
    data["contributors_30d"]["people"] = {"count": 35, "logins": many}
    write_inputs(tmp_path / "b", {"contributors.json": data})
    line = rows(ledger.render(tmp_path / "b"), "Contributors")[1]
    assert line.startswith("| Contributors (30 days) | 35 people (user00, user01,")
    assert "user29 and 5 more); 1 agent" in line and "user30" not in line


@pytest.mark.parametrize(
    ("path", "value", "message"),
    [
        (["contributors_30d", "people", "count"], 3, "count is 3, but logins has 2"),
        (["contributors_30d", "agents", "names"], ["a", "a"], "names repeats a name"),
        (["first_time_contributors_30d", "logins"], ["newbie", ""], "must be a list of non-empty strings"),
        (["sources"], ["git", "crystal-ball"], "names unknown sources: crystal-ball"),
        (["as_of"], "2026-11-3", "as_of must match"),
        (["first_time_contributors_30d"], KeyError, "first_time_contributors_30d is required"),
        (["contributors_30d", "bots"], {"count": 0, "logins": []}, "has unknown keys: bots"),
    ],
)
def test_invalid_contributors_are_rejected(tmp_path: Path, path: list[str], value: Any, message: str) -> None:
    data = copy.deepcopy(CONTRIBUTORS)
    target: Any = data
    for key in path[:-1]:
        target = target[key]
    if value is KeyError:
        del target[path[-1]]
    else:
        target[path[-1]] = value
    write_inputs(tmp_path, {"contributors.json": data})
    with pytest.raises(ledger.LedgerInputError) as err:
        ledger.render(tmp_path)
    assert str(err.value).startswith("ledger/inputs/contributors.json")
    assert message in str(err.value)
