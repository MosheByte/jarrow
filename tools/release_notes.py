#!/usr/bin/env python3
"""Write the "Changes since" and "Contributors" sections of a nightly's release notes.

The nightly workflow (.github/workflows/nightly.yml) runs this in its `changes` job and puts
the output into the release notes between the opening paragraphs and the CLI help.

Usage:
    python tools/release_notes.py --repo OWNER/NAME --tag nightly-YYYYMMDD [--head REV]
                                  [--out FILE] [--root DIR] [--server URL]

The range: from the newest tag nightly-YYYYMMDD older than --tag to --head (default HEAD).
The tag being published is never its own base, so a same-day rerun still compares with the
nightly before it. With no older nightly, the range starts after the first commit (the root
of --head).

Changes since <previous nightly>: every pull request merged into main whose merge commit is in
the range, with its number, title and author handle. They come from
    gh pr list --state merged --base main --search "merged:>=<base commit day - 1>"
filtered to merge commits in the range. A commit in the range that no listed pull request
accounts for is looked up with gh api repos/<repo>/commits/<sha>/pulls; when no merged pull
request into main holds it either, it is listed on its own as a commit without a pull request.

Contributors: every handle that authored or co-authored those changes. Pull request authors;
commit authors from git log (with .mailmap applied), bots such as dependabot[bot] included; and
every Co-Authored-By trailer. People are shown by GitHub handle (@login), taken from a GitHub
no-reply address or else from GitHub's own record of the commit's authors (GraphQL); bots by
their account name; agents (as tools/contributors.py recognises them) by name, for example
"Claude Opus 5.5". A person GitHub cannot match to an account is listed by the name in the
commit. No e-mail address is ever printed.

The sections go to stdout and, with --out, to FILE. Any git or gh failure is an error: a nightly
never ships a partial list.

Exit status: 0 written, 1 a git or gh failure, 2 a usage error. Standard library only.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Iterable, Protocol

sys.path.insert(0, str(Path(__file__).resolve().parent))
from contributors import NOREPLY, bot_login, identity_kind, parse_person  # noqa: E402

NIGHTLY_TAG = re.compile(r"^nightly-[0-9]{8}$")
DEFAULT_BRANCH = "main"
DEFAULT_SERVER = "https://github.com"
SEARCH_LIMIT = 1000  # the most GitHub's search returns
GRAPHQL_BATCH = 50


class NotesError(RuntimeError):
    """A git or gh call failed, or the input cannot be read."""


# ---------------------------------------------------------------- git


def git(root: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-c", "core.quotepath=off", "-C", str(root), *args],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        message = proc.stderr.decode("utf-8", "replace").strip()
        raise NotesError(f"git {' '.join(args)} failed: {message}")
    return proc.stdout.decode("utf-8", "replace")


@dataclass
class Commit:
    sha: str
    subject: str
    author: tuple[str, str]
    coauthors: list[tuple[str, str]]


def resolve(root: Path, rev: str) -> str:
    return git(root, "rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}").strip()


def previous_nightly(root: Path, tag: str) -> str | None:
    """The newest nightly-YYYYMMDD tag that sorts before `tag`, or None."""
    older = [t for t in git(root, "tag", "--list", "nightly-*").split() if NIGHTLY_TAG.match(t) and t < tag]
    return max(older) if older else None


def first_commit(root: Path, head: str) -> str:
    roots = git(root, "rev-list", "--max-parents=0", head).split()
    if not roots:
        raise NotesError(f"no root commit behind {head}")
    return roots[-1]


def read_commits(root: Path, base: str, head: str) -> list[Commit]:
    """Commits in base..head, oldest first."""
    fmt = "%H%x1f%aN%x1f%aE%x1f%s%x1f%(trailers:key=Co-authored-by,valueonly,unfold,separator=%x1d)%x1e"
    raw = git(root, "log", "--reverse", f"--format={fmt}", f"{base}..{head}", "--")
    commits = []
    for record in raw.split("\x1e"):
        record = record.strip("\n")
        if not record:
            continue
        sha, name, email, subject, trailers = (record.split("\x1f") + [""] * 5)[:5]
        coauthors: list[tuple[str, str]] = []
        for value in trailers.split("\x1d"):
            person = parse_person(value)
            if value.strip() and person not in coauthors:
                coauthors.append(person)
        commits.append(Commit(sha.strip(), subject.strip(), (name, email), coauthors))
    return commits


def commit_day(root: Path, rev: str) -> datetime:
    stamp = git(root, "log", "-1", "--format=%cI", rev, "--").strip()
    return datetime.fromisoformat(stamp).astimezone(timezone.utc)


# ---------------------------------------------------------------- GitHub


@dataclass(frozen=True)
class Pull:
    number: int
    title: str
    author: str  # a handle: "MosheByte", "dependabot[bot]"
    is_bot: bool
    merge_sha: str


class Gh(Protocol):
    def merged_pulls(self, repo: str, since: str) -> list[Pull]: ...

    def commit_pulls(self, repo: str, sha: str) -> list[Pull]: ...

    def commit_logins(self, repo: str, shas: list[str]) -> dict[str, dict[str, str]]: ...


def pull_author(login: str, is_bot: bool) -> str:
    """gh pr list names an app "app/dependabot"; its account handle is "dependabot[bot]"."""
    login = login.strip()
    if login.startswith("app/"):
        login, is_bot = login[4:], True
    if is_bot and not login.endswith("[bot]"):
        login += "[bot]"
    return login


class GhCli:
    """The gh CLI. A failure becomes NotesError with gh's own last line (never a token)."""

    def __init__(self, exe: str) -> None:
        self.exe = exe

    def _run(self, args: list[str]) -> object:
        env = {**os.environ, "GH_PROMPT_DISABLED": "1", "GH_NO_UPDATE_NOTIFIER": "1", "GH_PAGER": "cat", "NO_COLOR": "1"}
        try:
            proc = subprocess.run([self.exe, *args], capture_output=True, check=False, env=env, timeout=120)
        except (OSError, subprocess.TimeoutExpired) as err:
            raise NotesError(f"gh {args[0]} failed: {err}") from err
        if proc.returncode != 0:
            lines = [ln.strip() for ln in proc.stderr.decode("utf-8", "replace").splitlines() if ln.strip()]
            raise NotesError(f"gh {args[0]} failed: {lines[-1] if lines else f'exit {proc.returncode}'}")
        out = proc.stdout.decode("utf-8", "replace")
        return json.loads(out) if out.strip() else None

    def merged_pulls(self, repo: str, since: str) -> list[Pull]:
        data = self._run(
            [
                "pr", "list", "--repo", repo, "--state", "merged", "--base", DEFAULT_BRANCH,
                "--search", f"merged:>={since}", "--limit", str(SEARCH_LIMIT),
                "--json", "number,title,author,mergeCommit",
            ]
        )
        pulls = []
        for item in data or []:
            author = item.get("author") or {}
            handle = pull_author(author.get("login") or "ghost", bool(author.get("is_bot")))
            merge = (item.get("mergeCommit") or {}).get("oid") or ""
            pulls.append(Pull(int(item["number"]), str(item.get("title") or ""), handle, handle.endswith("[bot]"), merge))
        return pulls

    def commit_pulls(self, repo: str, sha: str) -> list[Pull]:
        data = self._run(["api", "-H", "Accept: application/vnd.github+json", f"repos/{repo}/commits/{sha}/pulls"])
        pulls = []
        for item in data or []:
            if not item.get("merged_at") or (item.get("base") or {}).get("ref") != DEFAULT_BRANCH:
                continue
            user = item.get("user") or {}
            handle = pull_author(user.get("login") or "ghost", user.get("type") == "Bot")
            pulls.append(
                Pull(int(item["number"]), str(item.get("title") or ""), handle, handle.endswith("[bot]"), str(item.get("merge_commit_sha") or ""))
            )
        return pulls

    def commit_logins(self, repo: str, shas: list[str]) -> dict[str, dict[str, str]]:
        """For each commit, {lower-case e-mail: login} of the authors GitHub matched to an account."""
        owner, name = repo.split("/", 1)
        found: dict[str, dict[str, str]] = {}
        for start in range(0, len(shas), GRAPHQL_BATCH):
            batch = shas[start : start + GRAPHQL_BATCH]
            fields = " ".join(
                f'c{i}: object(oid: "{sha}") {{ ... on Commit {{ oid authors(first: 50) {{ nodes {{ email user {{ login }} }} }} }} }}'
                for i, sha in enumerate(batch)
            )
            query = f"query($owner: String!, $name: String!) {{ repository(owner: $owner, name: $name) {{ {fields} }} }}"
            data = self._run(["api", "graphql", "-f", f"query={query}", "-f", f"owner={owner}", "-f", f"name={name}"])
            repository = ((data or {}).get("data") or {}).get("repository") or {}
            for node in repository.values():
                if not node:
                    continue
                logins = found.setdefault(node["oid"], {})
                for author in (node.get("authors") or {}).get("nodes") or []:
                    login = (author.get("user") or {}).get("login")
                    if login and author.get("email"):
                        logins.setdefault(author["email"].strip().lower(), login)
        return found


