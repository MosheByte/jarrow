"""Tests for the community workflows: welcome.yml, proposal-to-issue.yml and red-mod-intake.yml.

Each workflow runs one inline Python script (a `shell: python` step). These tests cut that
script out of the YAML, run it with a fake GitHub API and fake HTTP, and check what it would
post. They also hold the workflows to the repository's rules: actions pinned to full SHAs,
top-level permissions read-only, and no checkout in workflows that react to other people's
content. Standard library only, no network.
"""
from __future__ import annotations

import base64
import io
import json
import re
import textwrap
import zipfile
from pathlib import Path
from typing import Any, Callable

import pytest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
TEMPLATE = ROOT / ".github" / "ISSUE_TEMPLATE" / "red-mod-report.yml"
REPO = "MosheByte/jarrow"


def run_blocks(text: str) -> list[str]:
    """Every `run: |` block of a workflow, dedented."""
    lines = text.splitlines()
    blocks = []
    for i, line in enumerate(lines):
        m = re.match(r"^(\s*)run: \|\s*$", line)
        if not m:
            continue
        indent = len(m.group(1))
        body = []
        for nxt in lines[i + 1 :]:
            if nxt.strip() and len(nxt) - len(nxt.lstrip()) <= indent:
                break
            body.append(nxt)
        blocks.append(textwrap.dedent("\n".join(body)) + "\n")
    return blocks


def load(workflow: str, marker: str) -> dict[str, Any]:
    """The inline script of WORKFLOW whose first line names MARKER, executed as a module."""
    text = (WORKFLOWS / workflow).read_text(encoding="utf-8")
    scripts = [b for b in run_blocks(text) if b.lstrip().startswith(f"# {marker}")]
    assert len(scripts) == 1, f"{workflow}: expected one script marked {marker}"
    namespace: dict[str, Any] = {"__name__": "workflow_script"}
    exec(compile(scripts[0], f"{workflow}:{marker}", "exec"), namespace)
    return namespace


class FakeApi:
    """Answers api(path, method, body) from a table of (method, path prefix) -> response."""

    def __init__(self, routes: dict[tuple[str, str], Any]) -> None:
        self.routes = routes
        self.calls: list[tuple[str, str, Any]] = []

    def __call__(self, path: str, method: str = "GET", body: Any = None) -> Any:
        self.calls.append((method, path, body))
        best = None
        for (m, prefix), value in self.routes.items():
            if m == method and path.startswith(prefix) and (best is None or len(prefix) > len(best[0])):
                best = (prefix, value)
        if best is None:
            raise self.error(f"gh: Not Found (HTTP 404) {method} {path}")
        value = best[1]
        if isinstance(value, Exception):
            raise value
        return value(path, body) if callable(value) else value

    def all(self, path: str) -> list:
        result = self(path)
        return list(result or [])

    error: Callable[[str], Exception] = RuntimeError


def install(ns: dict[str, Any], routes: dict[tuple[str, str], Any]) -> FakeApi:
    fake = FakeApi(routes)
    fake.error = ns["GhError"]  # type: ignore[assignment]
    ns["api"] = fake
    ns["api_all"] = fake.all
    return fake


# ------------------------------------------------------------------ workflow rules


@pytest.mark.parametrize("name", ["welcome.yml", "proposal-to-issue.yml", "red-mod-intake.yml", "contributors.yml"])
def test_workflow_rules(name: str) -> None:
    text = (WORKFLOWS / name).read_text(encoding="utf-8")
    assert "\r" not in text
    assert re.search(r"^permissions:\n  contents: read\n", text, re.MULTILINE), "top-level permissions must be contents: read"
    for uses in re.findall(r"uses: (\S+)(.*)", text):
        ref, comment = uses
        assert re.fullmatch(r"[\w.-]+/[\w./-]+@[0-9a-f]{40}", ref), ref
        assert re.match(r"\s+# v\d", comment), f"{ref} needs a version comment"
    assert "pull_request_target" not in text
    if name != "contributors.yml":
        assert "actions/checkout" not in text, f"{name} must not check out code"
    assert "guaranteed" not in text.lower()


def on_block(text: str) -> str:
    """The workflow's `on:` block, up to the next top-level key."""
    m = re.search(r"^on:\n((?:[ #].*\n|\n)*)", text, re.MULTILINE)
    assert m, "the workflow has an on: block"
    return m.group(1)


@pytest.mark.parametrize("name", ["ci.yml", "harness.yml"])
def test_bot_pull_requests_can_start_ci_by_dispatch(name: str) -> None:
    # tools/bot_pr.sh pushes with the workflow token, which starts no push or pull_request
    # run, so it starts these two with `gh workflow run`.
    text = (WORKFLOWS / name).read_text(encoding="utf-8")
    assert re.search(r"^  workflow_dispatch:", on_block(text), re.MULTILINE), name
    assert '"bot/**"' not in text


def bot_pr_jobs(text: str) -> list[str]:
    """The jobs (as text) that call tools/bot_pr.sh."""
    jobs = re.split(r"^  (?=[a-z][\w-]*:\n)", text.split("\njobs:\n", 1)[1], flags=re.MULTILINE)
    return [job for job in jobs if "tools/bot_pr.sh" in job]


