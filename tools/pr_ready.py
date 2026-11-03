#!/usr/bin/env python3
"""Checks every change to Jarrow must pass before it merges (book section 0, rule 2).

Usage:
    python tools/pr_ready.py --base <sha> --head <sha>   PR mode: the commits in base..head
                                                         and the tracked tree
    python tools/pr_ready.py --all                       push mode: the tracked tree only
    options: --check NAME (repeatable) runs only the named checks; --repo DIR picks the
             repository (default: the git top level of the current directory)

Checks (names as accepted by --check):
    trailer        every commit in base..head carries the git trailer
                   "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" in its final
                   paragraph, unless its author name ends with "[bot]"           (PR mode)
    artifacts      no tracked .jar, .class, .war or .ear file, no zip archive holding
                   classes, and no JVM source that carries a decompiler header comment
    research-lock  a change to research-lock.json comes with a DECISIONS.md change that
                   adds a new "## D-<n>" entry                                  (PR mode)
    decisions      DECISIONS.md headings read "## D-<3 digits> — <title>" (em dash), IDs
                   run D-001, D-002, ... with no gaps, and every entry has lines starting
                   **Date:**, **Decision:**, **Why:**, **Reversal cost:** and **Links:**
    ledger         LEDGER.md equals the output of "python tools/ledger.py --stdout"
    architecture   every crates/<name>/ with a Cargo.toml is named literally as
                   "crates/<name>" in docs/architecture.md
    secrets        no tracked text file contains a GitHub, npm, GitLab, Slack or AWS
                   token, a private key block, a Discord or Slack webhook URL, or a literal
                   CLOUDFLARE_API_TOKEN value. Matches are reported by file and line; the
                   value itself is never printed.

Exit status: 0 when every check passes, 1 when any fails, 2 on a usage or git error.
Standard library only. Files are read as bytes; anything that is not UTF-8 is binary.
Paths under .git/, node_modules/ and target/ are never inspected.
"""
from __future__ import annotations

import argparse
import io
import os
import re
import subprocess
import sys
import zipfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ledger  # noqa: E402  (sibling module in tools/)

TRAILER_KEY = "Co-Authored-By"
TRAILER_VALUE = "Claude Opus 5.5 <noreply@anthropic.com>"
TRAILER = f"{TRAILER_KEY}: {TRAILER_VALUE}"

SKIP_DIRS = frozenset({".git", "node_modules", "target"})
BANNED_EXTENSIONS = (".jar", ".class", ".war", ".ear")
JVM_SOURCE_EXTENSIONS = (".java", ".jav", ".kt", ".kts", ".groovy", ".scala")
HEADER_LINES = 40
FOOTER_LINES = 15
MAX_TEXT_BYTES = 20 * 1024 * 1024

DECISIONS = "DECISIONS.md"
RESEARCH_LOCK = "research-lock.json"
LEDGER = "LEDGER.md"
ARCHITECTURE = "docs/architecture.md"

DECISION_HEADING = re.compile(r"^## D-(\d{3}) — (\S.*)$")
DECISION_ID = re.compile(r"^## D-(\d+)\b")
DECISION_FIELDS = ("**Date:**", "**Decision:**", "**Why:**", "**Reversal cost:**", "**Links:**")

JAVA_PACKAGE = re.compile(r"^\s*package\s+[A-Za-z_][\w.]*\s*;", re.MULTILINE)
DECOMPILER_MARK = re.compile(
    r"decompil|recreated from a \.class file|"
    r"\b(?:vineflower|fernflower|quiltflower|forgeflower|cfr|procyon|jd-core|jadx|krakatau)\b",
    re.IGNORECASE,
)

SECRET_PATTERNS: tuple[tuple[str, re.Pattern[bytes]], ...] = (
    ("a GitHub token", re.compile(rb"\bgh[pousr]_[A-Za-z0-9]{36,}")),
    ("a GitHub fine-grained token", re.compile(rb"\bgithub_pat_[A-Za-z0-9_]{22,}")),
    ("an npm token", re.compile(rb"\bnpm_[A-Za-z0-9]{36}\b")),
    ("a GitLab token", re.compile(rb"\bglpat-[A-Za-z0-9_\-]{20,}")),
    ("a Slack token", re.compile(rb"\bxox[abposr]-[A-Za-z0-9-]{10,}")),
    ("an AWS access key id", re.compile(rb"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b")),
    ("a private key block", re.compile(rb"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY(?: BLOCK)?-----")),
    (
        "a Discord webhook URL",
        re.compile(rb"https://(?:(?:canary|ptb)\.)?discord(?:app)?\.com/api/webhooks/\d+/[A-Za-z0-9_\-]+"),
    ),
    (
        "a Slack webhook URL",
        re.compile(rb"https://hooks\.slack\.com/(?:services|workflows|triggers)/[A-Za-z0-9_/\-]{20,}"),
    ),
    (
        "a literal CLOUDFLARE_API_TOKEN value",
        re.compile(rb"CLOUDFLARE_API_TOKEN\s*[=:]\s*[\"']?[A-Za-z0-9_\-]{20,}"),
    ),
)


