"""Tests for what .github/workflows/publish.yml publishes (D-012, docs/releases.md).

Only jarrow-report and jarrow may reach crates.io, the npm package shares their version and
repository, every published LICENSE equals the root LICENSE, and the workflow keeps its
token rules. Standard library only, no network.
"""
from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "publish.yml"
PUBLISHED_CRATES = {"report": "jarrow-report", "cli": "jarrow"}


def manifest(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def workspace() -> dict:
    return manifest(ROOT / "Cargo.toml")["workspace"]


def test_only_report_and_cli_are_publishable() -> None:
    assert workspace()["package"]["publish"] is False
    crates = sorted(p.parent for p in (ROOT / "crates").glob("*/Cargo.toml"))
    assert crates, "no crates found"
    for crate in crates:
        package = manifest(crate / "Cargo.toml")["package"]
        publish = package.get("publish", {"workspace": True})
        if crate.name in PUBLISHED_CRATES:
            assert package["name"] == PUBLISHED_CRATES[crate.name]
            assert publish is True, f"crates/{crate.name} must set publish = true"
            assert (crate / package["readme"]).is_file()
            assert len(package["keywords"]) <= 5 and len(package["categories"]) <= 5
        else:
            assert publish in (False, {"workspace": True}), f"crates/{crate.name} must stay private"


def test_workspace_dependency_matches_the_version() -> None:
    ws = workspace()
    assert ws["dependencies"]["jarrow-report"]["version"] == ws["package"]["version"]


def test_npm_package_matches_the_workspace() -> None:
    ws = workspace()["package"]
    pkg = json.loads((ROOT / "packages" / "npm" / "package.json").read_text(encoding="utf-8"))
    assert pkg["name"] == "jarrow"
    assert pkg["version"] == ws["version"]
    assert pkg["license"] == ws["license"] == "MIT"
    assert pkg["repository"]["url"] == f"git+{ws['repository']}.git"
    assert pkg["repository"]["directory"] == "packages/npm"
    assert pkg["files"] == ["tokens.css", "tokens.json", "README.md", "LICENSE"]
    assert "scripts" not in pkg, "the publish job runs no package scripts"


def test_generated_tokens_are_not_committed() -> None:
    ignored = (ROOT / "packages" / "npm" / ".gitignore").read_text(encoding="utf-8").split()
    assert {"tokens.css", "tokens.json"} <= set(ignored)


def test_published_licenses_equal_the_root_license() -> None:
    root = (ROOT / "LICENSE").read_bytes().replace(b"\r\n", b"\n")
    for rel in ("crates/cli/LICENSE", "crates/report/LICENSE", "packages/npm/LICENSE"):
        assert (ROOT / rel).read_bytes().replace(b"\r\n", b"\n") == root, rel


def jobs(text: str) -> dict[str, str]:
    """The text of each job under `jobs:`, by job id."""
    body = text.split("\njobs:\n", 1)[1]
    parts = re.split(r"^  ([A-Za-z][\w-]*):\s*$", body, flags=re.MULTILINE)
    return dict(zip(parts[1::2], parts[2::2]))


def test_workflow_rules() -> None:
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "\non:\n  workflow_dispatch:\n" in text, "manual runs only"
    assert "\npermissions:\n  contents: read\n" in text
    for action in re.findall(r"uses:\s*(\S+)", text):
        assert re.fullmatch(r"[\w.-]+/[\w./-]+@[0-9a-f]{40}", action), f"not pinned to a SHA: {action}"
    by_job = jobs(text)
    assert set(by_job) == {"crates", "npm-tokens", "npm"}
    assert [job for job, body in by_job.items() if "id-token: write" in body] == ["npm"]
    assert set(re.findall(r"secrets\.(\w+)", text)) == {"CARGO_REGISTRY_TOKEN", "NPM_TOKEN"}
    assert "secrets.CARGO_REGISTRY_TOKEN" in by_job["crates"]
    assert "secrets.NPM_TOKEN" in by_job["npm"]
    for line in text.splitlines():
        if re.search(r"\b(echo|printf)\b", line):
            assert not re.search(r"TOKEN", line), f"prints a token variable: {line.strip()}"