# board.yml switches to tools/bot_pr.sh in the follow-up after PR #16 (it rewrites board.yml).
@pytest.mark.parametrize("name", ["contributors.yml"])
def test_bot_commits_go_through_bot_pr_with_the_workflow_token(name: str) -> None:
    text = (WORKFLOWS / name).read_text(encoding="utf-8")
    jobs = bot_pr_jobs(text)
    assert len(jobs) == 1, f"{name}: one job calls tools/bot_pr.sh"
    job = jobs[0]
    assert "if: github.ref == 'refs/heads/main'" in job
    for permission in ("contents: write", "pull-requests: write", "actions: write"):
        assert permission in job, f"{name}: the bot_pr job needs {permission}"
    assert "GH_TOKEN: ${{ github.token }}" in job
    # The deploy key and direct pushes are gone; GITHUB_TOKEN pushes the bot/ branch.
    assert "BOT_DEPLOY_KEY" not in text and "GIT_SSH_COMMAND" not in text
    assert "HEAD:main" not in text and "git push" not in text
    assert "persist-credentials: false" in job


def test_contributors_workflow_opens_a_pull_request_instead_of_pushing() -> None:
    text = (WORKFLOWS / "contributors.yml").read_text(encoding="utf-8")
    assert "bash tools/bot_pr.sh contributors" in text
    assert "python tools/contributors.py --ledger" in text and "python tools/ledger.py" in text
    for path in ("ledger/inputs/contributors.json", "LEDGER.md", "site/src/data/team.json"):
        assert path in text


def test_deps_automerge_only_enables_auto_merge_for_the_dependency_bots() -> None:
    text = (WORKFLOWS / "deps-automerge.yml").read_text(encoding="utf-8")
    assert "\r" not in text
    assert re.search(r"^permissions:\n  contents: read\n", text, re.MULTILINE)
    assert re.search(r"^  pull_request_target:\n    types: \[opened, reopened, synchronize\]\n", on_block(text), re.M)
    # pull_request_target runs with a write token: never check out, build or run PR code.
    assert "actions/checkout" not in text and "uses:" not in text
    assert "github.event.pull_request.head.sha" not in text and "head.ref" not in text
    assert "github.event.pull_request.user.login == 'dependabot[bot]'" in text
    assert "github.event.pull_request.user.login == 'renovate[bot]'" in text
    assert "github.event.pull_request.head.repo.full_name == github.repository" in text
    for permission in ("contents: write", "pull-requests: write"):
        assert permission in text
    commands = [line.strip() for block in run_blocks(text) for line in block.splitlines()]
    gh_calls = [c for c in commands if c.startswith("gh ") or "$(gh " in c]
    assert len(gh_calls) == 2
    assert 'gh pr merge "$PR" --repo "$REPO" --auto' in gh_calls
    # Untrusted values reach the script through env only, never through ${{ }} in run.
    for block in run_blocks(text):
        assert "${{" not in block


def test_renovate_leaves_dependabot_ecosystems_alone() -> None:
    config = json.loads((ROOT / "renovate.json").read_text(encoding="utf-8"))
    managers = set(config["enabledManagers"])
    # Dependabot keeps these (.github/dependabot.yml); Renovate must not open the same updates.
    assert not managers & {"cargo", "npm", "github-actions", "pip_requirements", "pip-compile", "pep621"}
    assert {"dockerfile", "gradle", "rust-toolchain", "custom.regex"} <= managers
    dependabot = (ROOT / ".github" / "dependabot.yml").read_text(encoding="utf-8")
    assert set(re.findall(r"package-ecosystem: ([\w-]+)", dependabot)) == {"cargo", "npm", "github-actions", "pip"}
    assert config["automerge"] is True and config["platformAutomerge"] is True
    # Of the Dockerfiles, only the devcontainer's: the ClusterFuzzLite builder is switched off.
    off = [r for r in config["packageRules"] if r.get("enabled") is False]
    assert len(off) == 1 and off[0]["matchManagers"] == ["dockerfile"]
    assert off[0]["matchFileNames"] == [".clusterfuzzlite/**"]
    rules = {rule.get("matchDepNames", [""])[0]: rule for rule in config["packageRules"]}
    assert rules["typescript"]["allowedVersions"] == "<7"
    assert rules["net.fabricmc.fabric-loom-remap"]["allowedVersions"] == "<1.18"


def renovate_matches(manager: dict, path: str) -> list[dict[str, str]]:
    """What one regex custom manager extracts from a repository file, like Renovate does."""
    text = (ROOT / path).read_text(encoding="utf-8")
    found = []
    for pattern in manager["matchStrings"]:
        # Renovate's matchStrings are JavaScript regexes; apart from the named groups, these use
        # only syntax Python shares.
        for m in re.finditer(pattern.replace("(?<", "(?P<"), text):
            found.append(m.groupdict())
    return found


@pytest.mark.parametrize(
    "path, dep, value",
    [
        ("Cargo.toml", "rust", None),
        (".github/workflows/harness.yml", "gradle", None),
        (".github/workflows/board.yml", "wrangler", "4.157.0"),
        (".github/workflows/infra.yml", "wrangler", "4.157.0"),
    ],
)
def test_renovate_regex_managers_find_their_pins(path: str, dep: str, value: str | None) -> None:
    config = json.loads((ROOT / "renovate.json").read_text(encoding="utf-8"))
    hits = []
    for manager in config["customManagers"]:
        patterns = [p.strip("/") for p in manager["managerFilePatterns"]]
        if not any(re.search(p, path) for p in patterns):
            continue
        for groups in renovate_matches(manager, path):
            name = groups.get("depName") or manager.get("depNameTemplate")
            if name == dep:
                hits.append(groups["currentValue"])
    assert len(hits) == 1, (path, dep, hits)
    if value is not None:
        assert hits == [value]


# ------------------------------------------------------------------ welcome.yml

SHA = "a" * 40


def pr(association: str = "FIRST_TIME_CONTRIBUTOR", login: str = "newbie", kind: str = "User") -> dict:
    return {
        "number": 17,
        "state": "open",
        "head": {"sha": SHA},
        "user": {"login": login, "type": kind},
        "author_association": association,
    }