# ---------------------------------------------------------------- the notes


@dataclass
class Contributors:
    people: dict[str, str] = field(default_factory=dict)  # lower-case login -> login
    agents: dict[str, str] = field(default_factory=dict)
    bots: dict[str, str] = field(default_factory=dict)
    unmatched: dict[str, str] = field(default_factory=dict)  # people without a GitHub account

    @staticmethod
    def _add(group: dict[str, str], value: str) -> None:
        value = " ".join(value.split())
        if value:
            group.setdefault(value.lower(), value)

    def empty(self) -> bool:
        return not (self.people or self.agents or self.bots or self.unmatched)


@dataclass
class Notes:
    repo: str
    head: str
    base: str
    base_tag: str | None
    pulls: list[Pull]
    loose: list[tuple[Commit, str]]  # commits without a pull request, with the author's label
    contributors: Contributors


def bot_handle(name: str, email: str) -> str:
    login = bot_login(name, email)
    if name.strip().lower().endswith("[bot]") or email.strip().lower().endswith("[bot]@users.noreply.github.com"):
        return f"{login}[bot]"
    return login


def needs_lookup(name: str, email: str) -> bool:
    return identity_kind(name, email) == "human" and not NOREPLY.match(email.strip())


def add_identity(who: Contributors, name: str, email: str, login: str | None) -> str | None:
    """Adds one commit author or co-author; returns how it is shown (None for GitHub itself)."""
    kind = identity_kind(name, email)
    if kind == "web-flow":
        return None
    if kind == "bot":
        handle = bot_handle(name, email)
        who._add(who.bots, handle)
        return f"`{handle}`"
    if kind == "agent":
        who._add(who.agents, name)
        return " ".join(name.split())
    noreply = NOREPLY.match(email.strip())
    login = noreply.group(2) if noreply else login
    if login:
        who._add(who.people, login)
        return f"@{login}"
    who._add(who.unmatched, name or "unknown")
    return " ".join(name.split()) or "unknown"


