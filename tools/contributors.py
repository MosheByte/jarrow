#!/usr/bin/env python3
"""Generate "Who works on Jarrow": the people, agents and bots that work on the repository.

The list is never edited by hand. This script writes four outputs from one set of data:

    .all-contributorsrc       the all-contributors spec (https://allcontributors.org) with the
                              custom contribution types "agent", "bot" and "ci". It is also the
                              cache that --offline runs and failed GitHub lookups fall back on.
    README.md                 the all-contributors table between the ALL-CONTRIBUTORS-LIST
                              markers, one sub-heading per group, the role line under each
                              name. Only the text between the markers changes.
    docs/team.md              the same three groups as a page, with roles and commit counts
    site/src/data/team.json   the same data for the site's /about page

Usage:
    python tools/contributors.py              write every output
    python tools/contributors.py --check      write nothing; exit 1 when an output differs
                                              from what would be written
    python tools/contributors.py --offline    git and repository files only, never gh;
                                              logins and avatars come from .all-contributorsrc
    python tools/contributors.py --ledger     write only ledger/inputs/contributors.json, the
                                              contributor numbers tools/ledger.py renders
    options: --no-readme (leave README.md alone), --root DIR (repository, default: the git
             top level of the current directory), --repo OWNER/NAME (default: the github.com
             "origin" remote, else MosheByte/jarrow), --rev REV (default HEAD),
             --as-of YYYY-MM-DD (last day of the --ledger window, default today in UTC)

--ledger: the last 30 days (the as-of day and the 29 before it, UTC), from merged pull request
authors (gh pr list --state merged) and from the authors and co-authors of commits on --rev
whose committer date falls in the window (git). People and agents are counted separately and
bots are not counted. "first_time_contributors_30d" lists the people whose first merged pull
request in the repository falls in the window; it is null when the pull request lookup failed
or --offline was given. The file keeps its old "as_of" when nothing else would change, so the
nightly opens a pull request only when a number or a login changes:

    {
      "schema": 1,
      "as_of": "2026-11-03",
      "window_days": 30,
      "sources": ["git", "github-pulls"],      "git" alone when the PR lookup was unavailable
      "contributors_30d": {
        "people": {"count": 1, "logins": ["MosheByte"]},
        "agents": {"count": 1, "names": ["Claude Opus 5.5"]}
      },
      "first_time_contributors_30d": {"count": 0, "logins": []}
    }

Where the data comes from:
    People   every author, committer and human co-author of a commit on --rev (git log, with
             .mailmap applied), except bots, agents and GitHub's web-flow committer, plus the
             project owner. Members are shown by GitHub login, never by the name in a commit.
             A login comes from a GitHub no-reply address, from the commits API (gh api
             repos/<repo>/commits?sha=<sha>), or, offline, from a login already in
             .all-contributorsrc that equals the commit name. An identity without a login is
             left out, and the warning names only a commit SHA.
    Agents   every distinct agent named by a Co-Authored-By trailer (or as author): a model
             name (Claude Opus, Claude Sonnet, GPT, Codex, Gemini, Copilot, ...) or an agent
             no-reply address. Count: commits that name it. Agents have no GitHub account of
             their own here; they commit through the owner's account.
    Bots     what acts on the repository: GitHub App installations when the token can list
             them (gh api repos/<repo>/installations, then user/installations; both need a
             GitHub App token and normally fail, which is logged), the apps behind the check
             runs and non-empty check suites of the newest counted commit (the main tip when
             that commit is not on GitHub; GitHub creates an empty suite for every installed
             app on each push, so an empty one is not counted), bot pull-request authors,
             commit authors ending in "[bot]",
             the jarrow-bot deploy key (gh api repos/<repo>/keys), and the automation defined in
             .github/: workflows (github-actions), dependabot.yml (dependabot), CodeQL, and
             Renovate, release-please or Socket only when their config file or app exists.

Determinism: no clock (except the default --as-of day of --ledger) and no randomness, sorted
output, LF line endings. "generated_at" is
the committer date (UTC) of the newest counted commit, so reruns on the same history and the
same GitHub state give identical bytes. Commits whose message carries the trailer
"Jarrow-Generated: contributors" (the refresh commits .github/workflows/contributors.yml
makes) are not counted at all, so a refresh never triggers another one.

When a GitHub lookup fails (no gh, no network, or a token without the scope: the nightly's
token cannot list deploy keys), members that only that lookup could confirm are kept as
.all-contributorsrc lists them instead of being dropped.

Nothing this script prints contains an e-mail address or the name from a commit.

Exit status: 0 written or up to date, 1 --check found a stale output, 2 usage or input
error (for example README.md without the markers). Standard library only.
"""
from __future__ import annotations

import argparse
import html
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Callable, Iterable, Protocol

REPO_SLUG = "MosheByte/jarrow"
OWNER_LOGIN = "MosheByte"
PROJECT_NAME = "jarrow"
DEFAULT_BRANCH = "main"

RC_PATH = ".all-contributorsrc"
README_PATH = "README.md"
TEAM_MD_PATH = "docs/team.md"
TEAM_JSON_PATH = "site/src/data/team.json"
KEY_ICON_PATH = "docs/assets/key.svg"
OUTPUT_PATHS = (RC_PATH, README_PATH, TEAM_MD_PATH, TEAM_JSON_PATH)
LEDGER_INPUT_PATH = "ledger/inputs/contributors.json"
LEDGER_SCHEMA = 1
LEDGER_WINDOW_DAYS = 30
MERGED_PR_LIMIT = 10000

START_MARKER = "<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->"
END_MARKER = "<!-- ALL-CONTRIBUTORS-LIST:END -->"

REFRESH_TRAILER_KEY = "Jarrow-Generated"
REFRESH_TRAILER_VALUE = "contributors"

IMAGE_SIZE = 80
PER_LINE = 7
TEAM_MD_IMAGE_SIZE = 40

OWNER_ROLE = (
    "Owner: sets direction, holds the accounts. Commits through this account are written "
    "by the agent and disclosed in their trailers."
)
OPUS_NAME = "Claude Opus 5.5"
OPUS_ROLE = (
    "Engineering: writes the code, docs and infrastructure; co-author of every commit it touches"
)
AGENT_ROLE = "Agent: co-author of the commits that name it in a Co-Authored-By trailer."
PERSON_ROLE = "Contributor: commits in this repository."
GENERIC_BOT_ROLE = "GitHub App or bot account that acts on this repository."

GROUPS = (
    (
        "people",
        "People",
        "Every person who authored or committed a commit on main, shown by GitHub login, "
        "plus the project owner.",
    ),
    (
        "agents",
        "Agents",
        "Agents have no GitHub account of their own here. They act through the owner's "
        "account, and every commit they write names them in a Co-Authored-By trailer; the "
        "counts come from those trailers.",
    ),
    (
        "bots",
        "Bots and automation",
        "Every GitHub App, bot and workflow that acts on the repository, and what it does.",
    ),
)


# ---------------------------------------------------------------- contribution types


@dataclass(frozen=True)
class ContributionType:
    symbol: str
    description: str