def summary_zip(text: str) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr("summary.md", text)
    return buffer.getvalue()


def welcome_routes(overrides: dict | None = None) -> dict[tuple[str, str], Any]:
    routes: dict[tuple[str, str], Any] = {
        ("GET", f"repos/{REPO}/pulls/17"): pr(),
        ("GET", "search/issues"): {"total_count": 0},
        ("GET", f"repos/{REPO}/actions/runs?"): {
            "workflow_runs": [
                {"id": 100, "name": "ci", "status": "completed", "conclusion": "failure", "run_attempt": 1, "html_url": "https://github.com/r/100"},
                {"id": 90, "name": "ci", "status": "completed", "conclusion": "success", "run_attempt": 1, "html_url": "https://github.com/r/90"},
                {"id": 101, "name": "harness", "status": "completed", "conclusion": "success", "run_attempt": 2, "html_url": "https://github.com/r/101"},
                {"id": 102, "name": "codeql", "status": "completed", "conclusion": "success", "run_attempt": 1},
            ]
        },
        ("GET", f"repos/{REPO}/actions/runs/100/attempts/1/jobs"): {
            "jobs": [
                {"id": 1, "name": "rust (ubuntu-latest)", "status": "completed", "conclusion": "failure", "html_url": "https://github.com/j/1"},
                {"id": 2, "name": "fmt", "status": "completed", "conclusion": "success", "html_url": "https://github.com/j/2"},
            ]
        },
        ("GET", f"repos/{REPO}/actions/runs/101/attempts/2/jobs"): {
            "jobs": [{"id": 3, "name": "smoke | 25 mods", "status": "completed", "conclusion": "success", "html_url": "https://github.com/j/3"}]
        },
        ("GET", f"repos/{REPO}/actions/runs/101/artifacts"): {"artifacts": [{"id": 55, "name": "harness-summary", "expired": False}]},
        ("GET", f"repos/{REPO}/issues?state=open&sort=created&direction=asc&per_page=50&labels=good-first-rule"): [
            {"number": 4, "title": "PR, not an issue", "html_url": "https://x/4", "pull_request": {}},
            {"number": 5, "title": "ServerTickEvents.END_SERVER_TICK", "html_url": "https://x/5", "assignees": [{"login": "someone"}]},
            {"number": 6, "title": "fabric.mod.json: accessWidener", "html_url": "https://x/6", "assignees": []},
        ],
        ("GET", f"repos/{REPO}/issues/17/comments"): [],
        ("POST", f"repos/{REPO}/issues/17/comments"): {"id": 900},
    }
    routes.update(overrides or {})
    return routes


ENV = {"REPO": REPO, "HEAD_SHA": SHA, "PR_NUMBERS": "17", "HEAD_OWNER": "newbie", "HEAD_BRANCH": "patch-1"}


@pytest.fixture
def welcome() -> dict[str, Any]:
    ns = load("welcome.yml", "jarrow:welcome")
    ns["download"] = lambda path: summary_zip("Score 0.75 on 25 mods. cc @maintainer <!-- jarrow:welcome -->")
    return ns


def posted(fake: FakeApi) -> list[tuple[str, str, Any]]:
    return [c for c in fake.calls if c[0] in ("POST", "PATCH")]


def test_welcome_posts_one_comment_with_checks_harness_and_next_rule(welcome: dict[str, Any]) -> None:
    fake = install(welcome, welcome_routes())
    assert welcome["main"](ENV) == 0
    [(method, path, body)] = posted(fake)
    assert (method, path) == ("POST", f"repos/{REPO}/issues/17/comments")
    text = body["body"]
    lines = text.splitlines()
    assert lines[0] == "<!-- jarrow:welcome -->"
    assert lines[1] == "@newbie, this is your first pull request here. What the checks found on `aaaaaaa`:"
    # The newest ci run (100), each job with its result and link; codeql is not watched.
    assert "**ci** ([run 100](https://github.com/r/100)): failure" in text
    assert "| [fmt](https://github.com/j/2) | success |" in text
    assert "| [rust (ubuntu-latest)](https://github.com/j/1) | failure |" in text
    assert "run 90" not in text and "codeql" not in text
    assert "| [smoke \\| 25 mods](https://github.com/j/3) | success |" in text
    # The harness summary is quoted, with mentions defused and the marker neutralised.
    assert "<details><summary>Harness summary</summary>" in text
    assert "Score 0.75 on 25 mods. cc @​maintainer" in text
    assert text.count("<!-- jarrow:welcome -->") == 1
    # The oldest unassigned good-first-rule issue; PRs and assigned issues are skipped.
    assert lines[-1] == "Next rule you could take: [#6 fabric.mod.json: accessWidener](https://x/6) (`good-first-rule`)."


def test_welcome_edits_its_comment_instead_of_posting_twice(welcome: dict[str, Any]) -> None:
    existing = [
        {"id": 1, "body": "<!-- jarrow:welcome --> forged by a person", "user": {"login": "newbie"}},
        {"id": 2, "body": "<!-- jarrow:welcome -->\nold", "user": {"login": "github-actions[bot]"}},
    ]
    fake = install(
        welcome,
        welcome_routes(
            {
                ("GET", f"repos/{REPO}/issues/17/comments"): existing,
                ("PATCH", f"repos/{REPO}/issues/comments/2"): {"id": 2},
            }
        ),
    )
    welcome["main"](ENV)
    [(method, path, _)] = posted(fake)
    assert (method, path) == ("PATCH", f"repos/{REPO}/issues/comments/2")

    # Same body again: nothing is written.
    body = posted(fake)[0][2]["body"]
    existing[1]["body"] = body
    fake.calls.clear()
    welcome["main"](ENV)
    assert posted(fake) == []


