"""Tests for tools/contributors.py: grouping, counts, determinism and the README markers.

Every test runs offline against a throwaway git repository, or with a fake gh client.
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest

import contributors
from conftest import GitRepo

OPUS = "Claude Opus 5.5 <noreply@anthropic.com>"
SONNET = "Claude Sonnet 4.6 <noreply@anthropic.com>"
OWNER = "MosheByte <348820118+MosheByte@users.noreply.github.com>"
JANE = "Jane Doe <123+janedoe@users.noreply.github.com>"
DEPENDABOT = "dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>"
UNRESOLVED = "Real Person <real.person@example.invalid>"
BOT = "jarrow-bot <jarrow-bot@noreply.invalid>"
WEB_FLOW = "GitHub <noreply@github.com>"

README = "\n".join(
    [
        "# Test project",
        "",
        "Intro text that must survive.",
        "",
        "## Who works on Jarrow",
        "",
        contributors.START_MARKER,
        "stale text that the generator replaces",
        contributors.END_MARKER,
        "",
        "## After",
        "",
        "Text after the markers.",
        "",
    ]
)

WORKFLOWS = {
    ".github/workflows/ci.yml": "name: ci\non: [push]\njobs: {}\n",
    ".github/workflows/codeql.yml": (
        "name: codeql\njobs:\n  analyze:\n    strategy:\n      matrix:\n        include:\n"
        "          - language: python\n            build-mode: none\n"
        "    steps:\n      - uses: github/codeql-action/init@0000\n"
    ),
    ".github/workflows/contributors.yml": (
        "name: contributors\njobs:\n  refresh:\n    steps:\n      - env:\n"
        "          BOT_DEPLOY_KEY: ${{ secrets.BOT_DEPLOY_KEY }}\n"
    ),
    ".github/workflows/scorecard.yml": (
        "name: scorecard\njobs:\n  analysis:\n    steps:\n"
        "      - uses: github/codeql-action/upload-sarif@0000\n"
    ),
    ".github/dependabot.yml": (
        "version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n"
        "  - package-ecosystem: pip\n    directory: /tools\n"
    ),
}


def split(person: str) -> tuple[str, str]:
    name, _, rest = person.partition(" <")
    return name, rest.rstrip(">")


def message(subject: str, *trailers: str, refresh: bool = False) -> str:
    lines = [f"Co-Authored-By: {t}" for t in trailers]
    if refresh:
        lines.insert(0, "Jarrow-Generated: contributors")
    return subject + ("\n\n" + "\n".join(lines) if lines else "") + "\n"


def commit(
    repo: GitRepo,
    monkeypatch: pytest.MonkeyPatch,
    msg: str,
    *,
    author: str,
    date: str,
    committer: str | None = None,
    files: dict[str, str] | None = None,
) -> str:
    for rel, content in (files or {}).items():
        repo.write(rel, content)
    cname, cemail = split(committer or author)
    monkeypatch.setenv("GIT_AUTHOR_DATE", date)
    monkeypatch.setenv("GIT_COMMITTER_DATE", date)
    monkeypatch.setenv("GIT_COMMITTER_NAME", cname)
    monkeypatch.setenv("GIT_COMMITTER_EMAIL", cemail)
    repo.git("add", "-A", "-f")
    repo.git("commit", "-q", "--allow-empty", "-m", msg, "--author", author)
    return repo.head()


@pytest.fixture
def team_repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> GitRepo:
    """A human author, a bot author, an unresolvable human, the deploy key, two models."""
    repo = GitRepo(tmp_path / "team")
    commit(
        repo,
        monkeypatch,
        message("Initial", OPUS),
        author=OWNER,
        date="2026-01-01T10:00:00+02:00",
        files={"README.md": README, "tools/x.py": "print(1)\n", **WORKFLOWS},
    )
    commit(
        repo,
        monkeypatch,
        message("Docs and tests", OPUS, SONNET),
        author=JANE,
        date="2026-01-02T10:00:00+02:00",
        files={"docs/a.md": "# A\n", "tools/tests/test_x.py": "def test(): pass\n"},
    )
    commit(
        repo,
        monkeypatch,
        message("deps: bump something"),
        author=DEPENDABOT,
        committer=WEB_FLOW,
        date="2026-01-03T10:00:00+02:00",
        files={"site/package.json": "{}\n"},
    )
    commit(
        repo,
        monkeypatch,
        message("Code", SONNET),
        author=UNRESOLVED,
        date="2026-01-04T10:00:00+02:00",
        files={"crates/y.rs": "fn main() {}\n"},
    )
    commit(
        repo,
        monkeypatch,
        message("ledger: nightly", OPUS),
        author=BOT,
        date="2026-01-05T10:00:00+02:00",
        files={"LEDGER.md": "# Ledger\n"},
    )
    commit(
        repo,
        monkeypatch,
        message("chore(team): refresh", OPUS, refresh=True),
        author=BOT,
        date="2026-01-06T10:00:00+02:00",
        files={"docs/team.md": "old\n"},
    )
    return repo


def offline(root: Path, **kwargs) -> dict[str, str]:
    logs: list[str] = []
    out = contributors.generate(root, gh=None, log=logs.append, **kwargs)
    return out


def team_json(outputs: dict[str, str]) -> dict:
    return json.loads(outputs[contributors.TEAM_JSON_PATH])


def members(data: dict, group: str) -> list[dict]:
    return next(g for g in data["groups"] if g["id"] == group)["members"]


# ---------------------------------------------------------------- grouping and counts


def test_groups_are_people_agents_bots(team_repo: GitRepo) -> None:
    data = team_json(offline(team_repo.root))
    assert [g["id"] for g in data["groups"]] == ["people", "agents", "bots"]
    assert [g["title"] for g in data["groups"]] == ["People", "Agents", "Bots and automation"]
    agents_note = data["groups"][1]["note"]
    assert "owner's account" in agents_note and "Co-Authored-By" in agents_note


def test_people_are_logins_with_the_owner_first(team_repo: GitRepo) -> None:
    people = members(team_json(offline(team_repo.root)), "people")
    assert [p["login"] for p in people] == ["MosheByte", "janedoe"]
    owner, jane = people
    assert owner["role"] == contributors.OWNER_ROLE
    assert owner["contributions"] == ["ideas", "projectManagement"]
    assert owner["commits"] == 1
    assert owner["avatar_url"] == "https://avatars.githubusercontent.com/u/348820118?v=4"
    assert owner["profile_url"] == "https://github.com/MosheByte"
    assert jane["name"] == "janedoe"
    assert jane["commits"] == 1
    assert jane["contributions"] == ["doc", "test"]


def test_agents_counted_from_trailers(team_repo: GitRepo) -> None:
    agents = members(team_json(offline(team_repo.root)), "agents")
    by_name = {a["name"]: a for a in agents}
    assert list(by_name) == ["Claude Opus 5.5", "Claude Sonnet 4.6"]
    # Opus: commits 1, 2 and 5. The refresh commit (6) is not counted.
    assert by_name["Claude Opus 5.5"]["commits"] == 3
    assert by_name["Claude Opus 5.5"]["role"] == contributors.OPUS_ROLE
    assert by_name["Claude Sonnet 4.6"]["commits"] == 2
    assert by_name["Claude Sonnet 4.6"]["role"] == contributors.AGENT_ROLE
    for agent in agents:
        assert agent["contributions"][0] == "agent"
        assert agent["avatar_url"] == "https://avatars.githubusercontent.com/u/76263028?v=4"
        assert agent["profile_url"] == "https://www.anthropic.com/claude"


def test_bots_from_files_and_history_only(team_repo: GitRepo) -> None:
    bots = {b["login"]: b for b in members(team_json(offline(team_repo.root)), "bots")}
    assert sorted(bots) == ["dependabot", "github-actions", "github-advanced-security", "jarrow-bot"]
    assert bots["dependabot"]["commits"] == 1
    assert "Cargo and pip" in bots["dependabot"]["role"]
    assert bots["github-actions"]["role"] == (
        "Runs the workflows in .github/workflows: CI, CodeQL, this list and Scorecard."
    )
    assert bots["github-actions"]["contributions"] == ["ci", "infra"]
    assert bots["github-advanced-security"]["name"] == "CodeQL"
    # A workflow that only uploads SARIF (Scorecard) is not a CodeQL analysis.
    assert bots["github-advanced-security"]["role"] == "Code scanning with CodeQL (codeql.yml) for Python."
    key = bots["jarrow-bot"]
    assert key["commits"] == 1
    assert key["avatar_url"] is None and key["icon"] == "key"
    assert "contributors.yml" in key["role"]
    for absent in ("socket-security", "renovate", "release-please", "claude"):
        assert absent not in bots


def test_no_commit_names_or_emails_reach_the_outputs(team_repo: GitRepo) -> None:
    outputs = offline(team_repo.root)
    for rel, text in outputs.items():
        for leak in ("Real Person", "real.person", "Jane Doe", "Jarrow Test", "noreply", "example.invalid"):
            assert leak not in text, f"{leak!r} leaked into {rel}"


def test_unresolved_identity_is_reported_by_sha_only(team_repo: GitRepo) -> None:
    logs: list[str] = []
    contributors.generate(team_repo.root, gh=None, log=logs.append)
    joined = "\n".join(logs)
    assert "left out" in joined
    assert "Real Person" not in joined and "example.invalid" not in joined


def test_generated_at_is_the_newest_counted_commit(team_repo: GitRepo) -> None:
    data = team_json(offline(team_repo.root))
    # Commit 5 at 10:00 +02:00; the later refresh commit is ignored.
    assert data["generated_at"] == "2026-01-05T08:00:00Z"


def test_identity_kinds() -> None:
    kind = contributors.identity_kind
    assert kind("Claude Opus 5.5", "noreply@anthropic.com") == "agent"
    assert kind("Claude", "noreply@anthropic.com") == "agent"
    assert kind("Claude Monet", "claude@monet.example") == "human"
    assert kind("dependabot[bot]", "x@users.noreply.github.com") == "bot"
    assert kind("jarrow-bot", "jarrow-bot@noreply.invalid") == "bot"
    assert kind("GitHub", "noreply@github.com") == "web-flow"
    assert kind("Copilot", "175728472+Copilot@users.noreply.github.com") == "agent"


# ---------------------------------------------------------------- outputs and modes


def test_outputs_are_deterministic_and_lf(team_repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    root = team_repo.root
    assert contributors.main(["--offline", "--root", str(root)]) == 0
    first = {rel: (root / rel).read_bytes() for rel in contributors.OUTPUT_PATHS}
    assert contributors.main(["--offline", "--root", str(root)]) == 0
    second = {rel: (root / rel).read_bytes() for rel in contributors.OUTPUT_PATHS}
    assert first == second
    for rel, data in first.items():
        assert b"\r" not in data, rel
        assert data.endswith(b"\n"), rel
    assert contributors.main(["--offline", "--check", "--root", str(root)]) == 0
    capsys.readouterr()


def test_check_reports_a_stale_output_and_writes_nothing(
    team_repo: GitRepo, capsys: pytest.CaptureFixture[str]
) -> None:
    root = team_repo.root
    assert contributors.main(["--offline", "--root", str(root)]) == 0
    team_md = root / contributors.TEAM_MD_PATH
    team_md.write_text("edited by hand\n", encoding="utf-8", newline="\n")
    capsys.readouterr()
    assert contributors.main(["--offline", "--check", "--root", str(root)]) == 1
    assert "stale: docs/team.md" in capsys.readouterr().out
    assert team_md.read_text(encoding="utf-8") == "edited by hand\n"


def test_rc_follows_the_all_contributors_spec(team_repo: GitRepo) -> None:
    rc = json.loads(offline(team_repo.root)[contributors.RC_PATH])
    assert rc["projectName"] == "jarrow"
    assert rc["projectOwner"] == "MosheByte"
    assert rc["repoType"] == "github"
    assert rc["repoHost"] == "https://github.com"
    assert rc["files"] == ["README.md"]
    assert rc["imageSize"] == 80
    assert rc["contributorsPerLine"] == 7
    assert rc["commit"] is False
    assert set(rc["types"]) == {"agent", "bot", "ci"}
    assert all({"symbol", "description"} <= set(v) for v in rc["types"].values())
    logins = [c["login"] for c in rc["contributors"]]
    assert len(logins) == len(set(logins))
    for entry in rc["contributors"]:
        assert set(entry) == {"login", "name", "avatar_url", "profile", "contributions"}
    key = next(c for c in rc["contributors"] if c["login"] == "jarrow-bot")
    assert key["avatar_url"] == "docs/assets/key.svg"


def test_readme_changes_only_between_the_markers(team_repo: GitRepo) -> None:
    readme = offline(team_repo.root)[contributors.README_PATH]
    head, rest = readme.split(contributors.START_MARKER)
    block, tail = rest.split(contributors.END_MARKER)
    old_head, old_rest = README.split(contributors.START_MARKER)
    _, old_tail = old_rest.split(contributors.END_MARKER)
    assert (head, tail) == (old_head, old_tail)
    assert "stale text" not in block
    assert block.index("### People") < block.index("### Agents") < block.index("### Bots and automation")
    assert '<img src="https://avatars.githubusercontent.com/u/348820118?v=4&amp;s=80" width="80px;" alt="MosheByte"/>' in block
    assert f"<sub>{contributors.OWNER_ROLE}</sub>" in block.replace("&#x27;", "'")
    assert '<img src="docs/assets/key.svg"' in block
    assert 'width="14.28%"' in block
    # Running again on the generated README gives the same README.
    (team_repo.root / "README.md").write_text(readme, encoding="utf-8", newline="\n")
    assert offline(team_repo.root)[contributors.README_PATH] == readme


def test_missing_markers_fail_with_a_clear_error(
    team_repo: GitRepo, capsys: pytest.CaptureFixture[str]
) -> None:
    root = team_repo.root
    (root / "README.md").write_text("# No markers here\n", encoding="utf-8", newline="\n")
    assert contributors.main(["--offline", "--root", str(root)]) == 2
    err = capsys.readouterr().err
    assert "ALL-CONTRIBUTORS-LIST:START" in err and "--no-readme" in err
    assert not (root / contributors.TEAM_JSON_PATH).exists()
    assert contributors.main(["--offline", "--no-readme", "--root", str(root)]) == 0
    assert (root / "README.md").read_text(encoding="utf-8") == "# No markers here\n"
    assert (root / contributors.TEAM_JSON_PATH).exists()


def test_end_before_start_is_an_error() -> None:
    text = f"{contributors.END_MARKER}\n{contributors.START_MARKER}\n"
    with pytest.raises(contributors.InputError, match="END marker comes before"):
        contributors.replace_readme_block(text, "\nx\n")


def test_a_refresh_commit_changes_nothing(team_repo: GitRepo, monkeypatch: pytest.MonkeyPatch) -> None:
    root = team_repo.root
    assert contributors.main(["--offline", "--root", str(root)]) == 0
    commit(
        team_repo,
        monkeypatch,
        message("chore(team): refresh", OPUS, refresh=True),
        author=BOT,
        date="2026-02-01T00:00:00+00:00",
    )
    assert contributors.main(["--offline", "--check", "--root", str(root)]) == 0


# ---------------------------------------------------------------- GitHub lookups (fake gh)


class FakeGh:
    """Answers the gh calls the generator makes; anything unknown fails like a 404."""

    def __init__(self, responses: dict[str, object], pr_authors: list[dict] | None = None) -> None:
        self.responses = responses
        self.authors = pr_authors
        self.calls: list[str] = []

    def api(self, path: str) -> object:
        self.calls.append(path)
        for prefix, value in self.responses.items():
            if path.startswith(prefix):
                if isinstance(value, Exception):
                    raise value
                return value
        raise contributors.GhError("gh: Not Found (HTTP 404)")

    def pr_authors(self, repo: str) -> list[dict]:
        if self.authors is None:
            raise contributors.GhError("gh: Resource not accessible by integration (HTTP 403)")
        return self.authors


def suites(*apps: tuple[str, int], runs: int = 1) -> dict:
    return {
        "check_suites": [
            {"app": {"slug": s, "id": i, "name": s, "owner": {"login": "x"}}, "latest_check_runs_count": runs}
            for s, i in apps
        ]
    }


REPO = contributors.REPO_SLUG


def full_gh(*apps: tuple[str, int]) -> FakeGh:
    return FakeGh(
        {
            f"repos/{REPO}/commits/main/check-suites": suites(*apps),
            f"repos/{REPO}/commits/main/check-runs": {"check_runs": []},
            f"repos/{REPO}/keys": [{"title": "jarrow-bot (board-data and LEDGER commits from Actions)"}],
            "user/installations": contributors.GhError(
                "gh: You must authenticate with an access token authorized to a GitHub App "
                "in order to list installations (HTTP 403)"
            ),
        },
        pr_authors=[{"login": "app/dependabot", "is_bot": True}, {"login": "MosheByte", "is_bot": False}],
    )


def bot_logins(outputs: dict[str, str]) -> list[str]:
    return [b["login"] for b in members(team_json(outputs), "bots")]


def test_installed_apps_from_check_suites_and_the_installations_error_is_logged(
    team_repo: GitRepo,
) -> None:
    gh = full_gh(("socket-security", 156372), ("claude", 1236702), ("github-actions", 15368))
    logs: list[str] = []
    outputs = contributors.generate(team_repo.root, gh=gh, log=logs.append)
    assert "socket-security" in bot_logins(outputs)
    assert "claude" in bot_logins(outputs)
    assert any(f"repos/{REPO}/installations" in line and "HTTP 404" in line for line in logs)
    assert any("user/installations" in line and "GitHub App" in line for line in logs)


def test_an_installed_app_with_only_an_empty_check_suite_is_not_listed(
    team_repo: GitRepo,
) -> None:
    # GitHub creates an empty suite for every installed app with the checks permission on each
    # push. That shows the app is installed, not that it acts on the repository.
    gh = full_gh(("github-actions", 15368))
    empty = suites(("claude", 1236702), runs=0)["check_suites"]
    gh.responses[f"repos/{REPO}/commits/main/check-suites"]["check_suites"].extend(empty)
    outputs = contributors.generate(team_repo.root, gh=gh, log=lambda _: None)
    assert "claude" not in bot_logins(outputs)
    assert "github-actions" in bot_logins(outputs)


def test_checks_come_from_the_newest_counted_commit_not_the_refresh_tip(
    team_repo: GitRepo,
) -> None:
    # Commit 5 is the newest counted commit; commit 6 (the tip) is a refresh commit.
    newest = team_repo.git("rev-parse", "HEAD~1").strip()
    gh = full_gh(("github-actions", 15368))
    gh.responses = {
        f"repos/{REPO}/commits/{newest}/check-suites": suites(("socket-security", 156372)),
        f"repos/{REPO}/commits/{newest}/check-runs": {"check_runs": []},
        **gh.responses,
    }
    outputs = contributors.generate(team_repo.root, gh=gh, log=lambda _: None)
    assert "socket-security" in bot_logins(outputs)
    assert not any(c.startswith(f"repos/{REPO}/commits/main/") for c in gh.calls)


@pytest.fixture
def small_repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> GitRepo:
    """One owner commit and the workflow files; no bot ever committed."""
    repo = GitRepo(tmp_path / "small")
    commit(
        repo,
        monkeypatch,
        message("Initial", OPUS),
        author=OWNER,
        date="2026-01-01T10:00:00+02:00",
        files={"README.md": README, **WORKFLOWS},
    )
    return repo


def test_offline_keeps_what_github_confirmed_so_check_passes(small_repo: GitRepo) -> None:
    root = small_repo.root
    gh = full_gh(("socket-security", 156372), ("claude", 1236702))
    online = contributors.generate(root, gh=gh, log=lambda _: None)
    assert {"socket-security", "claude", "jarrow-bot"} <= set(bot_logins(online))
    contributors.write_outputs(root, online)
    assert contributors.stale_outputs(root, offline(root)) == []


def test_a_failed_lookup_keeps_only_the_bots_it_could_confirm(small_repo: GitRepo) -> None:
    root = small_repo.root
    gh = full_gh(("socket-security", 156372), ("claude", 1236702))
    contributors.write_outputs(root, contributors.generate(root, gh=gh, log=lambda _: None))
    # The nightly token: checks and pull requests readable, deploy keys not. The apps are gone
    # from the check suites, so they drop; the deploy key cannot be checked, so it stays.
    nightly = FakeGh(
        {
            f"repos/{REPO}/commits/main/check-suites": suites(("github-actions", 15368)),
            f"repos/{REPO}/commits/main/check-runs": {"check_runs": []},
            f"repos/{REPO}/keys": contributors.GhError("gh: Resource not accessible by integration (HTTP 403)"),
        },
        pr_authors=[],
    )
    logins = bot_logins(contributors.generate(root, gh=nightly, log=lambda _: None))
    assert "socket-security" not in logins and "claude" not in logins
    assert "jarrow-bot" in logins


def test_deploy_key_role_says_how_each_workflow_pushes(small_repo: GitRepo) -> None:
    files = contributors.read_repo_files(small_repo.root)
    files.workflows["contributors.yml"] += "      - run: bash tools/bot_pr.sh contributors t b -- x\n"
    files.workflows["board.yml"] = "jobs:\n  x:\n    env:\n      K: ${{ secrets.BOT_DEPLOY_KEY }}\n"
    role = contributors.role_deploy_key(files)
    assert (
        "as the branch of a bot pull request (tools/bot_pr.sh) for the nightly refresh of this "
        "list (contributors.yml)"
    ) in role
    assert "directly for LEDGER and board-data commits (board.yml)" in role


# ---------------------------------------------------------------- --ledger: contributor numbers


class LedgerGh(FakeGh):
    """FakeGh that also lists merged pull requests (None: the lookup fails)."""

    def __init__(self, merged: list[dict] | None) -> None:
        super().__init__({})
        self.merged = merged

    def merged_pulls(self, repo: str) -> list[dict]:
        if self.merged is None:
            raise contributors.GhError("gh: Resource not accessible by integration (HTTP 403)")
        return self.merged


def merged(number: int, login: str, when: str, is_bot: bool = False) -> dict:
    return {"number": number, "author": {"login": login, "is_bot": is_bot}, "mergedAt": when}


def numbers(root: Path, as_of: str, gh: object = None) -> dict:
    return contributors.ledger_numbers(
        root, gh=gh, as_of=contributors.date.fromisoformat(as_of), log=lambda _: None
    )


def test_ledger_offline_counts_people_and_agents_from_git(team_repo: GitRepo) -> None:
    data = numbers(team_repo.root, "2026-01-10")
    assert data["schema"] == 1 and data["as_of"] == "2026-01-10" and data["window_days"] == 30
    assert data["sources"] == ["git"]
    # Real Person has no login offline; dependabot and jarrow-bot are bots; the refresh commit
    # is not counted. jarrow-bot's commit names Opus, but a bot-authored commit counts for no one.
    assert data["contributors_30d"]["people"] == {"count": 2, "logins": ["janedoe", "MosheByte"]}
    assert data["contributors_30d"]["agents"] == {
        "count": 2,
        "names": ["Claude Opus 5.5", "Claude Sonnet 4.6"],
    }
    assert data["first_time_contributors_30d"] is None


def test_ledger_window_is_30_days_ending_on_as_of(team_repo: GitRepo) -> None:
    # Commit 1 is 2026-01-01 08:00Z, commit 2 is 2026-01-02 08:00Z, commit 4 is 2026-01-04.
    data = numbers(team_repo.root, "2026-01-02")
    assert data["contributors_30d"]["people"]["logins"] == ["janedoe", "MosheByte"]
    # 2026-01-30 is the last as-of day whose window (2026-01-01 to 2026-01-30) holds commit 1.
    late = numbers(team_repo.root, "2026-01-30")
    assert "MosheByte" in late["contributors_30d"]["people"]["logins"]
    after = numbers(team_repo.root, "2026-01-31")
    assert "MosheByte" not in after["contributors_30d"]["people"]["logins"]
    gone = numbers(team_repo.root, "2026-02-04")
    assert gone["contributors_30d"]["people"] == {"count": 0, "logins": []}
    assert gone["contributors_30d"]["agents"] == {"count": 0, "names": []}


def test_ledger_merged_pull_requests_and_first_timers(team_repo: GitRepo) -> None:
    gh = LedgerGh(
        [
            merged(1, "MosheByte", "2025-11-20T10:00:00Z"),
            merged(7, "MosheByte", "2026-01-06T10:00:00Z"),
            merged(8, "newbie", "2026-01-05T23:30:00Z"),
            merged(9, "Newbie", "2026-01-07T01:00:00Z"),
            merged(10, "app/dependabot", "2026-01-05T00:00:00Z", is_bot=True),
            merged(11, "renovate[bot]", "2026-01-05T00:00:00Z"),
            merged(12, "oldtimer", "2025-10-01T00:00:00Z"),
            merged(13, "nobody", "not a time"),
        ]
    )
    data = numbers(team_repo.root, "2026-01-10", gh)
    assert data["sources"] == ["git", "github-pulls"]
    assert data["contributors_30d"]["people"]["logins"] == ["janedoe", "MosheByte", "newbie"]
    # MosheByte's first merged PR is older than the window; newbie's falls in it.
    assert data["first_time_contributors_30d"] == {"count": 1, "logins": ["newbie"]}


def test_ledger_failed_pull_request_lookup_is_null_not_zero(team_repo: GitRepo) -> None:
    logs: list[str] = []
    data = contributors.ledger_numbers(
        team_repo.root, gh=LedgerGh(None), as_of=contributors.date(2026, 1, 10), log=logs.append
    )
    assert data["sources"] == ["git"]
    assert data["first_time_contributors_30d"] is None
    assert any("first-time contributors are not counted" in line for line in logs)


def test_ledger_input_keeps_as_of_until_a_number_changes(
    team_repo: GitRepo, capsys: pytest.CaptureFixture[str]
) -> None:
    root = team_repo.root
    path = root / contributors.LEDGER_INPUT_PATH
    base = ["--ledger", "--offline", "--root", str(root)]
    assert contributors.main([*base, "--as-of", "2026-01-10"]) == 0
    first = path.read_bytes()
    assert b"\r" not in first and json.loads(first)["as_of"] == "2026-01-10"
    # The next night finds the same people: nothing to commit, so no bot pull request.
    assert contributors.main([*base, "--as-of", "2026-01-11"]) == 0
    assert path.read_bytes() == first
    assert contributors.main([*base, "--check", "--as-of", "2026-01-11"]) == 0
    # Later everyone has aged out of the window: the file changes and moves on.
    assert contributors.main([*base, "--check", "--as-of", "2026-02-20"]) == 1
    assert path.read_bytes() == first
    assert contributors.main([*base, "--as-of", "2026-02-20"]) == 0
    later = json.loads(path.read_bytes())
    assert later["as_of"] == "2026-02-20"
    assert later["contributors_30d"]["people"]["count"] == 0
    # Only the ledger input is written in --ledger mode.
    assert not (root / contributors.TEAM_JSON_PATH).exists()
    capsys.readouterr()


def test_ledger_input_renders_in_ledger_md(team_repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    import ledger

    root = team_repo.root
    assert contributors.main(["--ledger", "--offline", "--root", str(root), "--as-of", "2026-01-10"]) == 0
    text = ledger.render(root)
    assert (
        "| Contributors (30 days) | 2 people (janedoe, MosheByte); "
        "2 agents (Claude Opus 5.5, Claude Sonnet 4.6) |"
    ) in text
    assert "| First-time contributors (30 days) | not measured" in text
    capsys.readouterr()


def test_as_of_needs_ledger_and_a_date(team_repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    root = str(team_repo.root)
    for argv in (
        ["--offline", "--root", root, "--as-of", "2026-01-10"],
        ["--ledger", "--offline", "--root", root, "--as-of", "10/01/2026"],
        ["--ledger", "--offline", "--root", root, "--as-of", "2026-02-30"],
    ):
        with pytest.raises(SystemExit) as exit_info:
            contributors.main(argv)
        assert exit_info.value.code == 2
    capsys.readouterr()
