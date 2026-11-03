"""Tests for tools/research_lock.py and research-lock.json.

Everything here is offline: an autouse fixture makes any real socket connection
fail, and the --verify logic is exercised against fake publishers built from
the lock itself.
"""
from __future__ import annotations

import copy
import hashlib
import json
import re
import socket
import sys
import urllib.parse
from pathlib import Path
from typing import Any, Callable

import pytest

TOOLS = Path(__file__).resolve().parent.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import research_lock as rl  # noqa: E402

ROOT = TOOLS.parent
LOCK_PATH = ROOT / "research-lock.json"
GAME_VERSIONS_PATH = ROOT / "docs" / "research" / "game-versions.json"


@pytest.fixture(autouse=True)
def no_network(monkeypatch: pytest.MonkeyPatch) -> None:
    def refuse(*_args: Any, **_kwargs: Any) -> None:
        raise AssertionError("tests must not touch the network")

    monkeypatch.setattr(socket.socket, "connect", refuse)
    monkeypatch.setattr(socket, "create_connection", refuse)


@pytest.fixture
def lock() -> dict:
    return rl.load_lock(LOCK_PATH)


def walk(node: Any, path: str = "$"):
    if isinstance(node, dict):
        for key, value in node.items():
            yield from walk(value, f"{path}.{key}")
            yield path, key, value
    elif isinstance(node, list):
        for i, value in enumerate(node):
            yield from walk(value, f"{path}[{i}]")


# --------------------------------------------------------------------------
# The committed lock
# --------------------------------------------------------------------------


def test_repo_lock_validates(lock: dict) -> None:
    assert rl.validate(lock) == []


def test_both_pinned_game_versions_present(lock: dict) -> None:
    assert set(rl.REQUIRED_GAME_VERSIONS) <= set(lock["game_versions"])


def test_lock_agrees_with_game_version_research(lock: dict) -> None:
    research = json.loads(GAME_VERSIONS_PATH.read_text(encoding="utf-8"))
    assert list(lock["game_versions"]) == research["pinned"]


def test_every_hash_has_the_right_format(lock: dict) -> None:
    lengths = {"sha1": 40, "maven_sha1": 40, "sha256": 64, "observed_sha256": 64, "sha512": 128}
    seen = 0
    for path, key, value in walk(lock):
        if key in lengths:
            seen += 1
            assert isinstance(value, str) and re.fullmatch(f"[0-9a-f]{{{lengths[key]}}}", value), f"{path}.{key}"
    assert seen >= 40


def test_every_url_is_https(lock: dict) -> None:
    for path, key, value in walk(lock):
        if isinstance(value, str) and value.startswith(("http://", "https://")):
            assert value.startswith("https://"), f"{path}.{key}"
        if key in ("url", "maven_url", "metadata_url"):
            assert isinstance(value, str) and value.startswith("https://"), f"{path}.{key}"


def test_lock_names_no_beta_without_flag(lock: dict) -> None:
    for game, entry in lock["game_versions"].items():
        nf = entry["neoforge"]
        assert nf["beta"] == nf["version"].endswith("-beta"), game


def test_lock_file_is_canonical_json(lock: dict) -> None:
    text = LOCK_PATH.read_text(encoding="utf-8")
    assert text == json.dumps(lock, indent=2, ensure_ascii=False) + "\n"


# --------------------------------------------------------------------------
# Schema validation catches broken locks
# --------------------------------------------------------------------------


def _g(lock: dict, game: str = "1.21.1") -> dict:
    return lock["game_versions"][game]