class GitError(RuntimeError):
    """A git command failed."""


@dataclass
class Repo:
    """A git repository and a cache of its tracked files."""

    root: Path
    _files: dict[str, bytes | None] = field(default_factory=dict)
    _tracked: list[str] | None = None

    def git(self, *args: str) -> bytes:
        proc = subprocess.run(
            ["git", "-c", "core.quotepath=off", "-C", str(self.root), *args],
            capture_output=True,
            check=False,
        )
        if proc.returncode != 0:
            message = proc.stderr.decode("utf-8", "replace").strip()
            raise GitError(f"git {' '.join(args)} failed: {message}")
        return proc.stdout

    def tracked(self) -> list[str]:
        """Tracked paths (index), '/'-separated, without .git/, node_modules/, target/."""
        if self._tracked is None:
            raw = self.git("ls-files", "-z")
            paths = [p.decode("utf-8", "surrogateescape") for p in raw.split(b"\0") if p]
            self._tracked = [p for p in paths if not SKIP_DIRS.intersection(p.split("/"))]
        return self._tracked

    def read(self, rel: str) -> bytes | None:
        """Working-tree bytes of a path, or None when it does not exist on disk."""
        if rel not in self._files:
            path = self.root / rel
            try:
                self._files[rel] = path.read_bytes() if path.is_file() else None
            except OSError:
                self._files[rel] = None
        return self._files[rel]

    def show(self, rev: str, rel: str) -> bytes | None:
        """Bytes of a path at a revision, or None when it does not exist there."""
        try:
            return self.git("show", f"{rev}:{rel}")
        except GitError:
            return None

    def commit(self, rev: str) -> str:
        try:
            return self.git("rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}").decode().strip()
        except GitError as err:
            raise GitError(
                f"{rev!r} is not a commit in this clone (CI needs actions/checkout with fetch-depth: 0)"
            ) from err


def as_text(data: bytes | None) -> str | None:
    """UTF-8 text, or None for binary (a NUL byte or invalid UTF-8)."""
    if data is None or b"\0" in data[:8192]:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


@dataclass
class Context:
    repo: Repo
    base: str | None = None
    head: str | None = None

    @property
    def range_mode(self) -> bool:
        return self.base is not None and self.head is not None


# ---------------------------------------------------------------------------- checks


def check_trailer(ctx: Context) -> list[str]:
    assert ctx.base is not None and ctx.head is not None
    fmt = f"%H%x1f%an%x1f%(trailers:key={TRAILER_KEY},valueonly,separator=%x1d)%x1f%s%x1e"
    raw = ctx.repo.git("log", f"--format={fmt}", f"{ctx.base}..{ctx.head}")
    problems = []
    for record in raw.decode("utf-8", "replace").split("\x1e"):
        record = record.strip("\n")
        if not record:
            continue
        sha, author, values, subject = (record.split("\x1f") + ["", "", ""])[:4]
        if author.endswith("[bot]"):
            continue
        found = {" ".join(v.split()) for v in values.split("\x1d")}
        if TRAILER_VALUE not in found:
            problems.append(
                f"commit {sha[:12]} ({subject.strip()!r} by {author}) lacks the trailer "
                f'"{TRAILER}" in the final paragraph of its message'
            )
    return problems


def _comments(lines: list[str], in_block: bool = False):
    """Yields the stripped comment lines (//, /* ... */ and their * continuations)."""
    for line in lines:
        s = line.strip()
        is_comment = s.startswith("//") or s.startswith("/*") or (in_block and s.startswith("*"))
        if s.startswith("/*") and "*/" not in s[2:]:
            in_block = True
        elif in_block and "*/" in s:
            is_comment = True
            in_block = False
        if is_comment:
            yield s


def _decompiled_header(text: str, path: str) -> str | None:
    """The decompiler banner of a JVM source file (header, or a JD-style footer), if any."""
    lines = text.splitlines()
    candidate = path.lower().endswith(JVM_SOURCE_EXTENSIONS) or bool(
        JAVA_PACKAGE.search("\n".join(lines[:60]))
    )
    if not candidate:
        return None
    header = _comments(lines[:HEADER_LINES])
    # JD-GUI writes its banner as a block comment at the end of the file.
    footer = _comments(lines[-FOOTER_LINES:], in_block=True) if len(lines) > HEADER_LINES else iter(())
    for comment in (*header, *footer):
        if DECOMPILER_MARK.search(comment):
            return comment
    return None


