"""Tests for tools/release_notes.py: the range, the pull requests, the contributors, the text.

Every test runs offline against a throwaway git repository and a fake gh client.
"""
from __future__ import annotations

import re
from pathlib import Path

import pytest

import release_notes
from conftest import GitRepo
from release_notes import Pull

REPO = "jarrow-dev/jarrow"
OPUS = "Claude Opus 5.5 <noreply@anthropic.com>"
OWNER = "Moshe Barami <348820118+MosheByte@users.noreply.github.com>"
JANE = "Jane Doe <123+janedoe@users.noreply.github.com>"
PRIVATE = "Sam Private <sam@example.invalid>"
STRANGER = "Lee Unknown <lee@example.invalid>"
DEPENDABOT = "dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>"
JARROW_BOT = "jarrow-bot <jarrow-bot@noreply.invalid>"
ROOT = Path(__file__).resolve().parents[2]


class FakeGh:
    def __init__(
        self,
        merged: list[Pull] | None = None,
        by_commit: dict[str, list[Pull]] | None = None,
        logins: dict[str, dict[str, str]] | None = None,
    ) -> None:
        self.merged = merged or []
        self.by_commit = by_commit or {}
        self.logins = logins or {}
        self.searches: list[str] = []
        self.commit_lookups: list[str] = []
        self.login_lookups: list[list[str]] = []

    def merged_pulls(self, repo: str, since: str) -> list[Pull]:
        assert repo == REPO
        self.searches.append(since)
        return self.merged

    def commit_pulls(self, repo: str, sha: str) -> list[Pull]:
        self.commit_lookups.append(sha)
        return self.by_commit.get(sha, [])

    def commit_logins(self, repo: str, shas: list[str]) -> dict[str, dict[str, str]]:
        self.login_lookups.append(shas)
        return {sha: self.logins[sha] for sha in shas if sha in self.logins}


def commit(repo: GitRepo, message: str, author: str = OWNER, coauthors: tuple[str, ...] = (OPUS,)) -> str:
    trailers = "".join(f"\nCo-Authored-By: {c}" for c in coauthors)
    body = f"{message}\n{trailers}\n" if coauthors else message
    return repo.commit(body, author=author, trailer=False)


@pytest.fixture
def history(tmp_path: Path) -> GitRepo:
    repo = GitRepo(tmp_path / "r")
    commit(repo, "Baseline")
    commit(repo, "Old work (#1)")
    repo.git("tag", "nightly-20261101")
    commit(repo, "Feature (#2)")
    repo.git("tag", "nightly-20261102")
    return repo


def test_range_starts_at_the_newest_older_nightly(history: GitRepo) -> None:
    a = commit(history, "Board (#3)")
    b = commit(history, "Typo fix (#4)", author=JANE, coauthors=())
    history.git("tag", "nightly-20261103")  # today's tag, from an earlier run: never its own base
    history.git("tag", "nightly-latest")
    history.git("tag", "nightly-2026110")
    gh = FakeGh(merged=[Pull(3, "Board", "MosheByte", False, a), Pull(4, "Typo fix", "janedoe", False, b)])
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", gh)
    assert notes.base_tag == "nightly-20261102"
    assert notes.base == history.git("rev-parse", "nightly-20261102").strip()
    assert [p.number for p in notes.pulls] == [3, 4]
    assert notes.loose == []
    assert gh.commit_lookups == []
    assert re.fullmatch(r"\d{4}-\d{2}-\d{2}", gh.searches[0])


def test_rerun_of_an_old_day_compares_with_the_day_before_it(history: GitRepo) -> None:
    notes = release_notes.collect(history.root, REPO, "nightly-20261102", "nightly-20261102", FakeGh())
    assert notes.base_tag == "nightly-20261101"


def test_without_an_older_nightly_the_range_starts_after_the_first_commit(tmp_path: Path) -> None:
    repo = GitRepo(tmp_path / "r")
    root = commit(repo, "Baseline")
    direct = commit(repo, "Pushed straight to main")
    merged = commit(repo, "Feature (#2)")
    gh = FakeGh(merged=[Pull(2, "Feature", "MosheByte", False, merged)])
    notes = release_notes.collect(repo.root, REPO, "nightly-20261103", "HEAD", gh)
    assert notes.base_tag is None and notes.base == root
    assert [p.number for p in notes.pulls] == [2]
    assert [c.sha for c, _ in notes.loose] == [direct]
    assert gh.commit_lookups == [direct]
    text = release_notes.render(notes)
    assert "## Changes since the first commit" in text
    assert f"after the first commit ([`{root[:7]}`]" in text
    assert "1 pull request and 1 commit without one" in text
    assert f"- [`{direct[:7]}`](https://github.com/{REPO}/commit/{direct}) Pushed straight to main by @MosheByte" in text