MUTATIONS: list[tuple[str, Callable[[dict], None], str]] = [
    ("missing game version", lambda d: d["game_versions"].pop("1.21.11"), "$.game_versions.1.21.11"),
    ("uppercase sha1", lambda d: _g(d)["fabric"]["loader"].update(sha1=_g(d)["fabric"]["loader"]["sha1"].upper()), "fabric.loader.sha1"),
    ("short sha512", lambda d: _g(d)["fabric"]["api"].update(sha512="ab" * 10), "fabric.api.sha512"),
    ("missing installer sha256", lambda d: _g(d)["neoforge"]["installer"].pop("sha256"), "neoforge.installer.sha256"),
    ("neoforge line mismatch", lambda d: _g(d)["neoforge"].update(version="21.2.1"), "neoforge.version"),
    ("launcher url off pattern", lambda d: _g(d)["fabric"]["server_launcher"].update(url="https://meta.fabricmc.net/x"), "server_launcher.url"),
    ("plain http", lambda d: _g(d)["mojang"]["server_jar"].update(url=_g(d)["mojang"]["server_jar"]["url"].replace("https", "http")), "server_jar.url"),
    ("maven and modrinth disagree", lambda d: _g(d)["fabric"]["api"].update(maven_sha1="0" * 40), "maven_sha1"),
    ("unflagged beta", lambda d: _g(d, "1.21.11")["neoforge"].update(version="21.11.48-beta"), "neoforge.beta"),
    ("unstable loader", lambda d: _g(d)["fabric"]["loader"].update(stable=False), "loader.stable"),
    ("bad maven range", lambda d: _g(d)["neoforge"]["mods_toml"].update(loaderVersion="1"), "mods_toml.loaderVersion"),
    ("range not anchored on pin", lambda d: _g(d)["neoforge"]["mods_toml"].update(neoforge_dependency_range="[21.1.0,)"), "neoforge_dependency_range"),
    ("bad date", lambda d: d.update(locked_at="03/11/2026"), "$.locked_at"),
    ("url not content-addressed", lambda d: _g(d)["mojang"]["server_mappings"].update(sha1="1" * 40), "content-addressed"),
    ("intermediary for another game", lambda d: _g(d)["fabric"]["intermediary"].update(version="1.21.4"), "intermediary.version"),
    ("api for another game", lambda d: _g(d)["fabric"]["api"].update(version="0.116.17+1.21.4"), "fabric.api.version"),
    ("zero size", lambda d: _g(d)["parchment"].update(size=0), "parchment.size"),
    ("unknown hash source", lambda d: _g(d)["fabric"]["installer"].update(hash_source="guess"), "installer.hash_source"),
    ("wrong hash source", lambda d: _g(d)["fabric"]["installer"].update(hash_source="observed"), "installer.hash_source"),
    ("boolean as size", lambda d: _g(d)["fabric"]["loader"].update(size=True), "loader.size"),
    ("parchment null without note", lambda d: _g(d).update(parchment=None), "1.21.1.parchment"),
    ("missing parchment key", lambda d: _g(d).pop("parchment"), "1.21.1.parchment"),
    ("missing mojang", lambda d: _g(d).pop("mojang"), "1.21.1.mojang"),
    ("snapshot pinned", lambda d: _g(d)["mojang"]["manifest_entry"].update(type="snapshot"), "manifest_entry.type"),
    ("empty notes entry", lambda d: d["notes"].append("  "), "$.notes"),
    ("vineflower missing sha1", lambda d: d["global"]["vineflower"].pop("sha1"), "vineflower.sha1"),
    ("schema version", lambda d: d.update(schema_version=2), "$.schema_version"),
]


@pytest.mark.parametrize("name,mutate,expected", MUTATIONS, ids=[m[0] for m in MUTATIONS])
def test_validation_rejects(lock: dict, name: str, mutate: Callable[[dict], None], expected: str) -> None:
    broken = copy.deepcopy(lock)
    mutate(broken)
    errors = rl.validate(broken)
    assert errors, f"{name}: expected a schema error"
    assert any(expected in e for e in errors), f"{name}: {errors}"


def test_parchment_null_with_note_is_allowed(lock: dict) -> None:
    edited = copy.deepcopy(lock)
    _g(edited)["parchment"] = None
    _g(edited)["parchment_note"] = "No Parchment release exists for 1.21.1 (test case)."
    assert rl.validate(edited) == []


def test_parchment_null_with_blank_note_is_rejected(lock: dict) -> None:
    edited = copy.deepcopy(lock)
    _g(edited)["parchment"] = None
    _g(edited)["parchment_note"] = "   "
    assert any("parchment" in e for e in rl.validate(edited))


def test_validation_rejects_non_object() -> None:
    assert rl.validate([]) == ["$: the lock must be a JSON object"]


