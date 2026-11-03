#!/usr/bin/env python3
"""Post the nightly board summary to GitHub Discussions.

Reads board.json (written by `jarrow-board merge`, contract in docs/board.md) and posts one
Discussion per direction in the category "Nightly results" (or "Announcements" when that
category does not exist), titled

    Nightly YYYY-MM-DD: G green, Y yellow, R red

With several directions on the board, each title names its direction:
"Nightly YYYY-MM-DD (fabric-neoforge): ...". The body holds the totals, the ten most common
reasons, links to the workflow run and to https://jarrow.pages.dev/board, and, while mods are
red only because no converter exists, a note that says so.

Idempotent: every body carries the marker
    <!-- jarrow-nightly date=YYYY-MM-DD direction=<direction> -->
and nothing is posted when a Discussion in the category already carries it.

Usage:
    python tools/board_summary.py --board out/board.json [--dry-run]
    options: --run-url URL   link to the workflow run (default: built from GITHUB_SERVER_URL,
                             GITHUB_REPOSITORY and GITHUB_RUN_ID; "not available" without them)
             --repo OWNER/NAME   default: $GITHUB_REPOSITORY, else MosheByte/jarrow
             --dry-run      print the titles and bodies; no network, no gh

Posting uses the gh CLI (`gh api graphql`), which reads its token from GH_TOKEN. The token is
never printed. Standard library only.

Exit status: 0 posted, already posted, or dry run; 1 on a GitHub error; 2 on bad input.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

BOARD_URL = "https://jarrow.pages.dev/board"
DEFAULT_REPO = "MosheByte/jarrow"
CATEGORY = "Nightly results"
FALLBACK_CATEGORY = "Announcements"
TOP_REASONS = 10
NO_CONVERTER = "no_converter"

_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}T")
_DIRECTION = re.compile(r"^(fabric|neoforge|forge|quilt)-(fabric|neoforge|forge|quilt)$")
_REPO = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")

LOADER_NAMES = {"fabric": "Fabric", "neoforge": "NeoForge", "forge": "Forge", "quilt": "Quilt"}


class BoardError(ValueError):
    """board.json lacks something the summary needs."""


class GitHubError(RuntimeError):
    """A gh call failed."""


@dataclass(frozen=True)
class Reason:
    kind: str
    label: str
    count: int


@dataclass(frozen=True)
class Column:
    """One direction of the board."""

    direction: str
    source: str
    target: str
    game_versions: list[str]
    mods: int
    green: int
    yellow: int
    red: int
    reasons: list[Reason]


@dataclass(frozen=True)
class Summary:
    date: str
    run_id: str
    commit: str
    generated_at: str
    files: int
    max_shard_minutes: float
    columns: list[Column]


@dataclass(frozen=True)
class Post:
    direction: str
    title: str
    body: str
    marker: str


# ------------------------------------------------------------------------------- reading


def _count(value: Any, where: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise BoardError(f"{where} must be a non-negative integer")
    return value


def _text(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise BoardError(f"{where} must be a non-empty string")
    return value


def _object(value: Any, where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise BoardError(f"{where} must be an object")
    return value


def _reasons(value: Any, where: str) -> list[Reason]:
    if not isinstance(value, list):
        raise BoardError(f"{where} must be a list")
    out = []
    for i, item in enumerate(value):
        item = _object(item, f"{where}[{i}]")
        out.append(
            Reason(
                kind=_text(item.get("kind"), f"{where}[{i}].kind"),
                label=_text(item.get("label"), f"{where}[{i}].label"),
                count=_count(item.get("count"), f"{where}[{i}].count"),
            )
        )
    return sorted(out, key=lambda r: (-r.count, r.kind))


def read_board(data: Any) -> Summary:
    """Validates the parts of board.json the summary uses."""
    board = _object(data, "board.json")
    if board.get("schema") != 1:
        raise BoardError("board.json schema must be 1")
    started = _text(board.get("started_at"), "started_at")
    if not _DATE.match(started):
        raise BoardError("started_at must be an RFC 3339 time")
    engine = _object(board.get("engine"), "engine")
    stats = _object(board.get("stats"), "stats")
    directions = board.get("directions")
    if not isinstance(directions, list) or not directions:
        raise BoardError("directions must be a non-empty list")
    columns = []
    for i, d in enumerate(directions):
        d = _object(d, f"directions[{i}]")
        direction = _text(d.get("direction"), f"directions[{i}].direction")
        match = _DIRECTION.match(direction)
        if not match or match.group(1) == match.group(2):
            raise BoardError(f"directions[{i}].direction {direction!r} is not <source>-<target>")
        totals = _object(d.get("totals"), f"directions[{i}].totals")
        column = Column(
            direction=direction,
            source=match.group(1),
            target=match.group(2),
            game_versions=[_text(v, f"directions[{i}].game_versions") for v in d.get("game_versions") or []],
            mods=_count(totals.get("mods"), f"directions[{i}].totals.mods"),
            green=_count(totals.get("green"), f"directions[{i}].totals.green"),
            yellow=_count(totals.get("yellow"), f"directions[{i}].totals.yellow"),
            red=_count(totals.get("red"), f"directions[{i}].totals.red"),
            reasons=_reasons(d.get("top_reasons"), f"directions[{i}].top_reasons"),
        )
        if column.green + column.yellow + column.red != column.mods:
            raise BoardError(f"{direction}: green + yellow + red must equal mods")
        columns.append(column)
    if len({c.direction for c in columns}) != len(columns):
        raise BoardError("a direction appears twice")
    columns.sort(key=lambda c: c.direction)
    return Summary(
        date=started[:10],
        run_id=_text(board.get("run_id"), "run_id"),
        commit=_text(engine.get("git_sha"), "engine.git_sha"),
        generated_at=_text(board.get("generated_at"), "generated_at"),
        files=_count(stats.get("files"), "stats.files"),
        max_shard_minutes=_count(stats.get("max_shard_wall_ms"), "stats.max_shard_wall_ms") / 60000,
        columns=columns,
    )


# ------------------------------------------------------------------------------ rendering


def marker(date: str, direction: str) -> str:
    return f"<!-- jarrow-nightly date={date} direction={direction} -->"


def title(summary: Summary, column: Column) -> str:
    counts = f"{column.green} green, {column.yellow} yellow, {column.red} red"
    if len(summary.columns) == 1:
        return f"Nightly {summary.date}: {counts}"
    return f"Nightly {summary.date} ({column.direction}): {counts}"


def _md(text: str) -> str:
    """Text safe in a Markdown table cell."""
    return text.replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def _pct(part: int, whole: int) -> str:
    return "n/a" if whole == 0 else f"{part * 100 / whole:.1f}%"


def body(summary: Summary, column: Column, run_url: str | None, repo: str) -> str:
    label = f"{LOADER_NAMES[column.source]} to {LOADER_NAMES[column.target]}"
    versions = ", ".join(column.game_versions) or "none"
    lines = [marker(summary.date, column.direction), ""]
    no_converter = sum(r.count for r in column.reasons if r.kind == NO_CONVERTER)
    if no_converter:
        share = (
            "every mod on this board is red for that reason"
            if no_converter == column.mods
            else f"{no_converter:,} of {column.mods:,} mods are red for that reason"
        )
        lines += [
            f"**No converter exists yet** (milestone M2), so {share}. These numbers measure the "
            "board pipeline (listing, downloading, checking and reporting), not conversions. "
            "Nothing is green until the parity harness measures it.",
            "",
        ]
    lines += [
        f"**{label}** on Minecraft {versions}, run `{summary.run_id}` "
        f"(commit `{summary.commit[:12]}`).",
        "",
        "| Outcome | Mods | Share |",
        "| --- | --- | --- |",
        f"| Green | {column.green:,} | {_pct(column.green, column.mods)} |",
        f"| Yellow | {column.yellow:,} | {_pct(column.yellow, column.mods)} |",
        f"| Red | {column.red:,} | {_pct(column.red, column.mods)} |",
        f"| Total | {column.mods:,} | |",
        "",
    ]
    shown = column.reasons[:TOP_REASONS]
    lines.append(f"### Top reasons ({len(shown)} of {len(column.reasons)})" if column.reasons else "### Top reasons")
    lines.append("")
    if shown:
        lines += ["| Reason | Kind | Mods |", "| --- | --- | --- |"]
        lines += [f"| {_md(r.label)} | `{r.kind}` | {r.count:,} |" for r in shown]
    else:
        lines.append("No yellow or red mods.")
    lines += [
        "",
        "### Links",
        "",
        f"- Board: {BOARD_URL}",
        f"- Workflow run: {run_url}" if run_url else "- Workflow run: not available (local run)",
        f"- Data: `board.json`, `board-summary.json` and `reports.tar.gz` in the "
        f"[board-latest release](https://github.com/{repo}/releases/tag/board-latest)",
        "",
        f"{summary.files:,} files checked; the slowest shard took {summary.max_shard_minutes:.1f} "
        f"minutes. Board generated at {summary.generated_at}.",
        "",
    ]
    return "\n".join(lines)


def posts(summary: Summary, run_url: str | None, repo: str) -> list[Post]:
    return [
        Post(
            direction=c.direction,
            title=title(summary, c),
            body=body(summary, c, run_url, repo),
            marker=marker(summary.date, c.direction),
        )
        for c in summary.columns
    ]


# --------------------------------------------------------------------------------- GitHub

Runner = Callable[[list[str], str], str]


def run_gh(args: list[str], stdin: str) -> str:
    """Runs gh with `stdin`; returns stdout. The token stays in gh's environment."""
    try:
        proc = subprocess.run(
            ["gh", *args],
            input=stdin.encode("utf-8"),
            capture_output=True,
            check=False,
            timeout=120,
        )
    except (OSError, subprocess.TimeoutExpired) as err:
        raise GitHubError(f"gh could not run: {err}") from err
    if proc.returncode != 0:
        message = proc.stderr.decode("utf-8", "replace").strip().splitlines()
        raise GitHubError(f"gh {' '.join(args[:2])} failed: {message[-1] if message else proc.returncode}")
    return proc.stdout.decode("utf-8", "replace")


