#!/usr/bin/env python3
"""Summarise an lcov file: the total line coverage and each crate's, as Markdown.

The coverage job in .github/workflows/ci.yml runs this on the lcov.info that
`cargo llvm-cov --workspace --lcov` writes, the same file it uploads to Codecov.

Usage:
    python tools/coverage_summary.py lcov.info [--root DIR]

Prints the summary, appends it to $GITHUB_STEP_SUMMARY when that is set, and prints a
`::notice` with the total for the run's annotations. Lines are lcov's LF (instrumented lines)
and LH (lines hit), summed per crates/<name> directory; files outside the repository count
under "other". Informational only: the exit status never depends on the number.

Exit status: 0 written, 1 the file is unreadable or lists no instrumented lines.
Standard library only.
"""
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path


def per_area(lcov: str, root: Path) -> dict[str, list[int]]:
    """{"crates/<name>" or top-level directory: [lines, lines hit]}."""
    root = root.resolve()
    totals: dict[str, list[int]] = {}
    area = "other"
    for line in lcov.splitlines():
        if line.startswith("SF:"):
            path = Path(line[3:].strip())
            path = (root / path if not path.is_absolute() else path).resolve()
            parts = path.relative_to(root).parts if path.is_relative_to(root) else ("other",)
            area = "/".join(parts[:2]) if parts[0] == "crates" and len(parts) > 2 else parts[0]
        elif line.startswith(("LF:", "LH:")):
            counts = totals.setdefault(area, [0, 0])
            counts[0 if line.startswith("LF:") else 1] += int(line[3:])
    return totals


def percent(hit: int, lines: int) -> str:
    return f"{100 * hit / lines:.2f}%" if lines else "n/a"


def render(totals: dict[str, list[int]]) -> tuple[str, str]:
    """The Markdown summary and the total as text."""
    lines = sum(v[0] for v in totals.values())
    hit = sum(v[1] for v in totals.values())
    total = percent(hit, lines)
    out = [
        f"### Line coverage: {total}",
        "",
        f"Total line coverage of the workspace tests: **{total}** ({hit} of {lines} lines), from `cargo llvm-cov`. "
        "Informational: no check fails on it yet. The 80% floor on `crates/` is enforced once the numbers settle "
        "(docs/releases.md, Test coverage).",
        "",
        "| Path | Lines | Hit | Coverage |",
        "| --- | ---: | ---: | ---: |",
    ]
    out += [f"| `{name}` | {n} | {h} | {percent(h, n)} |" for name, (n, h) in sorted(totals.items())]
    return "\n".join(out) + "\n", f"{total} ({hit} of {lines} lines)"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("lcov", type=Path)
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="the repository (default: the current directory)")
    args = parser.parse_args(argv)
    try:
        totals = per_area(args.lcov.read_text(encoding="utf-8"), args.root)
    except (OSError, ValueError) as err:
        print(f"coverage_summary: cannot read {args.lcov}: {err}", file=sys.stderr)
        return 1
    if not sum(v[0] for v in totals.values()):
        print(f"coverage_summary: {args.lcov} lists no instrumented lines", file=sys.stderr)
        return 1
    summary, total = render(totals)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as out:
            out.write(summary)
    sys.stdout.write(summary)
    print(f"::notice title=Line coverage::{total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