def collect(root: Path, repo: str, tag: str, head: str, gh: Gh) -> Notes:
    head = resolve(root, head)
    base_tag = previous_nightly(root, tag)
    base = resolve(root, base_tag) if base_tag else first_commit(root, head)
    commits = read_commits(root, base, head)
    in_range = {c.sha for c in commits}

    pulls: dict[int, Pull] = {}
    accounted: set[str] = set()
    if commits:
        since = (commit_day(root, base) - timedelta(days=1)).strftime("%Y-%m-%d")
        listed = gh.merged_pulls(repo, since)
        if len(listed) >= SEARCH_LIMIT:
            raise NotesError(f"{len(listed)} merged pull requests since {since}: more than the search can list")
        for pull in listed:
            if pull.merge_sha in in_range:
                pulls[pull.number] = pull
                accounted.add(pull.merge_sha)
    orphans = []
    for commit in commits:
        if commit.sha in accounted:
            continue
        holding = gh.commit_pulls(repo, commit.sha)
        for pull in holding:
            pulls.setdefault(pull.number, pull)
        if not holding:
            orphans.append(commit)

    lookup = [c.sha for c in commits if any(needs_lookup(n, e) for n, e in [c.author, *c.coauthors])]
    logins = gh.commit_logins(repo, lookup) if lookup else {}

    who = Contributors()
    for pull in pulls.values():
        who._add(who.bots if pull.is_bot else who.people, pull.author)
    labels: dict[str, str] = {}
    for commit in commits:
        known = logins.get(commit.sha, {})
        for index, (name, email) in enumerate([commit.author, *commit.coauthors]):
            label = add_identity(who, name, email, known.get(email.strip().lower()))
            if index == 0:
                labels[commit.sha] = label or "GitHub"
    loose = [(c, labels[c.sha]) for c in orphans]
    return Notes(repo, head, base, base_tag, sorted(pulls.values(), key=lambda p: p.number), loose, who)


MARKUP = re.compile(r"([\\`*_\[\]<>|])")


def escape(text: str) -> str:
    """A title or subject as plain text in Markdown."""
    return MARKUP.sub(r"\\\1", " ".join(text.split()))


def plural(count: int, word: str) -> str:
    return f"{count} {word}{'' if count == 1 else 's'}"