class GitHub:
    """The three GraphQL calls the summary needs, through gh."""

    def __init__(self, repo: str, runner: Runner = run_gh) -> None:
        if not _REPO.match(repo):
            raise GitHubError(f"{repo!r} is not OWNER/NAME")
        self.owner, self.name = repo.split("/", 1)
        self.runner = runner

    def graphql(self, query: str, variables: dict[str, Any]) -> dict[str, Any]:
        out = self.runner(["api", "graphql", "--input", "-"], json.dumps({"query": query, "variables": variables}))
        try:
            data = json.loads(out)
        except json.JSONDecodeError as err:
            raise GitHubError("gh returned something that is not JSON") from err
        if not isinstance(data, dict) or data.get("errors"):
            errors = data.get("errors") if isinstance(data, dict) else None
            raise GitHubError(f"GraphQL errors: {json.dumps(errors)[:300]}")
        return data.get("data") or {}

    def repository(self) -> tuple[str, dict[str, str]]:
        data = self.graphql(
            "query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) "
            "{ id discussionCategories(first: 100) { nodes { id name } } } }",
            {"owner": self.owner, "name": self.name},
        )
        repo = data.get("repository") or {}
        nodes = (repo.get("discussionCategories") or {}).get("nodes") or []
        return repo.get("id", ""), {n["name"]: n["id"] for n in nodes if n and "name" in n and "id" in n}

    def recent(self, category_id: str) -> list[dict[str, Any]]:
        data = self.graphql(
            "query($owner: String!, $name: String!, $category: ID!) { repository(owner: $owner, "
            "name: $name) { discussions(first: 100, categoryId: $category, orderBy: {field: "
            "CREATED_AT, direction: DESC}) { nodes { title url body } } } }",
            {"owner": self.owner, "name": self.name, "category": category_id},
        )
        nodes = ((data.get("repository") or {}).get("discussions") or {}).get("nodes") or []
        return [n for n in nodes if isinstance(n, dict)]

    def create(self, repository_id: str, category_id: str, post: Post) -> str:
        data = self.graphql(
            "mutation($repo: ID!, $category: ID!, $title: String!, $body: String!) { "
            "createDiscussion(input: {repositoryId: $repo, categoryId: $category, title: $title, "
            "body: $body}) { discussion { url } } }",
            {"repo": repository_id, "category": category_id, "title": post.title, "body": post.body},
        )
        return ((data.get("createDiscussion") or {}).get("discussion") or {}).get("url", "")


