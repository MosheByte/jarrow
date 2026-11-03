"""Tests for tools/dco_check.py on real throwaway git repositories."""
from __future__ import annotations

import os
from pathlib import Path

import pytest

import dco_check
from conftest import TRAILER, GitRepo

AFTER = "2026-11-04T09:00:00+00:00"
BEFORE = "2026-11-03T12:00:00+00:00"
ME = "Jarrow Test <test@example.invalid>"


def commit(r: GitRepo, message: str, *, when: str = AFTER, author: str | None = None, path: str = "f.txt") -> str:
    """Commits with the project trailer in the same final paragraph as any Signed-off-by."""
    r.write(path, message)
    subject, _, trailers = message.partition("\n\n")
    block = f"{trailers}\n{TRAILER}" if trailers else TRAILER
    os.environ["GIT_COMMITTER_DATE"] = when
    os.environ["GIT_AUTHOR_DATE"] = when
    try:
        return r.commit(f"{subject}\n\n{block}\n", author=author, trailer=False)
    finally:
        del os.environ["GIT_COMMITTER_DATE"]
        del os.environ["GIT_AUTHOR_DATE"]


def run(r: GitRepo, base: str, capsys: pytest.CaptureFixture[str]) -> tuple[int, str]:
    code = dco_check.main(["--base", base, "--head", r.head(), "--repo", str(r.root)])
    return code, capsys.readouterr().out


def test_signed_commits_pass(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    base = repo.head()
    commit(repo, f"Add a rule\n\nSigned-off-by: {ME}")
    commit(repo, "Fix it\n\nSigned-off-by: Jarrow Test <TEST@example.INVALID>")
    code, out = run(repo, base, capsys)
    assert code == 0, out
    assert out.count("signed ") == 2


def test_missing_and_foreign_sign_offs_fail(repo: GitRepo, capsys: pytest.CaptureFixture[str], tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    base = repo.head()
    commit(repo, "No sign-off")
    commit(repo, "Signed by someone else\n\nSigned-off-by: Other <other@example.invalid>")
    summary = tmp_path / "summary.md"
    monkeypatch.setenv("GITHUB_STEP_SUMMARY", str(summary))
    monkeypatch.setenv("GITHUB_ACTIONS", "true")
    code, out = run(repo, base, capsys)
    assert code == 1
    assert "no Signed-off-by trailer for the author test@example.invalid" in out
    assert "signed off by other@example.invalid, not by the author" in out
    assert out.count("::error title=DCO::") == 2
    text = summary.read_text(encoding="utf-8")
    assert "2 commit(s) lack a sign-off." in text and "git rebase --signoff" in text


def test_bots_merges_and_old_commits_are_exempt(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    base = repo.head()
    commit(repo, "Bump a dependency", author="dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>")
    commit(repo, "chore(team): refresh", author="jarrow-bot <jarrow-bot@noreply.invalid>")
    commit(repo, "Made before the requirement", when=BEFORE)
    repo.git("switch", "-q", "-c", "side", base)
    commit(repo, f"Side work\n\nSigned-off-by: {ME}", path="side.txt")
    repo.git("switch", "-q", "main")
    os.environ["GIT_COMMITTER_DATE"] = AFTER
    try:
        repo.git("merge", "-q", "--no-ff", "-m", "Merge side", "side")
    finally:
        del os.environ["GIT_COMMITTER_DATE"]
    code, out = run(repo, base, capsys)
    assert code == 0, out
    assert "bot author (dependabot[bot])" in out and "bot author (jarrow-bot)" in out
    assert "committed before 2026-11-03T18:30:00Z" in out
    assert "merge commit" in out
    assert out.count("signed ") == 1


def test_the_cutoff_is_inclusive_of_later_commits(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    base = repo.head()
    commit(repo, "At the cutoff", when="2026-11-03T18:30:00+00:00")
    code, out = run(repo, base, capsys)
    assert code == 1, out


def test_bad_input_is_a_usage_error(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    assert dco_check.main(["--base", "0" * 40, "--head", repo.head(), "--repo", str(repo.root)]) == 2
    assert dco_check.main(["--base", repo.head(), "--head", repo.head(), "--cutoff", "2026-11-03", "--repo", str(repo.root)]) == 2


def test_dco_workflow_is_enforcing_and_names_the_cutoff() -> None:
    text = (Path(__file__).resolve().parents[2] / ".github" / "workflows" / "dco.yml").read_text(encoding="utf-8")
    assert "continue-on-error" not in text
    assert f'DCO_CUTOFF: "{dco_check.DEFAULT_CUTOFF}"' in text
    assert "python tools/dco_check.py" in text
    assert "pull_request:" in text and "merge_group:" in text