def test_welcome_without_harness_run_or_open_rules(welcome: dict[str, Any]) -> None:
    routes = welcome_routes(
        {
            ("GET", f"repos/{REPO}/actions/runs?"): {
                "workflow_runs": [{"id": 100, "name": "ci", "status": "completed", "conclusion": "success", "run_attempt": 1, "html_url": "https://github.com/r/100"}]
            },
            ("GET", f"repos/{REPO}/issues?"): [],
        }
    )
    routes.pop(("GET", f"repos/{REPO}/issues?state=open&sort=created&direction=asc&per_page=50&labels=good-first-rule"))
    fake = install(welcome, routes)
    welcome["main"](ENV)
    text = posted(fake)[0][2]["body"]
    assert "**harness**: the harness did not run on this PR." in text
    assert text.rstrip().endswith("Next rule you could take: no open good-first-rule issue right now.")


def test_welcome_falls_back_to_good_first_issue(welcome: dict[str, Any]) -> None:
    routes = welcome_routes(
        {
            ("GET", f"repos/{REPO}/issues?state=open&sort=created&direction=asc&per_page=50&labels=good-first-rule"): [],
            ("GET", f"repos/{REPO}/issues?state=open&sort=created&direction=asc&per_page=50&labels=good%20first%20issue"): [
                {"number": 8, "title": "Docs typo", "html_url": "https://x/8"}
            ],
        }
    )
    fake = install(welcome, routes)
    welcome["main"](ENV)
    assert posted(fake)[0][2]["body"].rstrip().endswith("[#8 Docs typo](https://x/8) (`good first issue`).")


def test_welcome_says_when_the_harness_is_still_running(welcome: dict[str, Any]) -> None:
    runs = welcome_routes()[("GET", f"repos/{REPO}/actions/runs?")]
    runs["workflow_runs"][2].update(status="in_progress", conclusion=None)
    fake = install(welcome, welcome_routes({("GET", f"repos/{REPO}/actions/runs?"): runs}))
    welcome["main"](ENV)
    text = posted(fake)[0][2]["body"]
    assert "**harness** ([run 101](https://github.com/r/101)): in progress" in text
    assert "The harness is still running" in text and "Harness summary" not in text


def test_welcome_quotes_the_baseline_status_files(welcome: dict[str, Any]) -> None:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for version in ("1.21.11", "1.21.1"):
            status = {"game_version": version, "status": "not-run", "reason": "eula_not_accepted",
                      "detail": "not run: EULA not accepted | gate closed", "exit_code": None, "schema": 1}
            archive.writestr(f"{version}/baseline-status.json", json.dumps(status))
        archive.writestr("1.21.1/server.log", "x" * 100)
    welcome["download"] = lambda path: buffer.getvalue()
    fake = install(
        welcome,
        welcome_routes(
            {("GET", f"repos/{REPO}/actions/runs/101/artifacts"): {"artifacts": [{"id": 56, "name": "baseline", "size_in_bytes": 900}]}}
        ),
    )
    welcome["main"](ENV)
    text = posted(fake)[0][2]["body"]
    assert "Baseline per game version (`baseline` artifact):" in text
    rows = [line for line in text.splitlines() if line.startswith("| 1.21.")]
    assert rows == [
        "| 1.21.1 | not-run | eula_not_accepted | not run: EULA not accepted \\| gate closed |",
        "| 1.21.11 | not-run | eula_not_accepted | not run: EULA not accepted \\| gate closed |",
    ]


def test_welcome_without_any_harness_artifact(welcome: dict[str, Any]) -> None:
    fake = install(welcome, welcome_routes({("GET", f"repos/{REPO}/actions/runs/101/artifacts"): {"artifacts": []}}))
    welcome["main"](ENV)
    assert "The harness run left no summary" in posted(fake)[0][2]["body"]


@pytest.mark.parametrize(
    ("the_pr", "merged", "comments"),
    [
        (pr("FIRST_TIMER"), 5, True),
        (pr("CONTRIBUTOR"), 0, True),  # no merged PR by the author yet
        (pr("CONTRIBUTOR"), 2, False),
        (pr("OWNER", login="MosheByte"), 9, False),
        (pr("FIRST_TIME_CONTRIBUTOR", login="dependabot[bot]", kind="Bot"), 0, False),
    ],
)
def test_welcome_only_for_first_time_contributors(
    welcome: dict[str, Any], the_pr: dict, merged: int, comments: bool
) -> None:
    fake = install(
        welcome,
        welcome_routes({("GET", f"repos/{REPO}/pulls/17"): the_pr, ("GET", "search/issues"): {"total_count": merged}}),
    )
    welcome["main"](ENV)
    assert bool(posted(fake)) is comments


def test_welcome_finds_a_fork_pull_request_by_head_branch(welcome: dict[str, Any]) -> None:
    fork_routes = welcome_routes({("GET", f"repos/{REPO}/pulls?state=open"): [pr()]})
    del fork_routes[("GET", f"repos/{REPO}/pulls/17")]
    fake = install(welcome, fork_routes)
    welcome["main"]({**ENV, "PR_NUMBERS": "", "HEAD_BRANCH": "my branch"})
    lookups = [c[1] for c in fake.calls if "pulls?state=open" in c[1]]
    assert lookups == [f"repos/{REPO}/pulls?state=open&per_page=10&head=newbie:my%20branch"]
    assert posted(fake)