def pick_category(categories: dict[str, str]) -> tuple[str, str]:
    for name in (CATEGORY, FALLBACK_CATEGORY):
        if name in categories:
            return name, categories[name]
    raise GitHubError(f"neither {CATEGORY!r} nor {FALLBACK_CATEGORY!r} exists as a Discussion category")


def publish(github: GitHub, items: list[Post], log: Callable[[str], None]) -> list[str]:
    """Posts every item that is not already in the category. Returns what happened per item."""
    repository_id, categories = github.repository()
    if not repository_id:
        raise GitHubError("the repository was not found")
    name, category_id = pick_category(categories)
    existing = github.recent(category_id)
    results = []
    for post in items:
        found = next((d for d in existing if post.marker in (d.get("body") or "")), None)
        if found is not None:
            results.append(f"already posted: {post.title} ({found.get('url', '')})")
        else:
            url = github.create(repository_id, category_id, post)
            results.append(f"posted in {name}: {post.title} ({url})")
        log(results[-1])
    return results


# ------------------------------------------------------------------------------------ CLI


def default_run_url(env: dict[str, str]) -> str | None:
    server, repo, run = env.get("GITHUB_SERVER_URL"), env.get("GITHUB_REPOSITORY"), env.get("GITHUB_RUN_ID")
    if server and repo and run:
        return f"{server.rstrip('/')}/{repo}/actions/runs/{run}"
    return None


def main(argv: list[str] | None = None, runner: Runner = run_gh, env: dict[str, str] | None = None) -> int:
    env = dict(os.environ) if env is None else env
    parser = argparse.ArgumentParser(description="Post the nightly board summary to GitHub Discussions.")
    parser.add_argument("--board", type=Path, required=True, help="board.json from jarrow-board merge")
    parser.add_argument("--run-url", help="link to the workflow run")
    parser.add_argument("--repo", default=env.get("GITHUB_REPOSITORY") or DEFAULT_REPO, help="OWNER/NAME")
    parser.add_argument("--dry-run", action="store_true", help="print instead of posting")
    args = parser.parse_args(argv)
    try:
        summary = read_board(json.loads(args.board.read_bytes().decode("utf-8")))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, BoardError) as err:
        print(f"board_summary: {args.board}: {err}", file=sys.stderr)
        return 2
    run_url = args.run_url or default_run_url(env)
    items = posts(summary, run_url, args.repo)
    if args.dry_run:
        for post in items:
            sys.stdout.write(f"# {post.title}\n\n{post.body}\n")
        return 0
    try:
        publish(GitHub(args.repo, runner), items, print)
    except GitHubError as err:
        print(f"board_summary: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