def check_artifacts(ctx: Context) -> list[str]:
    problems = []
    for path in ctx.repo.tracked():
        lower = path.lower()
        if lower.endswith(BANNED_EXTENSIONS):
            problems.append(f"{path}: JVM archives and class files are never committed")
            continue
        data = ctx.repo.read(path)
        if data is None:
            continue
        if data[:4] == b"PK\x03\x04" or lower.endswith(".zip"):
            try:
                with zipfile.ZipFile(io.BytesIO(data)) as archive:
                    inner = [n for n in archive.namelist() if n.lower().endswith(BANNED_EXTENSIONS)]
            except (zipfile.BadZipFile, OSError, ValueError):
                inner = []
            if inner:
                problems.append(f"{path}: archive contains {inner[0]} (classes or JARs are never committed)")
                continue
        text = as_text(data) if len(data) <= MAX_TEXT_BYTES else None
        if text is not None:
            header = _decompiled_header(text, path)
            if header is not None:
                problems.append(
                    f"{path}: looks decompiled (header comment {header[:80]!r}); "
                    "decompiled source is never committed"
                )
    return problems


def decision_ids(text: str) -> set[int]:
    """IDs of every '## D-<n>' heading, outside code fences."""
    ids = set()
    fenced = False
    for line in text.splitlines():
        if line.lstrip().startswith(("```", "~~~")):
            fenced = not fenced
            continue
        m = DECISION_ID.match(line)
        if m and not fenced:
            ids.add(int(m.group(1)))
    return ids


def check_research_lock(ctx: Context) -> list[str]:
    assert ctx.base is not None and ctx.head is not None
    repo = ctx.repo
    merge_base = repo.git("merge-base", ctx.base, ctx.head).decode().strip()
    raw = repo.git("diff", "--name-only", "--no-renames", "-z", merge_base, ctx.head)
    changed = {p.decode("utf-8", "surrogateescape") for p in raw.split(b"\0") if p}
    if RESEARCH_LOCK not in changed:
        return []
    if DECISIONS not in changed:
        return [f"{RESEARCH_LOCK} changed, but {DECISIONS} did not: add a D- entry explaining why"]
    before = decision_ids(as_text(repo.show(merge_base, DECISIONS)) or "")
    after = decision_ids(as_text(repo.show(ctx.head, DECISIONS)) or "")
    if not after - before:
        return [f"{RESEARCH_LOCK} changed, but {DECISIONS} gained no new '## D-<n>' entry"]
    return []


def _field_present(lines: list[str], label: str) -> bool:
    for line in lines:
        s = line.strip()
        if s[:2] in ("- ", "* "):
            s = s[2:].lstrip()
        if s.startswith(label):
            return True
    return False


def check_decisions(ctx: Context) -> list[str]:
    if DECISIONS not in ctx.repo.tracked():
        return [f"{DECISIONS} is not tracked (book section 13 requires the decision log)"]
    text = as_text(ctx.repo.read(DECISIONS))
    if text is None:
        return [f"{DECISIONS} is missing from the working tree or is not UTF-8"]
    problems = []
    entries: list[tuple[int, int, list[str]]] = []  # (line number, id, body lines)
    fenced = False
    for number, line in enumerate(text.splitlines(), start=1):
        if line.lstrip().startswith(("```", "~~~")):
            fenced = not fenced
        if not fenced and line.startswith("## "):
            m = DECISION_HEADING.match(line.rstrip())
            if m is None:
                problems.append(
                    f"{DECISIONS}:{number}: {line.strip()!r} does not read "
                    "'## D-<3 digits> — <title>' (the dash is an em dash, U+2014)"
                )
                entries.append((number, -1, []))
            else:
                entries.append((number, int(m.group(1)), []))
            continue
        if entries:
            entries[-1][2].append(line)
    expected = 1
    for number, ident, body in entries:
        if ident < 0:
            continue
        if ident != expected:
            problems.append(
                f"{DECISIONS}:{number}: D-{ident:03d} found where D-{expected:03d} was expected "
                "(IDs increase by one, starting at D-001)"
            )
        expected = max(expected, ident) + 1
        missing = [label for label in DECISION_FIELDS if not _field_present(body, label)]
        if missing:
            problems.append(f"{DECISIONS}:{number}: D-{ident:03d} lacks " + ", ".join(missing))
    return problems