def joined(values: Iterable[str]) -> str:
    return ", ".join(values)


def render(notes: Notes, server: str = DEFAULT_SERVER) -> str:
    base_url = f"{server.rstrip('/')}/{notes.repo}"
    short = notes.base[:7]
    head_short = notes.head[:7]
    commit = f"[`{short}`]({base_url}/commit/{notes.base})"
    this = f"[`{head_short}`]({base_url}/commit/{notes.head})"
    lines: list[str] = []
    if notes.base_tag:
        lines.append(f"## Changes since {notes.base_tag}")
        lines.append("")
        intro = f"Everything that landed on `main` after [{notes.base_tag}]({base_url}/releases/tag/{notes.base_tag}) ({commit}), up to this build ({this})"
    else:
        lines.append("## Changes since the first commit")
        lines.append("")
        intro = f"There is no earlier nightly, so this lists everything that landed on `main` after the first commit ({commit}), up to this build ({this})"
    if not notes.pulls and not notes.loose:
        lines.append(f"{intro}: nothing. This build is from the same commit.")
    else:
        counts = [plural(len(notes.pulls), "pull request")]
        if notes.loose:
            counts.append(plural(len(notes.loose), "commit") + " without one")
        lines.append(f"{intro}: {' and '.join(counts)}. [Compare]({base_url}/compare/{notes.base}...{notes.head}).")
        if notes.pulls:
            lines.append("")
            for pull in notes.pulls:
                author = f"`{pull.author}`" if pull.is_bot else f"@{pull.author}"
                lines.append(f"- #{pull.number} {escape(pull.title)} by {author}")
        if notes.loose:
            lines.append("")
            lines.append("Commits without a pull request:")
            lines.append("")
            for item, label in notes.loose:
                lines.append(f"- [`{item.sha[:7]}`]({base_url}/commit/{item.sha}) {escape(item.subject)} by {label}")

    lines.append("")
    lines.append("## Contributors")
    lines.append("")
    who = notes.contributors
    if who.empty():
        lines.append("No change, so nobody to list.")
    else:
        lines.append(
            "Everyone who authored or co-authored these changes: pull request authors, commit authors and "
            "every `Co-Authored-By` trailer. People and bots by GitHub handle, agents by name."
        )
        lines.append("")
        key = str.lower
        if who.people:
            lines.append("- People: " + joined(f"@{v}" for v in sorted(who.people.values(), key=key)))
        if who.agents:
            lines.append("- Agents: " + joined(sorted(who.agents.values(), key=key)))
        if who.bots:
            lines.append("- Bots: " + joined(f"`{v}`" for v in sorted(who.bots.values(), key=key)))
        if who.unmatched:
            lines.append(
                "- People GitHub has no account for (by the name in the commit): "
                + joined(escape(v) for v in sorted(who.unmatched.values(), key=key))
            )
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------- command line


def main(argv: list[str] | None = None, gh: Gh | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo", required=True, help="OWNER/NAME on GitHub")
    parser.add_argument("--tag", required=True, help="the nightly-YYYYMMDD tag being published")
    parser.add_argument("--head", default="HEAD", help="the commit being built (default HEAD)")
    parser.add_argument("--out", type=Path, help="also write the sections to this file")
    parser.add_argument("--root", type=Path, help="the repository (default: the git top level of the current directory)")
    parser.add_argument("--server", default=DEFAULT_SERVER, help=f"GitHub's web address (default {DEFAULT_SERVER})")
    args = parser.parse_args(argv)
    if not NIGHTLY_TAG.match(args.tag):
        parser.error(f"--tag must look like nightly-YYYYMMDD, got {args.tag!r}")
    if not re.fullmatch(r"[\w.-]+/[\w.-]+", args.repo):
        parser.error(f"--repo must be OWNER/NAME, got {args.repo!r}")
    try:
        root = args.root or Path(git(Path.cwd(), "rev-parse", "--show-toplevel").strip())
        if gh is None:
            exe = shutil.which("gh")
            if exe is None:
                raise NotesError("the GitHub CLI (gh) is not on PATH")
            gh = GhCli(exe)
        text = render(collect(root, args.repo, args.tag, args.head, gh), args.server)
    except NotesError as err:
        print(f"release_notes: {err}", file=sys.stderr)
        return 1
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text, encoding="utf-8", newline="\n")
    if (getattr(sys.stdout, "encoding", None) or "utf-8").lower().replace("-", "") != "utf8":
        sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[union-attr]  (a Windows console)
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
