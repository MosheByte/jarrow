"""Shared fixtures for the tools/ tests: importable tools and throwaway git repositories."""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

TOOLS = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(TOOLS))

import ledger  # noqa: E402

TRAILER = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"

VALID_DECISIONS = "\n".join(
    [
        "# Decisions",
        "",
        "## D-001 — Pin the game versions",
        "",
        "**Date:** 2026-11-03",
        "**Decision:** Pin 1.21.1 and 1.21.11.",
        "**Why:** Most downloaded on Modrinth.",
        "**Reversal cost:** Low.",
        "**Links:** docs/research/game-versions.json",
        "",
    ]
)


@pytest.fixture(autouse=True)
def isolated_git(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    """Keeps the developer's global and system git config (hooks, signing, autocrlf) out."""
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(tmp_path / "empty-gitconfig"))
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
    for name in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
        monkeypatch.delenv(name, raising=False)


class GitRepo:
    """A temporary repository with a local identity, used to exercise pr_ready."""

    def __init__(self, root: Path) -> None:
        self.root = root
        root.mkdir(parents=True, exist_ok=True)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Jarrow Test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.autocrlf", "false")
        self.git("config", "core.hooksPath", str(root / ".no-hooks"))

    def git(self, *args: str) -> str:
        proc = subprocess.run(
            ["git", "-C", str(self.root), *args],
            capture_output=True,
            check=False,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
        )
        if proc.returncode != 0:
            raise AssertionError(
                f"git {' '.join(args)} failed: {proc.stderr.decode('utf-8', 'replace')}"
            )
        return proc.stdout.decode("utf-8", "replace")

    def write(self, rel: str, content: str | bytes) -> Path:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, str):
            content = content.encode("utf-8")
        path.write_bytes(content)
        return path

    def remove(self, rel: str) -> None:
        self.git("rm", "-q", rel)

    def commit(self, message: str, *, author: str | None = None, trailer: bool = True) -> str:
        if trailer:
            message = f"{message}\n\n{TRAILER}\n"
        self.git("add", "-A", "-f")
        args = ["commit", "-q", "--allow-empty", "-m", message]
        if author:
            args += ["--author", author]
        self.git(*args)
        return self.head()

    def head(self) -> str:
        return self.git("rev-parse", "HEAD").strip()

    def regenerate_ledger(self) -> None:
        self.write("LEDGER.md", ledger.render(self.root))


@pytest.fixture
def repo(tmp_path: Path) -> GitRepo:
    """A repository that passes every pr_ready check, with one commit."""
    r = GitRepo(tmp_path / "repo")
    r.write("README.md", "# Test\n")
    r.write("DECISIONS.md", VALID_DECISIONS)
    r.write("crates/report/Cargo.toml", '[package]\nname = "jarrow-report"\n')
    r.write("crates/report/src/lib.rs", "//! Report model.\n")
    r.write("docs/architecture.md", "# Architecture\n\n- `crates/report`: the report model.\n")
    r.regenerate_ledger()
    r.commit("Initial commit")
    return r