def test_welcome_ignores_a_run_for_an_older_head(welcome: dict[str, Any]) -> None:
    moved = pr()
    moved["head"] = {"sha": "b" * 40}
    fake = install(welcome, welcome_routes({("GET", f"repos/{REPO}/pulls/17"): moved}))
    welcome["main"](ENV)
    assert posted(fake) == []


# ------------------------------------------------------------------ proposal-to-issue.yml


def discussion(**overrides: Any) -> dict:
    data = {
        "id": "D_kwDO1",
        "number": 12,
        "title": "ServerTickEvents.END_SERVER_TICK to ServerTickEvent.Post",
        "body": "Fabric symbol: ...\n\nSources: ...",
        "url": "https://github.com/MosheByte/jarrow/discussions/12",
        "author": {"login": "proposer"},
        "category": {"name": "Rule proposals", "slug": "rule-proposals"},
        "early": {"nodes": []},
        "recent": {"nodes": []},
    }
    data.update(overrides)
    return data


def proposal_routes(found: dict, issues: list) -> dict[tuple[str, str], Any]:
    def graphql(path: str, body: dict) -> dict:
        if "addDiscussionComment" in body["query"]:
            return {"data": {"addDiscussionComment": {"comment": {"url": "https://x/c"}}}}
        assert body["variables"] == {"owner": "MosheByte", "name": "jarrow", "number": 12}
        return {"data": {"repository": {"discussion": found}}}

    return {
        ("POST", "graphql"): graphql,
        ("GET", f"repos/{REPO}/issues?state=all&per_page=100&labels=rule"): issues,
        ("POST", f"repos/{REPO}/issues"): {"number": 40, "html_url": "https://github.com/MosheByte/jarrow/issues/40"},
    }


ISSUE_ENV = {"REPO": REPO, "DISCUSSION_NUMBER": "12"}


def mutations(fake: FakeApi) -> list[dict]:
    return [c[2] for c in fake.calls if c[1] == "graphql" and "addDiscussionComment" in c[2]["query"]]


def test_proposal_becomes_an_issue_and_the_discussion_links_it() -> None:
    ns = load("proposal-to-issue.yml", "jarrow:proposal-to-issue")
    fake = install(ns, proposal_routes(discussion(), []))
    assert ns["main"](ISSUE_ENV) == 0
    [(_, _, issue)] = [c for c in fake.calls if c[:2] == ("POST", f"repos/{REPO}/issues")]
    assert issue["title"] == "ServerTickEvents.END_SERVER_TICK to ServerTickEvent.Post"
    assert issue["labels"] == ["rule"]
    assert issue["body"].startswith("<!-- jarrow:proposal-discussion=12 -->\n")
    assert "discussion #12 by @proposer: https://github.com/MosheByte/jarrow/discussions/12" in issue["body"]
    assert issue["body"].rstrip().endswith("Sources: ...")
    [comment] = mutations(fake)
    assert comment["variables"]["discussion"] == "D_kwDO1"
    assert "issue #40: https://github.com/MosheByte/jarrow/issues/40" in comment["variables"]["body"]


def test_proposal_to_issue_is_idempotent() -> None:
    ns = load("proposal-to-issue.yml", "jarrow:proposal-to-issue")
    existing = [{"number": 40, "html_url": "https://x/40", "body": "<!-- jarrow:proposal-discussion=12 -->\n..."}]
    linked = discussion(recent={"nodes": [{"body": "<!-- jarrow:proposal-issue -->\nAccepted.", "author": {"login": "github-actions"}}]})
    fake = install(ns, proposal_routes(linked, existing))
    assert ns["main"](ISSUE_ENV) == 0
    assert not [c for c in fake.calls if c[:2] == ("POST", f"repos/{REPO}/issues")]
    assert mutations(fake) == []

    # The issue exists but the comment is missing (a rerun after a failure): only the comment.
    fake = install(ns, proposal_routes(discussion(), existing))
    ns["main"](ISSUE_ENV)
    assert not [c for c in fake.calls if c[:2] == ("POST", f"repos/{REPO}/issues")]
    assert len(mutations(fake)) == 1

    # Another discussion's marker does not count, nor does a person quoting the marker.
    other = [{"number": 41, "html_url": "https://x/41", "body": "<!-- jarrow:proposal-discussion=120 -->"}]
    quoted = discussion(early={"nodes": [{"body": "<!-- jarrow:proposal-issue -->", "author": {"login": "proposer"}}]})
    fake = install(ns, proposal_routes(quoted, other))
    ns["main"](ISSUE_ENV)
    assert [c for c in fake.calls if c[:2] == ("POST", f"repos/{REPO}/issues")]
    assert len(mutations(fake)) == 1


def test_proposal_outside_rule_proposals_is_left_alone() -> None:
    ns = load("proposal-to-issue.yml", "jarrow:proposal-to-issue")
    fake = install(ns, proposal_routes(discussion(category={"name": "Ideas", "slug": "ideas"}), []))
    assert ns["main"](ISSUE_ENV) == 0
    assert [c for c in fake.calls if c[0] == "POST" and c[1] != "graphql"] == []
    assert mutations(fake) == []


def test_proposal_workflow_only_fires_for_accepted_rule_proposals() -> None:
    text = (WORKFLOWS / "proposal-to-issue.yml").read_text(encoding="utf-8")
    assert "github.event.label.name == 'accepted'" in text
    assert "github.event.discussion.category.slug == 'rule-proposals'" in text
    assert "discussions: write" in text and "issues: write" in text


# ------------------------------------------------------------------ red-mod-intake.yml