def test_only_merge_commits_in_the_range_count(history: GitRepo) -> None:
    inside = commit(history, "Inside (#5)")
    gh = FakeGh(
        merged=[
            Pull(5, "Inside", "MosheByte", False, inside),
            Pull(2, "Feature, before the previous nightly", "MosheByte", False, history.git("rev-parse", "nightly-20261102").strip()),
            Pull(9, "Merged on GitHub after this build", "MosheByte", False, "f" * 40),
        ]
    )
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", gh)
    assert [p.number for p in notes.pulls] == [5]


def test_commits_no_listed_pull_accounts_for_are_looked_up(history: GitRepo) -> None:
    inner = commit(history, "Inner commit of a merge-commit pull request")
    lone = commit(history, "Work on a branch that is not merged yet")
    gh = FakeGh(by_commit={inner: [Pull(6, "Merge-commit pull request", "janedoe", False, "e" * 40)]})
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", gh)
    assert [p.number for p in notes.pulls] == [6]
    assert [c.sha for c, _ in notes.loose] == [lone]
    assert sorted(gh.commit_lookups) == sorted([inner, lone])


def test_contributors_come_from_pulls_authors_and_trailers(history: GitRepo) -> None:
    bump = commit(history, "deps: bump sha1 (#7)", author=DEPENDABOT, coauthors=())
    mixed = commit(history, "Pair work (#8)", author=PRIVATE, coauthors=(OPUS, JANE, STRANGER, OPUS))
    refresh = commit(history, "Refresh the team list", author=JARROW_BOT, coauthors=())
    gh = FakeGh(
        merged=[
            Pull(7, "deps: bump sha1", "dependabot[bot]", True, bump),
            Pull(8, "Pair work", "outsider", False, mixed),
        ],
        logins={mixed: {"sam@example.invalid": "samp", "noreply@anthropic.com": "claude"}},
    )
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", gh)
    who = notes.contributors
    assert sorted(who.people.values()) == ["janedoe", "outsider", "samp"]
    assert list(who.agents.values()) == ["Claude Opus 5.5"]  # by name, not GitHub's "claude"
    assert sorted(who.bots.values()) == ["dependabot[bot]", "jarrow-bot"]
    assert list(who.unmatched.values()) == ["Lee Unknown"]
    assert gh.login_lookups == [[mixed]]  # only commits with an address GitHub has to match
    assert [c.sha for c, _ in notes.loose] == [refresh]

    text = release_notes.render(notes)
    assert "- People: @janedoe, @outsider, @samp" in text
    assert "- Agents: Claude Opus 5.5" in text
    assert "- Bots: `dependabot[bot]`, `jarrow-bot`" in text
    assert "(by the name in the commit): Lee Unknown" in text
    assert "- #7 deps: bump sha1 by `dependabot[bot]`" in text
    assert "- #8 Pair work by @outsider" in text
    assert "by `jarrow-bot`" in text
    assert "@" not in text.replace("@janedoe", "").replace("@outsider", "").replace("@samp", "").replace("@MosheByte", "")
    assert "example.invalid" not in text and "noreply" not in text


def test_the_owner_is_listed_once(history: GitRepo) -> None:
    sha = commit(history, "Feature (#3)")
    gh = FakeGh(merged=[Pull(3, "Feature", "MosheByte", False, sha)])
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", gh)
    assert list(notes.contributors.people.values()) == ["MosheByte"]


def test_render_with_a_previous_nightly(history: GitRepo) -> None:
    sha = commit(history, "Title with <b>markup</b> and *stars* | pipes (#3)")
    gh = FakeGh(merged=[Pull(3, "Title with <b>markup</b> and *stars* | pipes", "MosheByte", False, sha)])
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", gh)
    text = release_notes.render(notes, "https://github.com/")
    base = history.git("rev-parse", "nightly-20261102").strip()
    assert text.startswith("## Changes since nightly-20261102\n\n")
    assert f"after [nightly-20261102](https://github.com/{REPO}/releases/tag/nightly-20261102)" in text
    assert f"[Compare](https://github.com/{REPO}/compare/{base}...{sha})" in text
    assert "- #3 Title with \\<b\\>markup\\</b\\> and \\*stars\\* \\| pipes by @MosheByte" in text
    assert "\n## Contributors\n" in text
    assert "Commits without a pull request" not in text