def test_parse_sidecar() -> None:
    assert rl.parse_sidecar("ABCDEF0123  fabric-loader-0.20.1.jar\n") == "abcdef0123"
    assert rl.parse_sidecar("\n") == ""


# --------------------------------------------------------------------------
# Command line (offline modes)
# --------------------------------------------------------------------------


def test_check_mode(capsys: pytest.CaptureFixture[str]) -> None:
    assert rl.main(["--check", "--lock", str(LOCK_PATH)]) == 0
    assert "schema OK" in capsys.readouterr().out


def test_default_mode_is_check(capsys: pytest.CaptureFixture[str]) -> None:
    assert rl.main(["--lock", str(LOCK_PATH)]) == 0
    assert "schema OK" in capsys.readouterr().out


def test_print_mode(lock: dict, capsys: pytest.CaptureFixture[str]) -> None:
    assert rl.main(["--print", "--lock", str(LOCK_PATH)]) == 0
    out = capsys.readouterr().out
    for game, entry in lock["game_versions"].items():
        assert f"Minecraft {game}" in out
        assert entry["neoforge"]["version"] in out
        assert entry["fabric"]["api"]["version"] in out


def test_broken_lock_exits_1(lock: dict, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    broken = copy.deepcopy(lock)
    broken["game_versions"].pop("1.21.1")
    path = tmp_path / "lock.json"
    path.write_text(json.dumps(broken), encoding="utf-8")
    assert rl.main(["--check", "--lock", str(path)]) == 1
    assert "1.21.1" in capsys.readouterr().err


def test_unreadable_lock_exits_2(tmp_path: Path) -> None:
    path = tmp_path / "lock.json"
    path.write_text("{not json", encoding="utf-8")
    assert rl.main(["--lock", str(path)]) == 2
    assert rl.main(["--lock", str(tmp_path / "missing.json")]) == 2


def test_only_unknown_version_exits_2() -> None:
    assert rl.main(["--verify", "--only", "1.20.1", "--lock", str(LOCK_PATH)]) == 2


# --------------------------------------------------------------------------
# --verify against fake publishers
# --------------------------------------------------------------------------


class FakePublishers:
    """Serves the metadata each publisher would serve, derived from a lock."""

    def __init__(self, lock: dict) -> None:
        self.lock = lock
        self.bodies: dict[str, bytes] = {}
        self.sizes: dict[str, int] = {}
        self.failing: set[str] = set()
        self._build()

    def _json(self, url: str, obj: Any) -> None:
        self.bodies[url] = json.dumps(obj).encode("utf-8")

    def _artifact(self, art: dict) -> None:
        self.sizes[art["url"]] = art["size"]
        if art.get("hash_source") == "maven-sidecar":
            for algo in ("sha1", "sha256", "sha512"):
                if algo in art:
                    self.bodies[art["url"] + "." + algo] = f"{art[algo]}  file.jar\n".encode()

    def _build(self) -> None:
        lock = self.lock
        src = lock["sources"]
        manifest = []
        nf_versions = []
        for game, e in lock["game_versions"].items():
            mj = e["mojang"]
            version_json = json.dumps({
                "id": game,
                "javaVersion": {"majorVersion": mj["java_major"]},
                "downloads": {
                    "server": {k: mj["server_jar"][k] for k in ("url", "sha1", "size")},
                    "server_mappings": {k: mj["server_mappings"][k] for k in ("url", "sha1", "size")},
                    "client_mappings": {k: mj["client_mappings"][k] for k in ("url", "sha1", "size")},
                },
            }).encode("utf-8")
            sha1 = hashlib.sha1(version_json).hexdigest()
            entry = mj["manifest_entry"]
            entry["url"] = f"https://piston-meta.mojang.com/v1/packages/{sha1}/{game}.json"
            entry["sha1"] = sha1
            self.bodies[entry["url"]] = version_json
            manifest.append({"id": game, "url": entry["url"], "sha1": sha1, "type": "release"})
            for key in ("server_jar", "server_mappings", "client_mappings"):
                self._artifact(mj[key])

            fb = e["fabric"]
            loader = fb["loader"]
            self._json(src["fabric_meta_loader"], [{"version": loader["version"], "stable": True}])
            self._json(f"https://meta.fabricmc.net/v2/versions/loader/{game}/{loader['version']}", {"intermediary": {"version": game}})
            self._json(src["fabric_meta_intermediary"].format(game_version=game), [{"maven": f"net.fabricmc:intermediary:{game}", "stable": True}])
            self._json(src["fabric_meta_installer"], [{"version": fb["installer"]["version"], "stable": True}])
            for key in ("loader", "intermediary", "installer"):
                self._artifact(fb[key])

            api = fb["api"]
            self._json(f"{src['modrinth_api']}/version/{api['modrinth_version_id']}", {
                "id": api["modrinth_version_id"],
                "project_id": api["modrinth_project_id"],
                "version_number": api["version"],
                "version_type": "release",
                "game_versions": [game],
                "loaders": ["fabric"],
                "files": [{"primary": True, "url": api["url"], "size": api["size"],
                           "hashes": {"sha1": api["sha1"], "sha512": api["sha512"]}}],
            })
            self.bodies[api["maven_url"] + ".sha1"] = api["maven_sha1"].encode()
            self.sizes[api["url"]] = api["size"]
            prefix = f"{src['modrinth_api']}/project/{api['modrinth_project_id']}/version?"
            self.bodies[prefix + game] = json.dumps([{"version_number": api["version"], "version_type": "release"}]).encode()

            launcher_body = b"PK fake launcher for " + game.encode()
            fb["server_launcher"]["observed_sha256"] = hashlib.sha256(launcher_body).hexdigest()
            fb["server_launcher"]["size"] = len(launcher_body)
            self.bodies[fb["server_launcher"]["url"]] = launcher_body

            nf = e["neoforge"]
            nf_versions.append(nf["version"])
            self._artifact(nf["installer"])

            pa = e.get("parchment")
            if pa:
                self.bodies[pa["metadata_url"]] = (
                    f"<metadata><versioning><release>{pa['version']}</release>"
                    f"<versions><version>{pa['version']}</version></versions></versioning></metadata>"
                ).encode()
                self._artifact(pa)

        self._json(src["mojang_version_manifest"], {"versions": manifest})
        self._json(src["neoforge_versions"], {"isSnapshot": False, "versions": nf_versions})
        vf = lock["global"]["vineflower"]
        self.bodies[vf["metadata_url"]] = (
            f"<metadata><versioning><release>{vf['version']}</release>"
            f"<versions><version>{vf['version']}</version></versions></versioning></metadata>"
        ).encode()
        self._artifact(vf)

    def _key(self, url: str) -> str:
        parts = urllib.parse.urlsplit(url)
        if parts.hostname == "api.modrinth.com" and parts.path.endswith("/version") and parts.query:
            query = urllib.parse.parse_qs(parts.query)
            game = json.loads(query["game_versions"][0])[0]
            assert json.loads(query["loaders"][0]) == ["fabric"]
            return url.split("?", 1)[0] + "?" + game
        return url

    def fetch(self, url: str, limit: int = rl.METADATA_LIMIT) -> bytes:
        key = self._key(url)
        if key in self.failing:
            raise rl.NetError(f"{url}: simulated outage")
        if key not in self.bodies:
            raise rl.NetError(f"{url}: HTTP 404 (not served by the fake)")
        body = self.bodies[key]
        if len(body) > limit:
            raise rl.NetError(f"{url}: too large")
        return body

    def probe(self, url: str) -> rl.Probe:
        if url not in self.sizes:
            return rl.Probe(404, None, None)
        return rl.Probe(200, self.sizes[url], "application/octet-stream")


@pytest.fixture
def fake(lock: dict, monkeypatch: pytest.MonkeyPatch) -> FakePublishers:
    pubs = FakePublishers(copy.deepcopy(lock))
    monkeypatch.setattr(rl, "fetch", pubs.fetch)
    monkeypatch.setattr(rl, "probe", pubs.probe)
    return pubs


def statuses(report: rl.Report, status: str) -> list[rl.Result]:
    return [r for r in report.results if r.status == status]


def test_verify_passes_against_matching_publishers(fake: FakePublishers) -> None:
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    assert statuses(report, "WARN") == []
    assert report.count("OK") >= 70


def test_verify_only_one_game_version(fake: FakePublishers) -> None:
    report = rl.verify(fake.lock, only="1.21.11")
    subjects = {r.subject.split()[0] for r in report.results}
    assert subjects == {"1.21.11"}
    assert statuses(report, "FAIL") == []


def test_verify_fails_on_sidecar_mismatch(fake: FakePublishers) -> None:
    url = fake.lock["game_versions"]["1.21.1"]["neoforge"]["installer"]["url"] + ".sha256"
    fake.bodies[url] = b"f" * 64
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and fails[0].subject == "1.21.1 neoforge" and "sha256 mismatch" in fails[0].message


def test_verify_fails_on_size_mismatch(fake: FakePublishers) -> None:
    url = fake.lock["game_versions"]["1.21.11"]["mojang"]["server_jar"]["url"]
    fake.sizes[url] += 1
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and "size" in fails[0].message


def test_verify_fails_on_unreachable_url(fake: FakePublishers) -> None:
    del fake.sizes[fake.lock["global"]["vineflower"]["url"]]
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and "HTTP 404" in fails[0].message


def test_verify_fails_when_modrinth_disagrees(fake: FakePublishers) -> None:
    api = fake.lock["game_versions"]["1.21.1"]["fabric"]["api"]
    url = f"{fake.lock['sources']['modrinth_api']}/version/{api['modrinth_version_id']}"
    body = json.loads(fake.bodies[url])
    body["files"][0]["hashes"]["sha512"] = "0" * 128
    fake.bodies[url] = json.dumps(body).encode()
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and "sha512" in fails[0].message


def test_verify_fails_when_version_json_changes(fake: FakePublishers) -> None:
    entry = fake.lock["game_versions"]["1.21.1"]["mojang"]["manifest_entry"]
    fake.bodies[entry["url"]] += b" "
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert any("version JSON sha1" in f.message for f in fails)


def test_verify_warns_on_manifest_drift(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["mojang_version_manifest"]
    manifest = json.loads(fake.bodies[url])
    manifest["versions"][0]["sha1"] = "a" * 40
    fake.bodies[url] = json.dumps(manifest).encode()
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    warns = statuses(report, "WARN")
    assert len(warns) == 1 and "drift" in warns[0].message


def test_verify_warns_when_generated_launcher_changes(fake: FakePublishers) -> None:
    fake.bodies[fake.lock["game_versions"]["1.21.11"]["fabric"]["server_launcher"]["url"]] = b"different"
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    assert [w.subject for w in statuses(report, "WARN")] == ["1.21.11 fabric.server_launcher"]


def test_verify_reports_newer_upstream_as_info(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["fabric_meta_loader"]
    fake.bodies[url] = json.dumps([{"version": "9.9.9", "stable": True}, {"version": "0.20.1", "stable": True}]).encode()
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    assert any("9.9.9" in i.message for i in statuses(report, "INFO"))


def test_verify_outage_is_a_failure_not_a_crash(fake: FakePublishers) -> None:
    fake.failing.add(fake.lock["sources"]["neoforge_versions"])
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert fails and all("could not check" in f.message for f in fails)


def test_verify_cli_strict_turns_warnings_into_failure(
    fake: FakePublishers, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    url = fake.lock["sources"]["mojang_version_manifest"]
    manifest = json.loads(fake.bodies[url])
    manifest["versions"][0]["sha1"] = "a" * 40
    fake.bodies[url] = json.dumps(manifest).encode()
    path = tmp_path / "lock.json"
    path.write_text(json.dumps(fake.lock), encoding="utf-8")
    assert rl.main(["--verify", "--lock", str(path)]) == 0
    assert rl.main(["--verify", "--strict", "--lock", str(path)]) == 1
    assert rl.main(["--verify", "--json", "--lock", str(path)]) == 0
    out = capsys.readouterr().out
    report = json.loads(out[out.index('{\n  "results"'):])
    assert report["counts"]["WARN"] == 1 and report["passed"] is True
