"""Tests for tools/coverage_summary.py: per-crate sums, the total and the job summary."""
from __future__ import annotations

from pathlib import Path

import pytest

import coverage_summary


def lcov(root: Path) -> str:
    return "\n".join(
        [
            f"SF:{root / 'crates' / 'jar' / 'src' / 'lib.rs'}",
            "DA:1,1",
            "LF:100",
            "LH:85",
            "end_of_record",
            "SF:crates/jar/src/read.rs",  # relative paths are read against --root
            "LF:50",
            "LH:20",
            "end_of_record",
            f"SF:{root / 'crates' / 'cli' / 'src' / 'main.rs'}",
            "LF:10",
            "LH:10",
            "end_of_record",
            f"SF:{root.parent / 'elsewhere' / 'core.rs'}",
            "LF:5",
            "LH:0",
            "end_of_record",
            "",
        ]
    )


def test_sums_per_crate_and_in_total(tmp_path: Path) -> None:
    totals = coverage_summary.per_area(lcov(tmp_path), tmp_path)
    assert totals == {"crates/jar": [150, 105], "crates/cli": [10, 10], "other": [5, 0]}
    summary, total = coverage_summary.render(totals)
    assert total == "69.70% (115 of 165 lines)"
    assert summary.startswith("### Line coverage: 69.70%\n")
    assert "| `crates/cli` | 10 | 10 | 100.00% |" in summary
    assert "| `crates/jar` | 150 | 105 | 70.00% |" in summary
    assert "80% floor on `crates/`" in summary


def test_main_appends_to_the_job_summary(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    info = tmp_path / "lcov.info"
    info.write_text(lcov(tmp_path), encoding="utf-8")
    step = tmp_path / "summary.md"
    step.write_text("earlier step\n", encoding="utf-8")
    monkeypatch.setenv("GITHUB_STEP_SUMMARY", str(step))
    assert coverage_summary.main([str(info), "--root", str(tmp_path)]) == 0
    out = capsys.readouterr().out
    assert out.endswith("::notice title=Line coverage::69.70% (115 of 165 lines)\n")
    assert step.read_text(encoding="utf-8").startswith("earlier step\n### Line coverage: 69.70%\n")


def test_main_fails_without_instrumented_lines(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("GITHUB_STEP_SUMMARY", raising=False)
    info = tmp_path / "lcov.info"
    info.write_text("SF:crates/jar/src/lib.rs\nend_of_record\n", encoding="utf-8")
    assert coverage_summary.main([str(info), "--root", str(tmp_path)]) == 1
    assert coverage_summary.main([str(tmp_path / "missing.info")]) == 1