PROJECT = {
    "id": "AANobbMI",
    "slug": "sodium",
    "title": "Sodium",
    "project_type": "mod",
    "license": {"id": "LicenseRef-Polyform-Shield-1.0.0", "name": "", "url": "https://example.org/LICENSE.md"},
    "client_side": "required",
    "server_side": "unsupported",
    "loaders": ["fabric", "neoforge", "quilt"],
    "game_versions": ["1.20.1", "1.21", "1.21.1", "1.21.11"],
    "downloads": 259127085,
}


def version(number: str, gvs: list[str], loaders: list[str], day: str, kind: str = "release") -> dict:
    return {
        "version_number": number,
        "game_versions": gvs,
        "loaders": loaders,
        "date_published": f"{day}T10:00:00Z",
        "version_type": kind,
        "files": [
            {"filename": f"{number}-sources.jar", "url": "https://cdn.modrinth.com/s.jar", "primary": False},
            {"filename": f"{number}.jar", "url": f"https://cdn.modrinth.com/{number}.jar", "primary": True},
        ],
    }


VERSIONS = [
    version("mc1.20.1-0.5.11-fabric", ["1.20.1"], ["fabric", "quilt"], "2024-06-01"),
    version("mc1.20.1-0.5.13-fabric", ["1.20.1"], ["fabric", "quilt"], "2024-09-01"),
    version("mc1.21.1-0.6.0-fabric", ["1.21", "1.21.1"], ["fabric"], "2025-01-10"),
    version("mc1.21.1-0.6.0-neoforge", ["1.21.1"], ["neoforge"], "2025-01-11", "beta"),
]


@pytest.fixture
def intake() -> dict[str, Any]:
    return load("red-mod-intake.yml", "jarrow:red-mod-intake")


def form_body(label: str, value: str) -> str:
    return f"### {label}\n\n{value}\n\n### Report hash\n\nsha256:abc\n\n### Log excerpt\n\n_No response_\n"


def test_form_labels_read_the_real_template(intake: dict[str, Any]) -> None:
    labels = intake["form_labels"](TEMPLATE.read_text(encoding="utf-8"))
    assert any(labels.get(i) for i in ("mod-url", "source-mod-url")), labels


def test_form_labels_take_each_field_s_own_label(intake: dict[str, Any]) -> None:
    template = "\n".join(
        [
            "body:",
            "  - type: markdown",
            "    attributes:",
            "      value: |",
            "        Read this first.",
            "  - type: input",
            "    id: source-mod-url",
            "    attributes:",
            '      label: "Source mod (Modrinth URL)"',
            "  - type: checkboxes",
            "    id: confirmations",
            "    attributes:",
            "      label: Before you submit",
            "      options:",
            "        - label: I did not attach a JAR.",
        ]
    )
    assert intake["form_labels"](template) == {
        "source-mod-url": "Source mod (Modrinth URL)",
        "confirmations": "Before you submit",
    }


def test_form_url_by_field_id_label_and_fallbacks(intake: dict[str, Any]) -> None:
    form_url = intake["form_url"]
    renamed = {"source-mod-url": "Source mod (Modrinth URL)"}
    body = form_body("Source mod (Modrinth URL)", "https://modrinth.com/mod/sodium")
    assert form_url(body, renamed) == "https://modrinth.com/mod/sodium"
    assert form_url(form_body("Modrinth mod URL", "https://modrinth.com/mod/lithium"), {}) == "https://modrinth.com/mod/lithium"
    # Not a form at all: the first Modrinth link anywhere in the body.
    assert form_url("It breaks: see https://modrinth.com/mod/iris/version/1.8.0 please", {}) == (
        "https://modrinth.com/mod/iris/version/1.8.0"
    )
    assert form_url(form_body("Modrinth mod URL", "_No response_"), {}) == ""


@pytest.mark.parametrize(
    ("value", "slug"),
    [
        ("https://modrinth.com/mod/sodium", "sodium"),
        ("https://www.modrinth.com/mod/sodium/versions?l=fabric", "sodium"),
        ("<https://modrinth.com/project/AANobbMI>", "AANobbMI"),
        ("modrinth.com/plugin/luckperms", "luckperms"),
        ("https://modrinth.com/user/jellysquid", None),
        ("https://www.curseforge.com/minecraft/mc-mods/jei", None),
        ("https://modrinth.com.evil.example/mod/sodium", None),
        ("https://modrinth.com/mod/so`dium", None),
        ("not a link", None),
    ],
)
def test_modrinth_slug(intake: dict[str, Any], value: str, slug: str | None) -> None:
    assert intake["modrinth_slug"](value) == slug


def intake_routes(body: str, comments: list | None = None) -> dict[tuple[str, str], Any]:
    template = base64.b64encode(TEMPLATE.read_bytes()).decode("ascii")
    return {
        ("GET", f"repos/{REPO}/issues/9"): {"number": 9, "body": body},
        ("GET", f"repos/{REPO}/contents/.github/ISSUE_TEMPLATE/red-mod-report.yml"): {"content": template},
        ("GET", f"repos/{REPO}/issues/9/comments"): comments or [],
        ("POST", f"repos/{REPO}/issues/9/comments"): {"id": 1},
        ("PATCH", f"repos/{REPO}/issues/comments/"): {"id": 1},
    }


INTAKE_ENV = {"REPO": REPO, "ISSUE_NUMBER": "9", "DEFAULT_BRANCH": "main", "SITE_URL": "https://jarrow.pages.dev"}


def fake_http(board: Any) -> Callable[[str], Any]:
    def fetch(url: str) -> Any:
        if url == "https://api.modrinth.com/v2/project/sodium":
            return PROJECT
        if url == "https://api.modrinth.com/v2/project/AANobbMI/version":
            return VERSIONS
        if url == "https://jarrow.pages.dev/api/board.json":
            if isinstance(board, Exception):
                raise board
            return board
        return None

    return fetch