def check_ledger(ctx: Context) -> list[str]:
    if LEDGER not in ctx.repo.tracked():
        return [f"{LEDGER} is not tracked; generate it with: python tools/ledger.py"]
    try:
        expected = ledger.render(ctx.repo.root)
    except ledger.LedgerInputError as err:
        return [f"ledger inputs are invalid: {err}"]
    actual = as_text(ctx.repo.read(LEDGER))
    if actual is None or actual.replace("\r\n", "\n") != expected:
        return [
            f"{LEDGER} differs from 'python tools/ledger.py --stdout'; it is never edited by "
            "hand. Regenerate it with: python tools/ledger.py"
        ]
    return []


def check_architecture(ctx: Context) -> list[str]:
    crates = sorted(
        {
            parts[1]
            for parts in (p.split("/") for p in ctx.repo.tracked())
            if len(parts) == 3 and parts[0] == "crates" and parts[2] == "Cargo.toml"
        }
    )
    if not crates:
        return []
    text = as_text(ctx.repo.read(ARCHITECTURE)) if ARCHITECTURE in ctx.repo.tracked() else None
    if text is None:
        return [f"{ARCHITECTURE} is missing, but crates exist: " + ", ".join(f"crates/{c}" for c in crates)]
    return [
        f"crates/{name} has a Cargo.toml but is not mentioned as 'crates/{name}' in {ARCHITECTURE}"
        for name in crates
        if not re.search(rf"crates/{re.escape(name)}(?![\w-])", text)
    ]


def check_secrets(ctx: Context) -> list[str]:
    problems = []
    for path in ctx.repo.tracked():
        data = ctx.repo.read(path)
        if data is None or len(data) > MAX_TEXT_BYTES or as_text(data) is None:
            continue
        for label, pattern in SECRET_PATTERNS:
            for match in pattern.finditer(data):
                line = data.count(b"\n", 0, match.start()) + 1
                problems.append(f"{path}:{line}: looks like {label} (value not shown)")
    return problems


@dataclass(frozen=True)
class Check:
    name: str
    run: Callable[[Context], list[str]]
    needs_range: bool = False


CHECKS: tuple[Check, ...] = (
    Check("trailer", check_trailer, needs_range=True),
    Check("artifacts", check_artifacts),
    Check("research-lock", check_research_lock, needs_range=True),
    Check("decisions", check_decisions),
    Check("ledger", check_ledger),
    Check("architecture", check_architecture),
    Check("secrets", check_secrets),
)


def _annotate(check: str, message: str) -> None:
    if os.environ.get("GITHUB_ACTIONS") == "true":
        escaped = message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error title=pr_ready {check}::{escaped}")


def run(ctx: Context, only: list[str] | None = None) -> dict[str, list[str] | None]:
    """Runs the checks; maps each name to its problems, or None when it was skipped."""
    results: dict[str, list[str] | None] = {}
    for check in CHECKS:
        if only and check.name not in only:
            continue
        if check.needs_range and not ctx.range_mode:
            results[check.name] = None
            continue
        results[check.name] = check.run(ctx)
    return results


def main(argv: list[str] | None = None) -> int:
    names = [c.name for c in CHECKS]
    parser = argparse.ArgumentParser(description="Pre-merge checks for Jarrow.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--all", action="store_true", help="check the tracked tree only (push mode)")
    mode.add_argument("--base", help="base commit of the range (PR mode, with --head)")
    parser.add_argument("--head", help="head commit of the range (PR mode, with --base)")
    parser.add_argument("--check", action="append", choices=names, help="run only this check")
    parser.add_argument("--repo", type=Path, help="repository (default: git top level of the cwd)")
    args = parser.parse_args(argv)
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(errors="backslashreplace")
    if args.base is not None and not args.head:
        parser.error("--base needs --head")
    if args.all and args.head:
        parser.error("--all takes no --head")

    try:
        if args.repo is None:
            top = Repo(Path.cwd()).git("rev-parse", "--show-toplevel").decode().strip()
            root = Path(top)
        else:
            root = args.repo
        repo = Repo(root.resolve())
        ctx = Context(repo)
        if not args.all:
            ctx = Context(repo, repo.commit(args.base), repo.commit(args.head))
        results = run(ctx, args.check)
    except GitError as err:
        print(f"pr_ready: {err}", file=sys.stderr)
        return 2

    mode_text = f"range {ctx.base[:12]}..{ctx.head[:12]}" if ctx.range_mode else "tree only"
    print(f"pr_ready ({mode_text})")
    failed = 0
    for name, problems in results.items():
        if problems is None:
            print(f"  skip  {name:<14} needs --base/--head")
        elif not problems:
            print(f"  ok    {name}")
        else:
            failed += 1
            print(f"  FAIL  {name}")
            for problem in problems:
                print(f"        - {problem}")
                _annotate(name, problem)
    print("pr_ready: all checks passed" if not failed else f"pr_ready: {failed} check(s) failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