# Display order. "agent", "bot" and "ci" are custom types; the rest are all-contributors'.
TYPES: dict[str, ContributionType] = {
    "agent": ContributionType("\U0001f916", "AI agent"),
    "bot": ContributionType("\u2699\ufe0f", "Bot or GitHub App"),
    "ci": ContributionType("\U0001f501", "CI and scheduled workflows"),
    "code": ContributionType("\U0001f4bb", "Code"),
    "doc": ContributionType("\U0001f4d6", "Documentation"),
    "test": ContributionType("\u26a0\ufe0f", "Tests"),
    "infra": ContributionType("\U0001f687", "Infrastructure (Hosting, Build-Tools, etc)"),
    "security": ContributionType("\U0001f6e1\ufe0f", "Security"),
    "maintenance": ContributionType("\U0001f6a7", "Maintenance"),
    "ideas": ContributionType("\U0001f914", "Ideas, Planning, & Feedback"),
    "projectManagement": ContributionType("\U0001f4c6", "Project Management"),
}
CUSTOM_TYPES = ("agent", "bot", "ci")
COMMIT_LINK_TYPES = frozenset({"code", "doc", "test"})
TYPE_ORDER = {name: index for index, name in enumerate(TYPES)}

CODE_EXTENSIONS = (
    ".rs", ".py", ".ts", ".tsx", ".js", ".mjs", ".cjs", ".astro", ".css", ".html",
    ".toml", ".json", ".jsonc", ".java", ".kt", ".sh", ".ps1",
)


def ordered_types(types: set[str] | list[str] | tuple[str, ...]) -> list[str]:
    return sorted(set(types), key=lambda t: (TYPE_ORDER.get(t, len(TYPE_ORDER)), t))


def path_types(path: str) -> set[str]:
    """Contribution types a changed path stands for: test, infra, doc or code."""
    p = path.lower()
    name = p.rsplit("/", 1)[-1]
    if "/tests/" in f"/{p}" or name.startswith("test_") or ".test." in name:
        return {"test"}
    if p.startswith((".github/", "infra/")) or name in {"wrangler.toml", "wrangler.jsonc"}:
        return {"infra"}
    if p.startswith("docs/") or p.endswith((".md", ".mdx")):
        return {"doc"}
    if p.endswith(CODE_EXTENSIONS):
        return {"code"}
    return set()


# ---------------------------------------------------------------- errors and logging


class InputError(RuntimeError):
    """Bad input: a git failure, a missing marker, an unreadable file."""


class GhError(RuntimeError):
    """A gh call failed."""


Log = Callable[[str], None]


def stderr_log(message: str) -> None:
    print(f"contributors: {message}", file=sys.stderr)


# ---------------------------------------------------------------- git history


@dataclass
class Commit:
    sha: str
    author: tuple[str, str]
    committer: tuple[str, str]
    date: str
    coauthors: list[tuple[str, str]]
    files: list[str] = field(default_factory=list)


TRAILER_PERSON = re.compile(r"^(.*?)\s*<([^<>]*)>\s*$")