def test_render_when_nothing_changed(history: GitRepo) -> None:
    gh = FakeGh()
    notes = release_notes.collect(history.root, REPO, "nightly-20261103", "nightly-20261102", gh)
    text = release_notes.render(notes)
    assert ": nothing. This build is from the same commit." in text
    assert "No change, so nobody to list." in text
    assert gh.searches == [] and gh.commit_lookups == []


def test_pull_author_names_apps_by_their_bot_account() -> None:
    assert release_notes.pull_author("app/dependabot", True) == "dependabot[bot]"
    assert release_notes.pull_author("app/renovate", False) == "renovate[bot]"
    assert release_notes.pull_author("dependabot[bot]", True) == "dependabot[bot]"
    assert release_notes.pull_author("MosheByte", False) == "MosheByte"


def test_a_full_search_page_is_an_error(history: GitRepo) -> None:
    commit(history, "Feature (#3)")
    many = [Pull(n, "x", "a", False, "0" * 40) for n in range(release_notes.SEARCH_LIMIT)]
    with pytest.raises(release_notes.NotesError, match="more than the search can list"):
        release_notes.collect(history.root, REPO, "nightly-20261103", "HEAD", FakeGh(merged=many))


def test_main_writes_the_file_and_rejects_bad_tags(history: GitRepo, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    sha = commit(history, "Feature (#3)")
    out = tmp_path / "out" / "changes.md"
    gh = FakeGh(merged=[Pull(3, "Feature", "MosheByte", False, sha)])
    args = ["--repo", REPO, "--tag", "nightly-20261103", "--root", str(history.root), "--out", str(out)]
    assert release_notes.main(args, gh) == 0
    assert out.read_text(encoding="utf-8") == capsys.readouterr().out
    assert "- #3 Feature by @MosheByte" in out.read_text(encoding="utf-8")
    with pytest.raises(SystemExit) as exc:
        release_notes.main(["--repo", REPO, "--tag", "board-20261103", "--root", str(history.root)], gh)
    assert exc.value.code == 2


def test_main_reports_a_git_failure(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    repo = GitRepo(tmp_path / "r")
    commit(repo, "Baseline")
    args = ["--repo", REPO, "--tag", "nightly-20261103", "--root", str(repo.root), "--head", "no-such-rev"]
    assert release_notes.main(args, FakeGh()) == 1
    assert "release_notes: git rev-parse" in capsys.readouterr().err


# ------------------------------------------------------------------ workflow wiring


def test_nightly_puts_both_sections_into_the_notes() -> None:
    text = (ROOT / ".github" / "workflows" / "nightly.yml").read_text(encoding="utf-8")
    job = text.split("\n  changes:\n", 1)[1].split("\n  build:\n", 1)[0]
    assert "pull-requests: read" in job and "contents: read" in job
    assert "fetch-depth: 0" in job and "persist-credentials: false" in job
    assert 'python tools/release_notes.py --repo "$REPO" --tag "$TAG" --head "$HEAD_SHA"' in job
    publish = text.split("\n  publish:\n", 1)[1]
    assert "needs: [meta, build, changes]" in publish
    assert '.replace("{changes}", changes)' in publish
    assert "This is a nightly prerelease, not a release." in publish
    assert publish.index("{{changes}}") < publish.index("## What the CLI can do at this commit")


def test_ci_measures_coverage_without_failing_on_it() -> None:
    text = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    job = text.split("\n  coverage:\n", 1)[1].split("\n  deny:\n", 1)[0]
    assert "cargo llvm-cov --workspace --locked --lcov --output-path lcov.info" in job
    assert re.search(r"uses: taiki-e/install-action@[0-9a-f]{40} # v\d", job)
    assert re.search(r"uses: codecov/codecov-action@[0-9a-f]{40} # v\d", job)
    assert "token: ${{ secrets.CODECOV_TOKEN }}" in job
    assert "fail_ci_if_error: false" in job
    assert "github.event.pull_request.head.repo.full_name == github.repository" in job
    assert "github.actor != 'dependabot[bot]'" in job
    assert re.search(r"needs: \[[^\]]*\bcoverage\b[^\]]*\]", text.split("\n  ci-ok:\n", 1)[1])
    codecov = (ROOT / "codecov.yml").read_text(encoding="utf-8")
    assert codecov.count("informational: true") == 2