def comment_of(fake: FakeApi) -> str:
    [(method, _, body)] = [c for c in fake.calls if c[0] in ("POST", "PATCH")]
    return body["body"]


def the_body(intake: dict[str, Any]) -> str:
    """An issue as GitHub renders the real form, with the Sodium page in the URL field."""
    labels = intake["form_labels"](TEMPLATE.read_text(encoding="utf-8"))
    label = labels.get("source-mod-url") or labels["mod-url"]
    return form_body(label, "https://modrinth.com/mod/sodium")


def test_red_mod_intake_comment_has_modrinth_and_board_data(intake: dict[str, Any]) -> None:
    board = {
        "generated_at": "2026-11-04T04:12:00Z",
        "rows": [
            {"project_id": "AANobbMI", "slug": "sodium", "title": "Sodium", "outcome": "red", "tier": "b",
             "reason": "No converter yet (milestone M2)", "primary_game_version": "1.21.11", "last_run": "2026-11-04T03:55:00Z",
             "report_url": None}
        ],
    }
    intake["fetch_json"] = fake_http(board)
    fake = install(intake, intake_routes(the_body(intake)))
    assert intake["main"](INTAKE_ENV) == 0
    text = comment_of(fake)
    assert text.startswith("<!-- jarrow:red-mod-intake -->\n")
    assert "[Sodium](https://modrinth.com/mod/sodium)" in text
    assert "| Mod id | `AANobbMI` (slug `sodium`) |" in text
    assert "| License | [LicenseRef-Polyform-Shield-1.0.0](https://example.org/LICENSE.md) |" in text
    assert "| Client side / server side | required / unsupported |" in text
    assert "| Loaders | fabric, neoforge, quilt |" in text
    assert "| Game versions | 4 listed; pinned: 1.20.1, 1.21.1, 1.21.11 |" in text
    assert "| Downloads | 259,127,085 |" in text
    assert "| NeoForge build on Modrinth | yes, for 1.21.1 |" in text
    # Newest file per pinned version: the primary file, the newest by date.
    assert "| 1.20.1 | [mc1.20.1-0.5.13-fabric.jar](https://cdn.modrinth.com/mc1.20.1-0.5.13-fabric.jar)" in text
    assert "| 1.21.1 | [mc1.21.1-0.6.0-neoforge.jar]" in text and "`mc1.21.1-0.6.0-neoforge`, beta" in text
    assert "| 1.21.11 | none | | | no |" in text
    assert "0.5.11" not in text
    # The board row, with a link to it.
    assert "[the row for Sodium](https://jarrow.pages.dev/board/#mod-AANobbMI)" in text
    assert "| Result | red |" in text and "| Reason | No converter yet (milestone M2) |" in text


def test_red_mod_intake_board_states(intake: dict[str, Any]) -> None:
    for board, expected in (
        ({"generated_at": None, "rows": []}, "Board: no rows yet"),
        (None, "Board: no rows yet"),
        ({"generated_at": "2026-11-04T04:12:00Z", "rows": [{"mod_id": "other", "name": "Other"}]}, "Sodium is not on [the board]"),
        ({"rows": [{"mod_id": "AANobbMI", "name": "Sodium", "result": "green", "tier": "a"}]}, "| Result | green |"),
        (intake["FetchError"]("HTTP 503"), "could not be read (HTTP 503)"),
    ):
        intake["fetch_json"] = fake_http(board)
        fake = install(intake, intake_routes(the_body(intake)))
        intake["main"](INTAKE_ENV)
        assert expected in comment_of(fake), expected


def test_red_mod_intake_explains_a_bad_url(intake: dict[str, Any]) -> None:
    intake["fetch_json"] = fake_http(None)
    for body, expected in (
        (form_body("Modrinth mod URL", "https://www.curseforge.com/minecraft/mc-mods/jei"), "is not a Modrinth project URL"),
        ("Nothing useful here.", "The form has no Modrinth URL"),
        (form_body("Modrinth mod URL", "https://modrinth.com/mod/does-not-exist"), "Modrinth has no project `does-not-exist`"),
    ):
        fake = install(intake, intake_routes(body))
        intake["main"](INTAKE_ENV)
        assert expected in comment_of(fake), expected


def test_red_mod_intake_updates_its_one_comment(intake: dict[str, Any]) -> None:
    intake["fetch_json"] = fake_http(None)
    mine = {"id": 77, "body": "<!-- jarrow:red-mod-intake -->\nold", "user": {"login": "github-actions[bot]"}}
    fake = install(intake, intake_routes(the_body(intake), [mine]))
    intake["main"](INTAKE_ENV)
    [(method, path, _)] = [c for c in fake.calls if c[0] in ("POST", "PATCH")]
    assert (method, path) == ("PATCH", f"repos/{REPO}/issues/comments/77")


def test_red_mod_intake_triggers_and_permissions() -> None:
    text = (WORKFLOWS / "red-mod-intake.yml").read_text(encoding="utf-8")
    assert "types: [opened, edited, labeled]" in text
    assert "contains(github.event.issue.labels.*.name, 'red-mod')" in text
    assert "issues: write" in text and "pull-requests" not in text
    intake_ns = load("red-mod-intake.yml", "jarrow:red-mod-intake")
    assert intake_ns["PINNED"] == ("1.20.1", "1.21.1", "1.21.11")
    assert "User-Agent" in (WORKFLOWS / "red-mod-intake.yml").read_text(encoding="utf-8")


# ------------------------------------------------------------------ security review 2026-11-03


