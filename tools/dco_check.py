#!/usr/bin/env python3
"""Developer Certificate of Origin check for the commits of a pull request (CONTRIBUTING.md, "DCO").

Usage:
    python tools/dco_check.py --base <sha> --head <sha> [--cutoff <ISO 8601 time>] [--repo DIR]

Every commit in base..head must carry a "Signed-off-by: Name <email>" trailer whose email is
the commit's author email (compared ignoring case), as `git commit -s` writes it. Exempt:

    - merge commits (they add no authored change of their own; the commits they merge are checked);
    - commits by bots: an author name ending in "[bot]" (Dependabot, Renovate, github-actions)
      or "jarrow-bot", the project's deploy-key bot (tools/bot_pr.sh);
    - commits whose committer date is before the cutoff, 2026-11-03T18:30:00Z by default. That is
      when the DCO requirement started (the owner's consent to sign off the commits made through
      the owner's account); older commits predate it and are never rewritten.

The committer date is commit metadata that the committer sets, so the cutoff is not a security
boundary: a backdated commit in a pull request is a review finding (docs/code-review.md).

Prints one line per commit and, with GITHUB_STEP_SUMMARY set, appends a Markdown table to it.
Exit status: 0 when every commit passes or is exempt, 1 when one lacks its sign-off, 2 on a
usage or git error. Standard library only.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_CUTOFF = "2026-11-03T18:30:00Z"
BOT_NAMES = frozenset({"jarrow-bot"})
SIGN_OFF = re.compile(r"^(?P<name>.*?)\s*<(?P<email>[^<>]+)>\s*$")


class GitError(RuntimeError):
    """A git command failed."""


@dataclass(frozen=True)
class Commit:
    sha: str
    author_name: str
    author_email: str
    committed: datetime
    parents: int
    subject: str
    sign_offs: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class Verdict:
    commit: Commit
    status: str  # "signed", "missing", "exempt"
    reason: str


def git(root: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-c", "core.quotepath=off", "-C", str(root), *args], capture_output=True, check=False
    )
    if proc.returncode != 0:
        raise GitError(f"git {' '.join(args)} failed: {proc.stderr.decode('utf-8', 'replace').strip()}")
    return proc.stdout.decode("utf-8", "replace")


def parse_time(text: str) -> datetime:
    value = datetime.fromisoformat(text.strip().replace("Z", "+00:00"))
    if value.tzinfo is None:
        raise ValueError(f"{text!r} has no time zone")
    return value.astimezone(timezone.utc)


def commits(root: Path, base: str, head: str) -> list[Commit]:
    """The commits in base..head, oldest first."""
    fmt = "%H%x1f%an%x1f%ae%x1f%cI%x1f%P%x1f%s%x1f%(trailers:key=Signed-off-by,valueonly,unfold,separator=%x1d)%x1e"
    raw = git(root, "log", "--reverse", f"--format={fmt}", f"{base}..{head}", "--")
    out = []
    for record in raw.split("\x1e"):
        record = record.strip("\n")
        if not record:
            continue
        sha, name, email, date, parents, subject, trailers = (record.split("\x1f") + [""] * 7)[:7]
        sign_offs = []
        for value in trailers.split("\x1d"):
            m = SIGN_OFF.match(" ".join(value.split()))
            if m:
                sign_offs.append((m.group("name"), m.group("email")))
        out.append(
            Commit(sha, name, email, parse_time(date), len(parents.split()), subject, tuple(sign_offs))
        )
    return out


def judge(commit: Commit, cutoff: datetime) -> Verdict:
    name = commit.author_name.strip()
    if commit.parents > 1:
        return Verdict(commit, "exempt", "merge commit")
    if name.lower().endswith("[bot]") or name.lower() in BOT_NAMES:
        return Verdict(commit, "exempt", f"bot author ({name})")
    if commit.committed < cutoff:
        return Verdict(commit, "exempt", f"committed before {cutoff:%Y-%m-%dT%H:%M:%SZ}")
    author = commit.author_email.strip().lower()
    if any(email.strip().lower() == author for _, email in commit.sign_offs):
        return Verdict(commit, "signed", f"Signed-off-by {commit.author_email}")
    if commit.sign_offs:
        others = ", ".join(email for _, email in commit.sign_offs)
        return Verdict(commit, "missing", f"signed off by {others}, not by the author {commit.author_email}")
    return Verdict(commit, "missing", f"no Signed-off-by trailer for the author {commit.author_email}")


def cell(text: str) -> str:
    return text.replace("|", "\\|").replace("`", "'").replace("\r", " ").replace("\n", " ")


def summary(verdicts: list[Verdict], cutoff: datetime) -> str:
    missing = [v for v in verdicts if v.status == "missing"]
    lines = [
        "### DCO sign-off",
        "",
        f"{len(verdicts)} commit(s); requirement in force for commits made from {cutoff:%Y-%m-%dT%H:%M:%SZ}. "
        + ("Every commit passes." if not missing else f"{len(missing)} commit(s) lack a sign-off."),
        "",
        "| Commit | Author | Result | Why |",
        "| --- | --- | --- | --- |",
    ]
    for v in verdicts:
        lines.append(
            f"| `{v.commit.sha[:12]}` {cell(v.commit.subject[:72])} | {cell(v.commit.author_name)} | {v.status} | {cell(v.reason)} |"
        )
    if missing:
        lines += [
            "",
            "Sign off each listed commit (`git commit --amend -s` for the last one, or "
            "`git rebase --signoff <base>` for several) and push again. See CONTRIBUTING.md, \"DCO\".",
        ]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="DCO sign-off check for base..head.")
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--cutoff", default=DEFAULT_CUTOFF, help=f"ISO 8601 time (default {DEFAULT_CUTOFF})")
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    try:
        cutoff = parse_time(args.cutoff)
        verdicts = [judge(c, cutoff) for c in commits(args.repo, args.base, args.head)]
    except (GitError, ValueError) as err:
        print(f"dco_check: {err}", file=sys.stderr)
        return 2
    for v in verdicts:
        print(f"{v.status:<8} {v.commit.sha[:12]} {v.commit.author_name}: {v.reason}")
    text = summary(verdicts, cutoff)
    step_summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if step_summary:
        with open(step_summary, "a", encoding="utf-8") as handle:
            handle.write(text)
    missing = [v for v in verdicts if v.status == "missing"]
    if missing:
        if os.environ.get("GITHUB_ACTIONS") == "true":
            for v in missing:
                print(f"::error title=DCO::commit {v.commit.sha[:12]}: {cell(v.reason)}")
        print(f"dco_check: {len(missing)} commit(s) lack a Signed-off-by for their author")
        return 1
    print(f"dco_check: {len(verdicts)} commit(s) checked, none lacks a sign-off")
    return 0


if __name__ == "__main__":
    sys.exit(main())