def git(root: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-c", "core.quotepath=off", "-C", str(root), *args],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        message = proc.stderr.decode("utf-8", "replace").strip()
        raise InputError(f"git {' '.join(args)} failed: {message}")
    return proc.stdout.decode("utf-8", "replace")


def parse_person(value: str) -> tuple[str, str]:
    value = " ".join(value.split())
    m = TRAILER_PERSON.match(value)
    if m:
        return m.group(1).strip(), m.group(2).strip()
    return value, ""


def read_history(root: Path, rev: str) -> list[Commit]:
    """Commits reachable from rev, newest first, without the refresh commits."""
    fmt = (
        "%H%x1f%aN%x1f%aE%x1f%cN%x1f%cE%x1f%cI%x1f"
        "%(trailers:key=Co-Authored-By,valueonly,unfold,separator=%x1d)%x1f"
        f"%(trailers:key={REFRESH_TRAILER_KEY},valueonly,unfold,separator=%x1d)%x1e"
    )
    raw = git(root, "log", f"--format={fmt}", rev, "--")
    commits: list[Commit] = []
    for record in raw.split("\x1e"):
        record = record.strip("\n")
        if not record:
            continue
        parts = (record.split("\x1f") + [""] * 8)[:8]
        sha, an, ae, cn, ce, date, coauthors, refresh = parts
        if REFRESH_TRAILER_VALUE in {v.strip().lower() for v in refresh.split("\x1d")}:
            continue
        people = [parse_person(v) for v in coauthors.split("\x1d") if v.strip()]
        commits.append(Commit(sha.strip(), (an, ae), (cn, ce), date.strip(), people))
    if not commits:
        raise InputError(f"no commits to count on {rev}")
    files_raw = git(root, "log", "--format=%x1e%H", "--name-only", "--no-renames", rev, "--")
    files: dict[str, list[str]] = {}
    for chunk in files_raw.split("\x1e"):
        lines = [line for line in chunk.splitlines() if line.strip()]
        if lines:
            files[lines[0].strip()] = lines[1:]
    for commit in commits:
        commit.files = files.get(commit.sha, [])
    return commits


def generated_at(commits: list[Commit]) -> str:
    newest = max(datetime.fromisoformat(c.date) for c in commits)
    return newest.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# ---------------------------------------------------------------- identities

NOREPLY = re.compile(
    r"^(?:(\d+)\+)?([A-Za-z0-9](?:[A-Za-z0-9]|-(?=[A-Za-z0-9])){0,38})@users\.noreply\.github\.com$",
    re.IGNORECASE,
)
AGENT_EMAILS = frozenset({"noreply@anthropic.com", "noreply@openai.com"})
AGENT_NAME = re.compile(
    r"\b(?:claude[\s-]+(?:opus|sonnet|haiku|code)|chatgpt|gpt-?\d|codex|gemini|copilot)\b",
    re.IGNORECASE,
)
DEPLOY_KEY_LOGIN = "jarrow-bot"


def identity_kind(name: str, email: str) -> str:
    """'bot', 'agent', 'web-flow' (GitHub's merge committer) or 'human'."""
    n, e = name.strip().lower(), email.strip().lower()
    if n.endswith("[bot]") or e.endswith("[bot]@users.noreply.github.com"):
        return "bot"
    if n == DEPLOY_KEY_LOGIN:
        return "bot"
    if n == "github" and e == "noreply@github.com":
        return "web-flow"
    if e in AGENT_EMAILS or AGENT_NAME.search(name):
        return "agent"
    return "human"


def bot_login(name: str, email: str) -> str:
    n = name.strip()
    if n.lower().endswith("[bot]"):
        return n[:-5]
    m = re.match(r"^(?:\d+\+)?(.+)\[bot\]@users\.noreply\.github\.com$", email.strip(), re.I)
    if m:
        return m.group(1)
    return n


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


# ---------------------------------------------------------------- GitHub access


class GhClient(Protocol):
    def api(self, path: str) -> object: ...

    def pr_authors(self, repo: str) -> list[dict]: ...

    def merged_pulls(self, repo: str) -> list[dict]: ...


class GhCli:
    """The gh CLI. Errors become GhError with gh's own message (never a token)."""

    def __init__(self, exe: str) -> None:
        self.exe = exe

    def _run(self, args: list[str]) -> str:
        env = {
            **os.environ,
            "GH_PROMPT_DISABLED": "1",
            "GH_NO_UPDATE_NOTIFIER": "1",
            "GH_PAGER": "cat",
            "NO_COLOR": "1",
        }
        try:
            proc = subprocess.run(
                [self.exe, *args], capture_output=True, check=False, env=env, timeout=90
            )
        except (OSError, subprocess.TimeoutExpired) as err:
            raise GhError(str(err)) from err
        if proc.returncode != 0:
            lines = [ln.strip() for ln in proc.stderr.decode("utf-8", "replace").splitlines()]
            lines = [ln for ln in lines if ln]
            raise GhError(lines[-1] if lines else f"gh exited with {proc.returncode}")
        return proc.stdout.decode("utf-8", "replace")

    def api(self, path: str) -> object:
        out = self._run(["api", "-H", "Accept: application/vnd.github+json", path])
        return json.loads(out) if out.strip() else None

    def pr_authors(self, repo: str) -> list[dict]:
        out = self._run(
            ["pr", "list", "--repo", repo, "--state", "all", "--limit", "1000", "--json", "author"]
        )
        return [item.get("author") or {} for item in (json.loads(out) if out.strip() else [])]

    def merged_pulls(self, repo: str) -> list[dict]:
        """Every merged pull request as {"number", "author": {"login", "is_bot"}, "mergedAt"}."""
        out = self._run(
            [
                "pr", "list", "--repo", repo, "--state", "merged", "--limit", str(MERGED_PR_LIMIT),
                "--json", "number,author,mergedAt",
            ]
        )
        return list(json.loads(out)) if out.strip() else []


# ---------------------------------------------------------------- repository files


@dataclass
class RepoFiles:
    workflows: dict[str, str]  # file name -> text, sorted by name
    dependabot: str | None
    renovate: list[str]
    socket: list[str]
    release_please_files: list[str]


RENOVATE_FILES = (
    "renovate.json", "renovate.json5", ".github/renovate.json", ".github/renovate.json5",
    ".gitlab/renovate.json", ".gitlab/renovate.json5", ".renovaterc", ".renovaterc.json",
    ".renovaterc.json5",
)
SOCKET_FILES = ("socket.yml", "socket.yaml", ".github/socket.yml", ".github/socket.yaml")
RELEASE_PLEASE_FILES = ("release-please-config.json", ".release-please-manifest.json")


def read_text(path: Path) -> str | None:
    try:
        return path.read_bytes().decode("utf-8", "replace").replace("\r\n", "\n")
    except OSError:
        return None


def read_repo_files(root: Path) -> RepoFiles:
    wf_dir = root / ".github" / "workflows"
    workflows: dict[str, str] = {}
    if wf_dir.is_dir():
        for path in sorted(wf_dir.iterdir(), key=lambda p: p.name):
            if path.is_file() and path.suffix in (".yml", ".yaml"):
                workflows[path.name] = read_text(path) or ""
    dependabot = None
    for name in ("dependabot.yml", "dependabot.yaml"):
        text = read_text(root / ".github" / name)
        if text is not None:
            dependabot = text
            break
    return RepoFiles(
        workflows=workflows,
        dependabot=dependabot,
        renovate=[p for p in RENOVATE_FILES if (root / p).is_file()],
        socket=[p for p in SOCKET_FILES if (root / p).is_file()],
        release_please_files=[p for p in RELEASE_PLEASE_FILES if (root / p).is_file()],
    )


def and_join(items: list[str]) -> str:
    if len(items) <= 1:
        return "".join(items)
    return ", ".join(items[:-1]) + " and " + items[-1]


WORKFLOW_PURPOSES = {
    "board.yml": "nightly board",
    "ci.yml": "CI",
    "claim-names.yml": "registry name claims",
    "codeql.yml": "CodeQL",
    "contributors.yml": "this list",
    "deploy-site.yml": "site deploy",
    "infra.yml": "infra apply",
    "labeler.yml": "PR labels",
    "mirror.yml": "GitLab mirror",
    "pages.yml": "API docs",
    "proposal-to-issue.yml": "rule proposal issues",
    "red-mod-intake.yml": "red mod intake",
    "release.yml": "releases",
    "scorecard.yml": "Scorecard",
    "stale.yml": "stale sweep",
    "tierb.yml": "Tier B jobs",
    "welcome.yml": "first pull request comments",
}
DEPLOY_KEY_PURPOSES = {
    "board.yml": "LEDGER and board-data commits",
    "contributors.yml": "the nightly refresh of this list",
}
ECOSYSTEMS = {
    "bundler": "Bundler", "cargo": "Cargo", "composer": "Composer", "docker": "Docker",
    "github-actions": "GitHub Actions", "gomod": "Go modules", "gradle": "Gradle",
    "maven": "Maven", "npm": "npm", "nuget": "NuGet", "pip": "pip", "terraform": "Terraform",
    "uv": "uv",
}
CODEQL_LANGUAGES = {
    "actions": "Actions workflows", "c-cpp": "C/C++", "csharp": "C#", "go": "Go",
    "java-kotlin": "Java/Kotlin", "javascript": "JavaScript",
    "javascript-typescript": "JavaScript/TypeScript", "python": "Python", "ruby": "Ruby",
    "rust": "Rust", "swift": "Swift", "typescript": "TypeScript",
}


def workflow_purpose(name: str, text: str) -> str:
    if name in WORKFLOW_PURPOSES:
        return WORKFLOW_PURPOSES[name]
    m = re.search(r"^name:\s*['\"]?([^'\"\n#]+)", text, re.MULTILINE)
    return m.group(1).strip() if m else name.rsplit(".", 1)[0]


def dedupe(items: list[str]) -> list[str]:
    seen: list[str] = []
    for item in items:
        if item not in seen:
            seen.append(item)
    return seen


def codeql_workflows(files: RepoFiles) -> list[str]:
    """Workflows that run a CodeQL analysis (not ones that only upload someone else's SARIF)."""
    pattern = re.compile(r"github/codeql-action/(?:init|analyze)@")
    return [n for n, t in files.workflows.items() if pattern.search(t)]


def release_please_workflows(files: RepoFiles) -> list[str]:
    return [n for n, t in files.workflows.items() if "release-please-action" in t]


def deploy_key_workflows(files: RepoFiles) -> list[str]:
    return [n for n, t in files.workflows.items() if re.search(r"secrets\.BOT_DEPLOY_KEY\b", t)]


def role_github_actions(files: RepoFiles) -> str:
    if not files.workflows:
        return "Runs GitHub Actions jobs for this repository."
    purposes = dedupe([workflow_purpose(n, t) for n, t in files.workflows.items()])
    return f"Runs the workflows in .github/workflows: {and_join(purposes)}."


def role_dependabot(files: RepoFiles) -> str:
    if files.dependabot is None:
        return "Opens dependency update pull requests."
    found = re.findall(r"package-ecosystem:\s*['\"]?([\w-]+)", files.dependabot)
    names = dedupe([ECOSYSTEMS.get(e, e) for e in found])
    target = f" for {and_join(names)}" if names else ""
    return f"Opens dependency update pull requests{target} (.github/dependabot.yml)."


def role_codeql(files: RepoFiles) -> str:
    workflows = codeql_workflows(files)
    if not workflows:
        return "Code scanning: posts CodeQL results on pull requests."
    langs: list[str] = []
    for name in workflows:
        for lang in re.findall(r"^\s*-?\s*language:\s*['\"]?([\w-]+)", files.workflows[name], re.M):
            langs.append(CODEQL_LANGUAGES.get(lang, lang))
    langs = dedupe(langs)
    target = f" for {and_join(langs)}" if langs else ""
    return f"Code scanning with CodeQL ({', '.join(workflows)}){target}."


def role_deploy_key(files: RepoFiles) -> str:
    users = deploy_key_workflows(files)
    if not users:
        return "SSH deploy key with write access for jarrow-bot commits from Actions; no workflow uses it yet."
    def uses(names: list[str]) -> str:
        return "; ".join(f"{DEPLOY_KEY_PURPOSES.get(n, n)} ({n})" for n in names)

    via_pr = [n for n in users if "tools/bot_pr.sh" in files.workflows[n]]
    direct = [n for n in users if n not in via_pr]
    parts = []
    if via_pr:
        parts.append(f"as the branch of a bot pull request (tools/bot_pr.sh) for {uses(via_pr)}")
    if direct:
        parts.append(f"directly for {uses(direct)}")
    return f"SSH deploy key with write access. Actions pushes jarrow-bot commits with it, {', and '.join(parts)}."


# ---------------------------------------------------------------- bots registry


@dataclass(frozen=True)
class KnownBot:
    login: str
    name: str
    profile_url: str
    contributions: tuple[str, ...]
    app_slugs: tuple[str, ...] = ()
    bot_logins: tuple[str, ...] = ()
    app_id: int | None = None
    avatar_url: str | None = None
    icon: str | None = None
    # gh lookups that could confirm this bot; when one of them fails, a cached entry is kept.
    sources: frozenset[str] = frozenset({"check_suites", "check_runs", "pulls"})
    role: Callable[[RepoFiles], str] | None = None
    static_role: str = ""

    def role_text(self, files: RepoFiles) -> str:
        return self.role(files) if self.role else self.static_role


KNOWN_BOTS: tuple[KnownBot, ...] = (
    KnownBot(
        login="github-actions",
        name="GitHub Actions",
        profile_url="https://github.com/apps/github-actions",
        contributions=("ci", "infra"),
        app_slugs=("github-actions",),
        bot_logins=("github-actions",),
        app_id=15368,
        sources=frozenset({"check_suites", "check_runs"}),
        role=role_github_actions,
    ),
    KnownBot(
        login="github-advanced-security",
        name="CodeQL",
        profile_url="https://codeql.github.com",
        contributions=("ci", "security"),
        app_slugs=("github-advanced-security",),
        bot_logins=("github-advanced-security",),
        app_id=57789,
        sources=frozenset({"check_suites", "check_runs"}),
        role=role_codeql,
    ),
    KnownBot(
        login="dependabot",
        name="Dependabot",
        profile_url="https://github.com/apps/dependabot",
        contributions=("bot", "maintenance"),
        app_slugs=("dependabot",),
        bot_logins=("dependabot", "dependabot-preview"),
        app_id=29110,
        role=role_dependabot,
    ),
    KnownBot(
        login="socket-security",
        name="Socket Security",
        profile_url="https://github.com/apps/socket-security",
        contributions=("bot", "security"),
        app_slugs=("socket-security",),
        bot_logins=("socket-security",),
        app_id=156372,
        static_role=(
            "Dependency review: reports supply-chain risk in the project's dependencies and in "
            "those a pull request adds or changes."
        ),
    ),
    KnownBot(
        login="renovate",
        name="Renovate",
        profile_url="https://github.com/apps/renovate",
        contributions=("bot", "maintenance"),
        app_slugs=("renovate",),
        bot_logins=("renovate",),
        app_id=2740,
        static_role="Opens dependency update pull requests.",
    ),
    KnownBot(
        login="release-please",
        name="release-please",
        profile_url="https://github.com/googleapis/release-please",
        contributions=("bot", "maintenance"),
        bot_logins=("release-please",),
        avatar_url="https://github.com/googleapis.png",
        sources=frozenset({"pulls"}),
        static_role="Opens release pull requests and tags releases.",
    ),
    KnownBot(
        login="claude",
        name="Claude GitHub App",
        profile_url="https://github.com/apps/claude",
        contributions=("bot",),
        app_slugs=("claude",),
        bot_logins=("claude",),
        app_id=1236702,
        static_role="GitHub App from Anthropic through which Claude Code acts on this repository.",
    ),
    KnownBot(
        login=DEPLOY_KEY_LOGIN,
        name="jarrow-bot (deploy key)",
        profile_url="",  # set from the workflows that use the key
        contributions=("bot",),
        bot_logins=(DEPLOY_KEY_LOGIN,),
        icon="key",
        sources=frozenset({"keys"}),
        role=role_deploy_key,
    ),
)
KNOWN_BY_LOGIN = {b.login: b for b in KNOWN_BOTS}
KNOWN_BY_APP = {s: b for b in KNOWN_BOTS for s in b.app_slugs}
KNOWN_BY_BOT_LOGIN = {lg: b for b in KNOWN_BOTS for lg in b.bot_logins}
GENERIC_SOURCES = frozenset({"check_suites", "check_runs", "pulls"})

AGENT_PROVIDERS = (
    (re.compile(r"claude|anthropic", re.I), "anthropics", "https://www.anthropic.com/claude"),
    (re.compile(r"gpt|codex|openai", re.I), "openai", "https://openai.com"),
    (re.compile(r"gemini", re.I), "google-gemini", "https://gemini.google.com"),
    (re.compile(r"copilot", re.I), "github", "https://github.com/features/copilot"),
)
# Avatar URLs of GitHub accounts whose numeric id is known, as the users API returns them.
KNOWN_ACCOUNT_IDS = {"anthropics": 76263028, "github": 9919}


def app_avatar(app_id: int) -> str:
    return f"https://avatars.githubusercontent.com/in/{app_id}?v=4"


def user_avatar(user_id: int | str) -> str:
    return f"https://avatars.githubusercontent.com/u/{user_id}?v=4"


# ---------------------------------------------------------------- the data model


@dataclass
class Member:
    login: str
    name: str
    avatar_url: str | None
    profile_url: str
    role: str
    contributions: list[str]
    commits: int
    icon: str | None = None


@dataclass
class Group:
    id: str
    title: str
    note: str
    members: list[Member]


@dataclass
class Team:
    generated_at: str
    groups: list[Group]


# ---------------------------------------------------------------- collection


@dataclass
class Identity:
    name: str
    email: str
    # (sha, role) newest first; role is "author", "committer" or "coauthor"
    seen: list[tuple[str, str]] = field(default_factory=list)


class Collector:
    def __init__(
        self,
        root: Path,
        *,
        repo_slug: str,
        rev: str,
        gh: GhClient | None,
        log: Log,
    ) -> None:
        self.root = root
        self.repo_slug = repo_slug
        self.rev = rev
        self.gh = gh
        self.log = log
        self.cache = load_cache(root)
        self.failed: set[str] = set()

    # -- gh helpers

    def gh_api(self, path: str, source: str | None = None) -> object | None:
        if self.gh is None:
            if source:
                self.failed.add(source)
            return None
        try:
            return self.gh.api(path)
        except GhError as err:
            self.log(f"gh api {path} failed: {err}")
            if source:
                self.failed.add(source)
            return None

    def avatar_for_account(self, account: str, fallback: str | None) -> str | None:
        data = self.gh_api(f"users/{account}")
        if isinstance(data, dict) and isinstance(data.get("avatar_url"), str):
            return data["avatar_url"]
        return fallback

    # -- the three groups

    def collect(self) -> Team:
        commits = read_history(self.root, self.rev)
        files = read_repo_files(self.root)
        people = self.people(commits)
        agents = self.agents(commits)
        bots = self.bots(commits, files)
        groups = []
        for (gid, title, note), members in zip(GROUPS, (people, agents, bots)):
            groups.append(Group(gid, title, note, members))
        return Team(generated_at(commits), groups)

    def people(self, commits: list[Commit]) -> list[Member]:
        identities: dict[tuple[str, str], Identity] = {}

        def note(person: tuple[str, str], sha: str, role: str) -> None:
            name, email = person
            if identity_kind(name, email) != "human":
                return
            key = (name, email.lower())
            identities.setdefault(key, Identity(name, email)).seen.append((sha, role))

        for c in commits:
            note(c.author, c.sha, "author")
            if c.committer != c.author:
                note(c.committer, c.sha, "committer")
            for person in c.coauthors:
                note(person, c.sha, "coauthor")

        cached_people = {
            lg.lower(): lg for lg, entry in self.cache.items() if group_of(entry) == "people"
        }
        by_login: dict[str, dict] = {}
        unresolved: list[str] = []
        files_by_sha = {c.sha: c.files for c in commits}
        for key in sorted(identities, key=lambda k: (k[0].lower(), k[1])):
            ident = identities[key]
            login, avatar = self.resolve_login(ident, cached_people)
            if login is None:
                unresolved.append(ident.seen[0][0][:12])
                continue
            entry = by_login.setdefault(login.lower(), {"login": login, "avatar": None, "shas": set()})
            entry["avatar"] = entry["avatar"] or avatar
            for sha, role in ident.seen:
                if role in ("author", "coauthor"):
                    entry["shas"].add(sha)
        if unresolved:
            self.log(
                f"{len(unresolved)} commit identity(ies) have no GitHub login and are left out "
                f"(newest commit of each: {', '.join(sorted(unresolved))})"
            )

        owner_key = OWNER_LOGIN.lower()
        owner = by_login.pop(owner_key, {"login": OWNER_LOGIN, "avatar": None, "shas": set()})
        members = [
            Member(
                login=OWNER_LOGIN,
                name=OWNER_LOGIN,
                avatar_url=self.person_avatar(OWNER_LOGIN, owner["avatar"]),
                profile_url=f"https://github.com/{OWNER_LOGIN}",
                role=OWNER_ROLE,
                contributions=["ideas", "projectManagement"],
                commits=len(owner["shas"]),
            )
        ]
        others = []
        for entry in by_login.values():
            login = entry["login"]
            types: set[str] = set()
            for sha in entry["shas"]:
                for path in files_by_sha.get(sha, []):
                    types |= path_types(path)
            others.append(
                Member(
                    login=login,
                    name=login,
                    avatar_url=self.person_avatar(login, entry["avatar"]),
                    profile_url=f"https://github.com/{login}",
                    role=PERSON_ROLE,
                    contributions=ordered_types(types or {"maintenance"}),
                    commits=len(entry["shas"]),
                )
            )
        others.sort(key=lambda m: (-m.commits, m.login.lower(), m.login))
        return members + others

    def resolve_login(
        self, ident: Identity, cached_people: dict[str, str]
    ) -> tuple[str | None, str | None]:
        m = NOREPLY.match(ident.email.strip())
        if m:
            return m.group(2), user_avatar(m.group(1)) if m.group(1) else None
        if ident.name.lower() in cached_people:
            return cached_people[ident.name.lower()], None
        if self.gh is None:
            return None, None
        tried = 0
        for sha, role in ident.seen:
            if role == "coauthor" or tried >= 3:
                continue
            tried += 1
            data = self.gh_api(f"repos/{self.repo_slug}/commits?sha={sha}&per_page=1")
            if not isinstance(data, list) or not data or not isinstance(data[0], dict):
                continue
            if data[0].get("sha") != sha:
                continue
            account = data[0].get(role)
            if isinstance(account, dict) and isinstance(account.get("login"), str):
                if account.get("type") == "Bot":
                    return None, None
                avatar = account.get("avatar_url")
                return account["login"], avatar if isinstance(avatar, str) else None
        return None, None

    def person_avatar(self, login: str, known: str | None) -> str:
        if known:
            return known
        cached = self.cache.get(login, {}).get("avatar_url")
        if self.gh is not None:
            fetched = self.avatar_for_account(login, None)
            if fetched:
                return fetched
        if isinstance(cached, str) and cached:
            return cached
        return f"https://github.com/{login}.png"

    def agents(self, commits: list[Commit]) -> list[Member]:
        found: dict[str, dict] = {}
        for c in commits:
            named: dict[str, tuple[str, str]] = {}
            for person in (c.author, c.committer, *c.coauthors):
                name, email = person
                if identity_kind(name, email) == "agent":
                    named.setdefault(" ".join(name.split()), person)
            for name, person in named.items():
                entry = found.setdefault(name, {"email": person[1], "shas": set(), "types": set()})
                entry["shas"].add(c.sha)
                for path in c.files:
                    entry["types"] |= path_types(path)
        members = []
        for name, entry in found.items():
            login = slug(name) or "agent"
            provider = None
            for pattern, account, profile in AGENT_PROVIDERS:
                if pattern.search(f"{name} {entry['email']}"):
                    provider = (account, profile)
                    break
            avatar: str | None = None
            profile_url = f"https://github.com/{self.repo_slug}"
            if provider:
                account, profile_url = provider
                default = (
                    user_avatar(KNOWN_ACCOUNT_IDS[account])
                    if account in KNOWN_ACCOUNT_IDS
                    else f"https://github.com/{account}.png"
                )
                cached = self.cache.get(login, {}).get("avatar_url")
                if account in KNOWN_ACCOUNT_IDS:
                    avatar = default
                elif self.gh is not None:
                    avatar = self.avatar_for_account(account, cached or default)
                else:
                    avatar = cached or default
            role = OPUS_ROLE if name.lower() == OPUS_NAME.lower() else AGENT_ROLE
            members.append(
                Member(
                    login=login,
                    name=name,
                    avatar_url=avatar,
                    profile_url=profile_url,
                    role=role,
                    contributions=ordered_types({"agent"} | entry["types"]),
                    commits=len(entry["shas"]),
                )
            )
        members.sort(key=lambda m: (-m.commits, m.name.lower(), m.name))
        return members

    def bots(self, commits: list[Commit], files: RepoFiles) -> list[Member]:
        found: dict[str, dict] = {}  # login -> {"known": KnownBot|None, "app": dict|None}

        def add(login: str, app: dict | None = None) -> None:
            known = KNOWN_BY_LOGIN.get(login)
            entry = found.setdefault(login, {"known": known, "app": None})
            if app and not entry["app"]:
                entry["app"] = app

        def add_app(app_slug: str, app: dict | None) -> None:
            known = KNOWN_BY_APP.get(app_slug)
            add(known.login if known else app_slug, None if known else app)

        # Repository files.
        if files.workflows:
            add("github-actions")
        if files.dependabot is not None:
            add("dependabot")
        if codeql_workflows(files):
            add("github-advanced-security")
        if files.renovate:
            add("renovate")
        if files.socket:
            add("socket-security")
        if release_please_workflows(files) or files.release_please_files:
            add("release-please")

        # Commit authors and committers that are bots; their commit counts.
        counts: dict[str, int] = {}
        for c in commits:
            for index, person in enumerate((c.author, c.committer)):
                if identity_kind(*person) != "bot":
                    continue
                raw = bot_login(*person)
                known = KNOWN_BY_BOT_LOGIN.get(raw)
                login = known.login if known else raw
                add(login, None if known else {"slug": raw, "kind": "author"})
                if index == 0:
                    counts[login] = counts.get(login, 0) + 1

        # GitHub: installations (needs a GitHub App token; usually fails), checks, PRs, keys.
        if self.gh is not None:
            for app in self.installed_apps():
                add_app(app["slug"], app)
            # Checks on the newest counted commit, not on the branch tip: the tip may be a
            # refresh commit, and the result must not depend on which apps ran on it. When
            # that commit is not on GitHub (not pushed yet), the branch tip stands in.
            newest = max(commits, key=lambda c: datetime.fromisoformat(c.date)).sha
            for source, endpoint in (("check_suites", "check-suites"), ("check_runs", "check-runs")):
                data = None
                for ref in (newest, DEFAULT_BRANCH):
                    data = self.gh_api(f"repos/{self.repo_slug}/commits/{ref}/{endpoint}?per_page=100")
                    if isinstance(data, dict):
                        break
                if not isinstance(data, dict):
                    self.failed.add(source)
                    continue
                for item in data.get(source) or []:
                    item = item or {}
                    # On every push GitHub creates a check suite for each installed app that has
                    # the checks permission, so an app that never acted still has an empty one.
                    # Only a suite with check runs shows that the app did something.
                    if source == "check_suites" and item.get("latest_check_runs_count", 1) == 0:
                        continue
                    app = item.get("app") or {}
                    if isinstance(app.get("slug"), str):
                        add_app(app["slug"], app_summary(app))
            try:
                for author in self.gh.pr_authors(self.repo_slug):
                    login = str(author.get("login") or "")
                    is_bot = bool(author.get("is_bot")) or login.startswith("app/") or login.endswith("[bot]")
                    if not is_bot:
                        continue
                    raw = login.removeprefix("app/").removesuffix("[bot]")
                    known = KNOWN_BY_BOT_LOGIN.get(raw) or KNOWN_BY_APP.get(raw)
                    add(known.login if known else raw, None if known else {"slug": raw, "kind": "author"})
            except GhError as err:
                self.log(f"gh pr list failed: {err}")
                self.failed.add("pulls")
            keys = self.gh_api(f"repos/{self.repo_slug}/keys?per_page=100", "keys")
            if isinstance(keys, list):
                for key in keys:
                    title = str((key or {}).get("title") or "")
                    if title.lower().startswith(DEPLOY_KEY_LOGIN):
                        add(DEPLOY_KEY_LOGIN)
        else:
            self.failed |= {"check_suites", "check_runs", "pulls", "keys"}

        # Keep cached bots that a failed lookup could have confirmed.
        for login, entry in self.cache.items():
            if group_of(entry) != "bots" or login in found:
                continue
            known = KNOWN_BY_LOGIN.get(login)
            sources = known.sources if known else GENERIC_SOURCES
            if sources & self.failed:
                found[login] = {"known": known, "app": None, "cached": entry}

        members = [self.bot_member(login, entry, files, counts.get(login, 0)) for login, entry in found.items()]
        members.sort(key=lambda m: (m.name.lower(), m.login))
        return members

    def installed_apps(self) -> list[dict]:
        """Apps whose installation covers the repository, when the token can list them."""
        for path in (f"repos/{self.repo_slug}/installations", "user/installations"):
            data = self.gh_api(path, "installations")
            if not isinstance(data, dict):
                continue
            apps = []
            for inst in data.get("installations") or []:
                inst = inst or {}
                slug_ = inst.get("app_slug")
                if not isinstance(slug_, str):
                    continue
                if not self.installation_covers(inst):
                    continue
                apps.append({"slug": slug_, "id": inst.get("app_id"), "name": None, "owner": None})
            self.failed.discard("installations")
            return apps
        return []

    def installation_covers(self, inst: dict) -> bool:
        owner, _, name = self.repo_slug.partition("/")
        account = ((inst.get("account") or {}).get("login") or "").lower()
        if inst.get("repository_selection") == "all":
            return account == owner.lower()
        data = self.gh_api(f"user/installations/{inst.get('id')}/repositories?per_page=100")
        if not isinstance(data, dict):
            return False
        return any(
            str((r or {}).get("full_name") or "").lower() == self.repo_slug.lower()
            for r in data.get("repositories") or []
        )

    def bot_member(self, login: str, entry: dict, files: RepoFiles, commits: int) -> Member:
        known: KnownBot | None = entry.get("known")
        cached: dict = entry.get("cached") or self.cache.get(login) or {}
        if known is not None:
            avatar: str | None
            if known.icon:
                avatar = None
            elif known.app_id is not None:
                avatar = app_avatar(known.app_id)
            else:
                avatar = known.avatar_url
            profile = known.profile_url
            if known.login == DEPLOY_KEY_LOGIN:
                users = deploy_key_workflows(files)
                profile = (
                    f"https://github.com/{self.repo_slug}/blob/{DEFAULT_BRANCH}/.github/workflows/{users[0]}"
                    if users
                    else f"https://github.com/{self.repo_slug}/blob/{DEFAULT_BRANCH}/{TEAM_MD_PATH}"
                )
            return Member(
                login=known.login,
                name=known.name,
                avatar_url=avatar,
                profile_url=profile,
                role=known.role_text(files),
                contributions=ordered_types(known.contributions),
                commits=commits,
                icon=known.icon,
            )
        app = entry.get("app") or {}
        name = app.get("name") or cached.get("name") or login
        avatar = None
        if self.gh is not None:
            avatar = self.avatar_for_account(f"{login}[bot]", None)
        if not avatar:
            if isinstance(app.get("id"), int):
                avatar = app_avatar(app["id"])
            elif isinstance(cached.get("avatar_url"), str) and cached["avatar_url"]:
                avatar = cached["avatar_url"]
        cached_types = [t for t in cached.get("contributions") or [] if t in TYPES]
        return Member(
            login=login,
            name=str(name),
            avatar_url=avatar,
            profile_url=cached.get("profile") or f"https://github.com/apps/{login}",
            role=GENERIC_BOT_ROLE,
            contributions=ordered_types(cached_types or ["bot"]),
            commits=commits,
        )


def app_summary(app: dict) -> dict:
    return {
        "slug": app.get("slug"),
        "id": app.get("id") if isinstance(app.get("id"), int) else None,
        "name": app.get("name") if isinstance(app.get("name"), str) else None,
        "owner": ((app.get("owner") or {}).get("login")),
    }


# ---------------------------------------------------------------- cache (.all-contributorsrc)


def group_of(entry: dict) -> str:
    types = set(entry.get("contributions") or [])
    if "agent" in types:
        return "agents"
    if types & {"bot", "ci"}:
        return "bots"
    return "people"


def load_cache(root: Path) -> dict[str, dict]:
    text = read_text(root / RC_PATH)
    if text is None:
        return {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError as err:
        raise InputError(f"{RC_PATH} is not valid JSON: {err}") from err
    cache = {}
    for entry in data.get("contributors") or []:
        if isinstance(entry, dict) and isinstance(entry.get("login"), str):
            cache[entry["login"]] = entry
    return cache


# ---------------------------------------------------------------- rendering


def esc(text: str) -> str:
    return html.escape(text, quote=True)


def sized(url: str, size: int) -> str:
    if url.startswith("https://avatars.githubusercontent.com/"):
        return f"{url}{'&' if '?' in url else '?'}s={size}"
    if re.match(r"^https://github\.com/[^/?#]+\.png$", url):
        return f"{url}?size={size}"
    return url


def commits_text(n: int) -> str:
    return f"{n} commit" if n == 1 else f"{n} commits"


def rc_avatar(member: Member) -> str:
    if member.icon == "key":
        return KEY_ICON_PATH
    return member.avatar_url or ""


def render_rc(team: Team) -> str:
    contributors = []
    for group in team.groups:
        for m in group.members:
            contributors.append(
                {
                    "login": m.login,
                    "name": m.name,
                    "avatar_url": rc_avatar(m),
                    "profile": m.profile_url,
                    "contributions": m.contributions,
                }
            )
    data = {
        "projectName": PROJECT_NAME,
        "projectOwner": OWNER_LOGIN,
        "repoType": "github",
        "repoHost": "https://github.com",
        "files": [README_PATH],
        "imageSize": IMAGE_SIZE,
        "commit": False,
        "contributorsPerLine": PER_LINE,
        "linkToUsage": False,
        "types": {
            t: {"symbol": TYPES[t].symbol, "description": TYPES[t].description} for t in CUSTOM_TYPES
        },
        "contributors": contributors,
    }
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def contribution_links(m: Member, group_id: str, repo_slug: str) -> str:
    links = []
    for t in m.contributions:
        ct = TYPES[t]
        if group_id == "people" and t in COMMIT_LINK_TYPES:
            href = f"https://github.com/{repo_slug}/commits?author={m.login}"
        else:
            href = f"#{t}-{m.login}"
        links.append(f'<a href="{esc(href)}" title="{esc(ct.description)}">{ct.symbol}</a>')
    return " ".join(links)


def show_commits(group_id: str, m: Member) -> bool:
    return group_id in ("people", "agents") or m.commits > 0


def render_readme_block(team: Team, repo_slug: str) -> str:
    width = f"{int(100 / PER_LINE * 100) / 100:.2f}%"
    lines = ["<!-- prettier-ignore-start -->", "<!-- markdownlint-disable -->"]
    for group in team.groups:
        lines += [f"### {group.title}", "", group.note, ""]
        if not group.members:
            lines += ["None yet.", ""]
            continue
        lines += ["<table>", "  <tbody>"]
        for start in range(0, len(group.members), PER_LINE):
            lines.append("    <tr>")
            for m in group.members[start : start + PER_LINE]:
                avatar = rc_avatar(m)
                label = f"<sub><b>{esc(m.name)}</b></sub>"
                if avatar:
                    img = f'<img src="{esc(sized(avatar, IMAGE_SIZE))}" width="{IMAGE_SIZE}px;" alt="{esc(m.name)}"/>'
                    label = f"{img}<br />{label}"
                if m.profile_url:
                    label = f'<a href="{esc(m.profile_url)}">{label}</a>'
                parts = [label, f"<sub>{esc(m.role)}</sub>"]
                if show_commits(group.id, m):
                    parts.append(f"<sub>{commits_text(m.commits)}</sub>")
                parts.append(contribution_links(m, group.id, repo_slug))
                lines.append(
                    f'      <td align="center" valign="top" width="{width}">{"<br />".join(parts)}</td>'
                )
            lines.append("    </tr>")
        lines += ["  </tbody>", "</table>", ""]
    lines += ["<!-- markdownlint-restore -->", "<!-- prettier-ignore-end -->", ""]
    return "\n" + "\n".join(lines) + "\n"


def replace_readme_block(readme: str, block: str) -> str:
    readme = readme.replace("\r\n", "\n")
    starts = readme.count(START_MARKER)
    ends = readme.count(END_MARKER)
    if starts == 0 or ends == 0:
        missing = " and ".join(
            marker for marker, n in ((START_MARKER, starts), (END_MARKER, ends)) if n == 0
        )
        raise InputError(
            f"{README_PATH} lacks the marker(s) {missing}. Add both markers, START before END, "
            "where the list belongs, or run with --no-readme to skip README.md"
        )
    if starts > 1 or ends > 1:
        raise InputError(f"{README_PATH} has more than one START or END marker; keep exactly one of each")
    head, rest = readme.split(START_MARKER, 1)
    if END_MARKER not in rest:
        raise InputError(f"{README_PATH}: the END marker comes before the START marker")
    _, tail = rest.split(END_MARKER, 1)
    return f"{head}{START_MARKER}{block}{END_MARKER}{tail}"


def md_cell(text: str) -> str:
    return text.replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def render_team_md(team: Team) -> str:
    out = [
        "# Who works on Jarrow",
        "",
        "Generated by tools/contributors.py. Do not edit by hand.",
        "",
        "The people, agents, and bots and automation that work on this repository, and what "
        "each one does. The same list is in the [README](../README.md#who-works-on-jarrow) "
        "and on the site's [About page](https://jarrow.pages.dev/about/).",
        "",
        "Commit counts come from `git log` on `main`, up to the newest counted commit "
        f"({team.generated_at}). The refresh commits that update this list are not counted. "
        "A commit counts for every agent its trailers name.",
        "",
    ]
    for group in team.groups:
        out += [f"## {group.title}", "", group.note, ""]
        if not group.members:
            out += ["None yet.", ""]
            continue
        out += ["| | Who | Role | Commits | Contributions |", "| --- | --- | --- | ---: | --- |"]
        for m in group.members:
            if m.icon == "key":
                src = "assets/key.svg"
            else:
                src = sized(m.avatar_url, TEAM_MD_IMAGE_SIZE) if m.avatar_url else ""
            img = (
                f'<img src="{esc(src)}" width="{TEAM_MD_IMAGE_SIZE}" height="{TEAM_MD_IMAGE_SIZE}" alt="">'
                if src
                else ""
            )
            who = f"[{md_cell(m.name)}]({m.profile_url})" if m.profile_url else md_cell(m.name)
            types = "; ".join(f"{TYPES[t].symbol} {TYPES[t].description}" for t in m.contributions)
            out.append(f"| {img} | {who} | {md_cell(m.role)} | {m.commits} | {md_cell(types)} |")
        out.append("")
    return "\n".join(out)


def render_team_json(team: Team) -> str:
    used = ordered_types({t for g in team.groups for m in g.members for t in m.contributions})
    data = {
        "generated_at": team.generated_at,
        "groups": [
            {
                "id": g.id,
                "title": g.title,
                "note": g.note,
                "members": [
                    {
                        "login": m.login,
                        "name": m.name,
                        "avatar_url": m.avatar_url,
                        "profile_url": m.profile_url,
                        "role": m.role,
                        "contributions": m.contributions,
                        "commits": m.commits,
                        "icon": m.icon,
                    }
                    for m in g.members
                ],
            }
            for g in team.groups
        ],
        "types": {t: {"symbol": TYPES[t].symbol, "description": TYPES[t].description} for t in used},
    }
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def render_outputs(team: Team, root: Path, *, readme: bool, repo_slug: str) -> dict[str, str]:
    outputs = {
        RC_PATH: render_rc(team),
        TEAM_MD_PATH: render_team_md(team),
        TEAM_JSON_PATH: render_team_json(team),
    }
    if readme:
        current = read_text(root / README_PATH)
        if current is None:
            raise InputError(f"{README_PATH} not found under {root}")
        outputs[README_PATH] = replace_readme_block(current, render_readme_block(team, repo_slug))
    return dict(sorted(outputs.items()))


# ---------------------------------------------------------------- --ledger: contributor numbers


def commit_day(commit: Commit) -> date:
    return datetime.fromisoformat(commit.date).astimezone(timezone.utc).date()


def ledger_window(as_of: date) -> tuple[date, date]:
    """The as-of day and the LEDGER_WINDOW_DAYS - 1 days before it, both ends included."""
    return as_of - timedelta(days=LEDGER_WINDOW_DAYS - 1), as_of


def sorted_names(names: Iterable[str]) -> list[str]:
    return sorted(set(names), key=lambda n: (n.lower(), n))


def pr_author_is_bot(author: dict) -> bool:
    login = str(author.get("login") or "")
    return (
        bool(author.get("is_bot"))
        or login.startswith("app/")
        or identity_kind(login, "") == "bot"
    )


def ledger_numbers(
    root: Path,
    *,
    repo_slug: str = REPO_SLUG,
    rev: str = "HEAD",
    gh: GhClient | None = None,
    as_of: date,
    log: Log = stderr_log,
) -> dict:
    """The contributor numbers of the 30 days that end on as_of (see the module docstring)."""
    collector = Collector(root, repo_slug=repo_slug, rev=rev, gh=gh, log=log)
    start, end = ledger_window(as_of)
    commits = [
        c
        for c in read_history(root, rev)
        if start <= commit_day(c) <= end and identity_kind(*c.author) not in ("bot", "web-flow")
    ]

    identities: dict[tuple[str, str], Identity] = {}
    agents: set[str] = set()
    for c in commits:
        for role, person in [("author", c.author), *(("coauthor", p) for p in c.coauthors)]:
            name, email = person
            if identity_kind(name, email) == "human":
                identities.setdefault((name, email.lower()), Identity(name, email)).seen.append((c.sha, role))
        for person in (c.author, c.committer, *c.coauthors):
            if identity_kind(*person) == "agent":
                agents.add(" ".join(person[0].split()))

    cached_people = {lg.lower(): lg for lg, entry in collector.cache.items() if group_of(entry) == "people"}
    people: dict[str, str] = {}  # lower-case login -> login
    unresolved = 0
    for key in sorted(identities):
        login, _ = collector.resolve_login(identities[key], cached_people)
        if login is None:
            unresolved += 1
        else:
            people.setdefault(login.lower(), login)
    if unresolved:
        log(f"{unresolved} commit identity(ies) in the window have no GitHub login and are not counted")

    sources = ["git"]
    first_time: list[str] | None = None
    pulls: list[dict] | None = None
    if gh is None:
        log("no gh: merged pull requests are not read and first-time contributors are not counted")
    else:
        try:
            pulls = gh.merged_pulls(repo_slug)
        except GhError as err:
            log(f"gh pr list --state merged failed: {err}; first-time contributors are not counted")
    if pulls is not None:
        sources.append("github-pulls")
        first: dict[str, tuple[date, str]] = {}
        for pr in pulls:
            author = pr.get("author") or {}
            login = str(author.get("login") or "")
            merged = pr.get("mergedAt")
            if not login or pr_author_is_bot(author) or not isinstance(merged, str):
                continue
            try:
                day = datetime.fromisoformat(merged.replace("Z", "+00:00")).astimezone(timezone.utc).date()
            except ValueError:
                continue
            key = login.lower()
            if start <= day <= end:
                people.setdefault(key, login)
            if key not in first or day < first[key][0]:
                first[key] = (day, login)
        first_time = sorted_names(login for day, login in first.values() if start <= day <= end)

    people_list = sorted_names(people.values())
    agent_list = sorted_names(agents)
    return {
        "schema": LEDGER_SCHEMA,
        "as_of": as_of.isoformat(),
        "window_days": LEDGER_WINDOW_DAYS,
        "sources": sources,
        "contributors_30d": {
            "people": {"count": len(people_list), "logins": people_list},
            "agents": {"count": len(agent_list), "names": agent_list},
        },
        "first_time_contributors_30d": (
            None if first_time is None else {"count": len(first_time), "logins": first_time}
        ),
    }


def settle_as_of(root: Path, data: dict) -> dict:
    """Keeps the committed "as_of" when nothing but "as_of" would change."""
    current = read_text(root / LEDGER_INPUT_PATH)
    if current is None:
        return data
    try:
        old = json.loads(current)
    except json.JSONDecodeError:
        return data
    if isinstance(old, dict) and isinstance(old.get("as_of"), str) and {**old, "as_of": data["as_of"]} == data:
        return {**data, "as_of": old["as_of"]}
    return data


def render_ledger_input(data: dict) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


# ---------------------------------------------------------------- main


def generate(
    root: Path,
    *,
    repo_slug: str = REPO_SLUG,
    rev: str = "HEAD",
    gh: GhClient | None = None,
    readme: bool = True,
    log: Log = stderr_log,
) -> dict[str, str]:
    """Every output as {repository-relative path: content}. Writes nothing."""
    team = Collector(root, repo_slug=repo_slug, rev=rev, gh=gh, log=log).collect()
    return render_outputs(team, root, readme=readme, repo_slug=repo_slug)


def write_outputs(root: Path, outputs: dict[str, str]) -> list[str]:
    changed = []
    for rel, content in outputs.items():
        path = root / rel
        current = read_text(path)
        if current == content:
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(content)
        changed.append(rel)
    return changed


def stale_outputs(root: Path, outputs: dict[str, str]) -> list[str]:
    return [rel for rel, content in outputs.items() if read_text(root / rel) != content]


REPO_PATTERN = re.compile(r"[A-Za-z0-9-]+/[A-Za-z0-9._-]+")


def default_repo(root: Path) -> str:
    """OWNER/NAME of the github.com 'origin' remote, else REPO_SLUG."""
    try:
        url = git(root, "remote", "get-url", "origin").strip()
    except InputError:
        return REPO_SLUG
    m = re.match(r"^(?:https://github\.com/|git@github\.com:|ssh://git@github\.com/)(.+?)(?:\.git)?/?$", url)
    if m and REPO_PATTERN.fullmatch(m.group(1)):
        return m.group(1)
    return REPO_SLUG


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Generate the 'Who works on Jarrow' list.")
    parser.add_argument("--check", action="store_true", help="exit 1 if an output is stale; write nothing")
    parser.add_argument("--offline", action="store_true", help="git and repository files only, no gh")
    parser.add_argument("--no-readme", action="store_true", help="leave README.md untouched")
    parser.add_argument("--root", type=Path, help="repository root (default: git top level of the cwd)")
    parser.add_argument(
        "--repo", help=f"GitHub repository (default: the github.com origin remote, else {REPO_SLUG})"
    )
    parser.add_argument("--rev", default="HEAD", help="revision whose history is counted (default HEAD)")
    parser.add_argument(
        "--ledger", action="store_true", help=f"write only {LEDGER_INPUT_PATH} (contributor numbers)"
    )
    parser.add_argument(
        "--as-of", help="last day of the --ledger window, YYYY-MM-DD (default: today in UTC)"
    )
    args = parser.parse_args(argv)
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
    if args.repo is not None and not REPO_PATTERN.fullmatch(args.repo):
        parser.error("--repo must look like OWNER/NAME")
    as_of = datetime.now(timezone.utc).date()
    if args.as_of is not None:
        if not args.ledger:
            parser.error("--as-of applies only to --ledger")
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", args.as_of):
            parser.error("--as-of must look like YYYY-MM-DD")
        try:
            as_of = date.fromisoformat(args.as_of)
        except ValueError:
            parser.error("--as-of is not a calendar date")

    try:
        root = args.root or Path(git(Path.cwd(), "rev-parse", "--show-toplevel").strip())
        root = root.resolve()
        repo_slug = args.repo or default_repo(root)
        gh: GhClient | None = None
        if not args.offline:
            exe = shutil.which("gh")
            if exe is None:
                stderr_log("gh not found; continuing with git and repository files only")
            else:
                gh = GhCli(exe)
        if args.ledger:
            numbers = ledger_numbers(root, repo_slug=repo_slug, rev=args.rev, gh=gh, as_of=as_of)
            outputs = {LEDGER_INPUT_PATH: render_ledger_input(settle_as_of(root, numbers))}
        else:
            outputs = generate(root, repo_slug=repo_slug, rev=args.rev, gh=gh, readme=not args.no_readme)
    except InputError as err:
        print(f"contributors: {err}", file=sys.stderr)
        return 2

    if args.check:
        stale = stale_outputs(root, outputs)
        if stale:
            for rel in stale:
                print(f"stale: {rel}")
            print(f"Regenerate with: python tools/contributors.py{' --ledger' if args.ledger else ''}")
            return 1
        print("contributors: every output is up to date")
        return 0
    changed = write_outputs(root, outputs)
    for rel in outputs:
        print(f"{'wrote' if rel in changed else 'unchanged'}: {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