def test_red_mod_intake_urls_cannot_break_out_of_their_link(intake: dict[str, Any]) -> None:
    # F-07: a project author sets the license URL on Modrinth. A ")" or a space would end the
    # Markdown link early and let the rest render, for example as a mention the bot sends.
    hostile = dict(PROJECT, license={"id": "MIT", "name": "", "url": "https://example.org/x) @everyone [y](https://evil.example"})
    intake["fetch_json"] = lambda url: hostile if url.endswith("/project/sodium") else (VERSIONS if url.endswith("/version") else None)
    fake = install(intake, intake_routes(the_body(intake)))
    intake["main"](INTAKE_ENV)
    text = comment_of(fake)
    [license_row] = [line for line in text.splitlines() if line.startswith("| License |")]
    assert license_row == "| License | [MIT](https://example.org/x%29%20%40everyone%20%5By%5D%28https://evil.example) |"
    assert "@everyone" not in text
    assert intake["link_url"]("http://example.org/") is None
    assert intake["link_url"](None) is None
    assert intake["link_url"]("https://cdn.modrinth.com/data/AANobbMI/versions/x/a+b.jar?x=1#f") == (
        "https://cdn.modrinth.com/data/AANobbMI/versions/x/a+b.jar?x=1#f"
    )


def test_proposal_body_cannot_carry_markers_or_mentions() -> None:
    # F-06: the proposer writes the body. Copied as is, another discussion's marker would make
    # that proposal look as if it already had its issue, and mentions would be pinged by the bot.
    ns = load("proposal-to-issue.yml", "jarrow:proposal-to-issue")
    body = "<!-- jarrow:proposal-discussion=13 -->\nPing @maintainer and @org/team.\n<!-- jarrow:proposal-issue -->"
    fake = install(ns, proposal_routes(discussion(body=body), []))
    assert ns["main"](ISSUE_ENV) == 0
    [(_, _, issue)] = [c for c in fake.calls if c[:2] == ("POST", f"repos/{REPO}/issues")]
    assert issue["body"].startswith("<!-- jarrow:proposal-discussion=12 -->\n")
    assert issue["body"].count("<!--") == 1
    assert "<!-- jarrow:proposal-discussion=13 -->" not in issue["body"]
    assert "@maintainer" not in issue["body"] and "@​maintainer" in issue["body"]
    assert "by @proposer" in issue["body"]


def unreadable_zip(name: str) -> bytes:
    """A zip whose one entry claims compression method 97 (WavPack), which zipfile does not
    implement: opening the entry raises NotImplementedError."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr(name, b'{"game_version": "1.21.1", "status": "ran"}')
    data = bytearray(buffer.getvalue())
    local = data.index(b"PK\x03\x04")
    central = data.index(b"PK\x01\x02")
    data[local + 8 : local + 10] = (97).to_bytes(2, "little")
    data[central + 10 : central + 12] = (97).to_bytes(2, "little")
    with zipfile.ZipFile(io.BytesIO(bytes(data))) as check, pytest.raises(NotImplementedError):
        check.read(name)
    return bytes(data)


@pytest.mark.parametrize("artifact", ["harness-summary", "baseline"])
def test_welcome_survives_an_unreadable_artifact(welcome: dict[str, Any], artifact: str) -> None:
    # F-09: the artifact comes from a pull request's run. An unknown compression method raises
    # NotImplementedError and an encrypted entry RuntimeError; neither may fail the job.
    name = "summary.md" if artifact == "harness-summary" else "1.21.1/baseline-status.json"
    welcome["download"] = lambda path: unreadable_zip(name)
    fake = install(
        welcome,
        welcome_routes({("GET", f"repos/{REPO}/actions/runs/101/artifacts"): {"artifacts": [{"id": 57, "name": artifact, "size_in_bytes": 300}]}}),
    )
    assert welcome["main"](ENV) == 0
    [(method, _, body)] = posted(fake)
    assert method == "POST" and body["body"].startswith("<!-- jarrow:welcome -->")
    assert "Baseline per game version" not in body["body"]


def test_secret_jobs_skip_pull_requests_from_bots() -> None:
    # F-01: a same-repository pull request from any bot (Dependabot, Renovate, another App) runs
    # `npm ci` or a build with the bot's dependency changes; no step after that may hold a secret.
    # Pushes to main come from github-merge-queue[bot], so the bot test applies to pull requests only.
    deploy = (WORKFLOWS / "deploy-site.yml").read_text(encoding="utf-8")
    assert "github.event_name != 'pull_request' || (github.event.pull_request.head.repo.full_name == github.repository && !endsWith(github.actor, '[bot]') && !endsWith(github.event.pull_request.user.login, '[bot]'))" in deploy
    harness = (WORKFLOWS / "harness.yml").read_text(encoding="utf-8")
    assert "(github.event_name != 'pull_request' || (github.event.pull_request.head.repo.full_name == github.repository && !endsWith(github.actor, '[bot]') && !endsWith(github.event.pull_request.user.login, '[bot]')))" in harness
    # F-03: the Cloudflare token is set on the steps that call wrangler, never on a whole job.
    for name in ("infra.yml", "deploy-site.yml", "board.yml"):
        lines = (WORKFLOWS / name).read_text(encoding="utf-8").splitlines()
        for i, line in enumerate(lines):
            if "secrets.CLOUDFLARE_API_TOKEN" in line and "apiToken:" not in line:
                env_line = next(lines[j] for j in range(i - 1, -1, -1) if lines[j].strip() == "env:")
                assert len(env_line) - len(env_line.lstrip()) >= 8, f"{name}:{i + 1}: the token is set on a job, not a step"
