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


def test_all_pinned_game_versions_present(lock: dict) -> None:
    assert rl.REQUIRED_GAME_VERSIONS == ("1.20.1", "1.21.1", "1.21.11")
    assert set(rl.REQUIRED_GAME_VERSIONS) <= set(lock["game_versions"])


def test_lock_agrees_with_game_version_research(lock: dict) -> None:
    # 1.21.1 and 1.21.11 come from the download ranking (D-002); 1.20.1 from the owner's scope change (D-029).
    research = json.loads(GAME_VERSIONS_PATH.read_text(encoding="utf-8"))
    pinned = list(lock["game_versions"])
    assert pinned[: len(research["pinned"])] == research["pinned"]
    assert [g for g in pinned if g not in research["pinned"]] == ["1.20.1"]
    assert "D-029" in lock["game_versions"]["1.20.1"]["pinned_by"]


def test_every_game_version_has_all_four_loaders(lock: dict) -> None:
    assert rl.LOADERS == ("fabric", "neoforge", "forge", "quilt")
    for game, entry in lock["game_versions"].items():
        for loader in rl.LOADERS:
            assert isinstance(entry.get(loader), dict), f"{game} {loader}: a build exists for every pinned version today"


def test_runtime_names_match_the_installers(lock: dict) -> None:
    names = {game: {k: e[k]["runtime_names"] for k in ("neoforge", "forge", "quilt")} for game, e in lock["game_versions"].items()}
    assert names["1.20.1"] == {"neoforge": "srg", "forge": "srg", "quilt": "intermediary"}
    for game in ("1.21.1", "1.21.11"):
        assert names[game] == {"neoforge": "mojang-official", "forge": "mojang-official", "quilt": "intermediary"}


def test_java_major_per_game_version(lock: dict) -> None:
    assert lock["jdk"]["major"] == 21
    assert lock["jdk"]["by_game_version"] == {"1.21.1": 21, "1.21.11": 21, "1.20.1": 17}


def test_quilt_api_choice(lock: dict) -> None:
    games = lock["game_versions"]
    assert games["1.20.1"]["quilt"]["api"]["kind"] == "qfapi"
    for game in ("1.21.1", "1.21.11"):
        api = games[game]["quilt"]["api"]
        assert api["kind"] == "fabric-api" and api["version"] == games[game]["fabric"]["api"]["version"]


def test_forge_pins_are_recommended_builds(lock: dict) -> None:
    for game, entry in lock["game_versions"].items():
        assert entry["forge"]["promotion"] == "recommended", game
        assert entry["forge"]["promotions_key"] == f"{game}-recommended"


def test_existing_pins_unchanged(lock: dict) -> None:
    # D-029 extends the lock; the D-022 pins for 1.21.1 and 1.21.11 stay as they were.
    expected = {
        "1.21.1": ("018296bcb207bea6fd25b05f08eee3942dcec406", "0.20.1", "0.116.17+1.21.1", "21.1.265", "2024.11.17"),
        "1.21.11": ("96403ac4a56447675325d2b1b5e320d76a5b5625", "0.20.1", "0.141.6+1.21.11", "21.11.48", "2025.12.20"),
    }
    for game, (manifest_sha1, loader, api, neoforge, parchment) in expected.items():
        e = lock["game_versions"][game]
        assert e["mojang"]["manifest_entry"]["sha1"] == manifest_sha1
        assert e["fabric"]["loader"]["version"] == loader
        assert e["fabric"]["api"]["version"] == api
        assert e["neoforge"]["version"] == neoforge
        assert e["parchment"]["version"] == parchment
    assert lock["global"]["vineflower"]["version"] == "1.12.0"


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
    # three game versions, four loaders
    ("missing 1.20.1", lambda d: d["game_versions"].pop("1.20.1"), "$.game_versions.1.20.1"),
    ("missing forge key", lambda d: _g(d).pop("forge"), "1.21.1.forge"),
    ("missing quilt key", lambda d: _g(d, "1.20.1").pop("quilt"), "1.20.1.quilt"),
    ("forge null without note", lambda d: _g(d).update(forge=None), "1.21.1.forge"),
    ("quilt null with blank note", lambda d: _g(d, "1.21.11").update(quilt=None, quilt_note=" "), "1.21.11.quilt"),
    ("neoforge null without note", lambda d: _g(d, "1.20.1").update(neoforge=None), "1.20.1.neoforge"),
    ("missing source key", lambda d: d["sources"].pop("forge_promotions"), "$.sources.forge_promotions"),
    ("jdk by version disagrees", lambda d: d["jdk"]["by_game_version"].update({"1.20.1": 21}), "$.jdk.by_game_version.1.20.1"),
    # NeoForge for 1.20.1 is the Forge 47.1 fork
    ("1.20.1 neoforge modern artifact", lambda d: _g(d, "1.20.1")["neoforge"].update(artifact="net.neoforged:neoforge"), "1.20.1.neoforge.artifact"),
    ("1.20.1 neoforge mojang names", lambda d: _g(d, "1.20.1")["neoforge"].update(runtime_names="mojang-official"), "1.20.1.neoforge.runtime_names"),
    ("1.20.1 neoforge wrong line", lambda d: _g(d, "1.20.1")["neoforge"].update(version="20.1.5"), "1.20.1.neoforge.version"),
    ("1.20.1 neoforge maven version", lambda d: _g(d, "1.20.1")["neoforge"].update(maven_version="47.1.106"), "1.20.1.neoforge.maven_version"),
    ("1.20.1 neoforge dependency id", lambda d: _g(d, "1.20.1")["neoforge"]["mods_toml"].update(loader_dependency_id="neoforge"), "loader_dependency_id"),
    ("1.20.1 neoforge toml file", lambda d: _g(d, "1.20.1")["neoforge"]["mods_toml"].pop("file"), "1.20.1.neoforge.mods_toml.file"),
    ("1.21.1 neoforge srg names", lambda d: _g(d)["neoforge"].update(runtime_names="srg"), "1.21.1.neoforge.runtime_names"),
    # Forge
    ("forge latest without note", lambda d: _g(d)["forge"].update(promotion="latest", promotions_key="1.21.1-latest"), "forge.promotion_note"),
    ("forge unknown promotion", lambda d: _g(d)["forge"].update(promotion="beta"), "forge.promotion"),
    ("forge promotions key", lambda d: _g(d)["forge"].update(promotions_key="1.21.1-latest"), "forge.promotions_key"),
    ("forge installer elsewhere", lambda d: _g(d)["forge"]["installer"].update(url="https://example.org/forge.jar"), "forge.installer.url"),
    ("forge missing sha512", lambda d: _g(d, "1.20.1")["forge"]["installer"].pop("sha512"), "forge.installer.sha512"),
    ("forge loaderVersion", lambda d: _g(d)["forge"]["mods_toml"].update(loaderVersion="[1,)"), "forge.mods_toml.loaderVersion"),
    ("forge range not on pin", lambda d: _g(d)["forge"]["mods_toml"].update(forge_dependency_range="[52.0.0,)"), "forge_dependency_range"),
    ("forge runtime names", lambda d: _g(d)["forge"].update(runtime_names="intermediary"), "forge.runtime_names"),
    ("forge no evidence", lambda d: _g(d)["forge"].update(runtime_names_evidence=""), "forge.runtime_names_evidence"),
    ("forge maven version", lambda d: _g(d)["forge"].update(maven_version="52.1.20"), "forge.maven_version"),
    ("forge bad version", lambda d: _g(d)["forge"].update(version="52.1"), "forge.version"),
    # Quilt
    ("quilt beta loader", lambda d: _g(d)["quilt"]["loader"].update(version="0.31.0-beta.5"), "quilt.loader.version"),
    ("quilt unstable loader", lambda d: _g(d)["quilt"]["loader"].update(stable=False), "quilt.loader.stable"),
    ("quilt loader off maven", lambda d: _g(d)["quilt"]["loader"].update(url="https://example.org/quilt-loader.jar"), "quilt.loader.url"),
    ("quilt runtime names", lambda d: _g(d)["quilt"].update(runtime_names="hashed"), "quilt.runtime_names"),
    ("quilt intermediary for another game", lambda d: _g(d)["quilt"].update(intermediary="net.fabricmc:intermediary:1.21.11"), "quilt.intermediary"),
    ("quilt api kind", lambda d: _g(d)["quilt"]["api"].update(kind="qsl"), "quilt.api.kind"),
    ("quilt fabric api version drift", lambda d: _g(d)["quilt"]["api"].update(version="0.116.16+1.21.1"), "quilt.api.version"),
    ("quilt api without source", lambda d: _g(d)["quilt"]["api"].pop("source"), "quilt.api.source"),
    ("qfapi wrong project", lambda d: _g(d, "1.20.1")["quilt"]["api"].update(modrinth_project_id="P7dR8mSH"), "quilt.api.modrinth_project_id"),
    ("qfapi for another game", lambda d: _g(d, "1.20.1")["quilt"]["api"].update(version="7.7.0+0.92.2-1.20.4"), "quilt.api.version"),
    ("qfapi short sha512", lambda d: _g(d, "1.20.1")["quilt"]["api"].update(sha512="ab"), "quilt.api.sha512"),
    ("quilt installer maven", lambda d: _g(d)["quilt"]["installer"].update(maven="org.quiltmc:quilt-installer:0.15.0"), "quilt.installer.maven"),
    ("quilt harness method", lambda d: _g(d)["quilt"]["harness"].update(method="server-launcher"), "quilt.harness.method"),
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


@pytest.mark.parametrize("loader", rl.LOADERS)
def test_loader_null_with_note_is_allowed(lock: dict, loader: str) -> None:
    edited = copy.deepcopy(lock)
    _g(edited, "1.20.1")[loader] = None
    _g(edited, "1.20.1")[f"{loader}_note"] = f"No {loader} build exists for 1.20.1 (test case)."
    assert rl.validate(edited) == []


def test_quilt_fabric_api_fallback_needs_a_fabric_pin(lock: dict) -> None:
    edited = copy.deepcopy(lock)
    _g(edited)["fabric"] = None
    _g(edited)["fabric_note"] = "test case"
    assert any("1.21.1.quilt.api.version" in e for e in rl.validate(edited))


def test_neoforge_for_a_version_without_neoforge_must_be_null(lock: dict) -> None:
    edited = copy.deepcopy(lock)
    edited["game_versions"]["1.19.2"] = copy.deepcopy(_g(edited, "1.20.1"))
    errors = rl.validate(edited)
    assert any("1.19.2.neoforge" in e and "no build line" in e for e in errors)
    assert rl.neoforge_line("1.19.2") is None
    assert rl.neoforge_line("1.20.1")["artifact"] == "net.neoforged:forge"
    assert rl.neoforge_line("1.20.2")["prefix"] == "20.2."
    assert rl.neoforge_line("1.21.11")["prefix"] == "21.11."


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
        assert entry["forge"]["version"] in out
        assert entry["quilt"]["loader"]["version"] in out
        assert entry["quilt"]["api"]["version"] in out
    assert "forge=srg" in out and "forge=mojang-official" in out


def test_print_mode_with_a_null_loader(lock: dict, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    edited = copy.deepcopy(lock)
    _g(edited, "1.20.1")["forge"] = None
    _g(edited, "1.20.1")["forge_note"] = "test case"
    path = tmp_path / "lock.json"
    path.write_text(json.dumps(edited), encoding="utf-8")
    assert rl.main(["--print", "--lock", str(path)]) == 0
    assert "forge             none (forge_note)" in capsys.readouterr().out


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
    assert rl.main(["--verify", "--only", "1.19.2", "--lock", str(LOCK_PATH)]) == 2


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

    def _modrinth_version(self, api: dict, game: str, loader: str) -> None:
        src = self.lock["sources"]
        self._json(f"{src['modrinth_api']}/version/{api['modrinth_version_id']}", {
            "id": api["modrinth_version_id"],
            "project_id": api["modrinth_project_id"],
            "version_number": api["version"],
            "version_type": "release",
            "game_versions": [game],
            "loaders": [loader],
            "files": [{"primary": True, "url": api["url"], "size": api["size"],
                       "hashes": {"sha1": api["sha1"], "sha512": api["sha512"]}}],
        })
        self.sizes[api["url"]] = api["size"]

    def modrinth_listing(self, project: str, game: str, loader: str, versions: list[dict]) -> None:
        self.bodies[f"{self.lock['sources']['modrinth_api']}/project/{project}/version?{game}|{loader}"] = json.dumps(versions).encode()

    def _build(self) -> None:
        lock = self.lock
        src = lock["sources"]
        manifest = []
        nf_versions = []
        nf_legacy_versions = []
        promos: dict[str, str] = {}
        quilt_loaders: dict[str, dict] = {}
        quilt_installers: dict[str, dict] = {}
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
            self._modrinth_version(api, game, "fabric")
            self.bodies[api["maven_url"] + ".sha1"] = api["maven_sha1"].encode()
            self.modrinth_listing(api["modrinth_project_id"], game, "fabric", [{"version_number": api["version"], "version_type": "release"}])

            launcher_body = b"PK fake launcher for " + game.encode()
            fb["server_launcher"]["observed_sha256"] = hashlib.sha256(launcher_body).hexdigest()
            fb["server_launcher"]["size"] = len(launcher_body)
            self.bodies[fb["server_launcher"]["url"]] = launcher_body

            nf = e["neoforge"]
            if rl.neoforge_line(game)["artifact"] == "net.neoforged:forge":
                nf_legacy_versions.append(f"{game}-{nf['version']}")
            else:
                nf_versions.append(nf["version"])
            self._artifact(nf["installer"])

            fg = e["forge"]
            promos[f"{game}-{fg['promotion']}"] = fg["version"]
            promos.setdefault(f"{game}-latest", fg["version"])
            self._artifact(fg["installer"])

            qu = e["quilt"]
            ql = qu["loader"]
            quilt_loaders[ql["version"]] = {"version": ql["version"], "file_size": ql["size"],
                                            "hashes": {a: ql[a] for a in ("sha1", "sha256", "sha512")}}
            self._artifact(ql)
            self._json(src["quilt_meta_loader_game"].format(game_version=game),
                       [{"loader": {"version": ql["version"]}, "intermediary": {"maven": qu["intermediary"]}}])
            qi = qu["installer"]
            quilt_installers[qi["version"]] = {"version": qi["version"], "file_size": qi["size"],
                                               "hashes": {a: qi[a] for a in ("sha1", "sha256", "sha512")}}
            self._artifact(qi)
            qa = qu["api"]
            if qa["kind"] == "qfapi":
                self._modrinth_version(qa, game, "quilt")
                self.modrinth_listing(rl.QFAPI_PROJECT, game, "quilt", [{"version_number": qa["version"], "version_type": "release"}])
            else:
                self.modrinth_listing(rl.QFAPI_PROJECT, game, "quilt", [{"version_number": "11.0.0-alpha.3+0.102.0-1.21", "version_type": "beta"}])

            pa = e.get("parchment")
            if pa:
                self.bodies[pa["metadata_url"]] = (
                    f"<metadata><versioning><release>{pa['version']}</release>"
                    f"<versions><version>{pa['version']}</version></versions></versioning></metadata>"
                ).encode()
                self._artifact(pa)

        self._json(src["mojang_version_manifest"], {"versions": manifest})
        self._json(src["neoforge_versions"], {"isSnapshot": False, "versions": nf_versions})
        # the live legacy listing carries a stray un-prefixed entry; the verifier must ignore it
        self._json(src["neoforge_legacy_versions"], {"isSnapshot": False, "versions": nf_legacy_versions + ["47.1.82"]})
        self._json(src["forge_promotions"], {"homepage": "https://files.minecraftforge.net/", "promos": promos})
        self._json(src["quilt_meta_loader"], [{"version": "9.0.0-beta.1", "file_size": 1, "hashes": {}}, *quilt_loaders.values()])
        self._json(src["quilt_meta_installer"], list(quilt_installers.values()))
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
            loaders = json.loads(query["loaders"][0])
            assert len(loaders) == 1
            return url.split("?", 1)[0] + "?" + game + "|" + loaders[0]
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
    assert statuses(report, "INFO") == []
    assert report.count("OK") >= 170


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


def test_verify_covers_every_loader_and_game_version(fake: FakePublishers) -> None:
    report = rl.verify(fake.lock)
    subjects = {r.subject for r in report.results if r.status == "OK"}
    for game in rl.REQUIRED_GAME_VERSIONS:
        for part in ("mojang", "fabric.loader", "fabric.api", "neoforge", "forge", "quilt.loader", "quilt.api", "quilt.installer", "parchment"):
            assert f"{game} {part}" in subjects, f"{game} {part}"


def test_verify_reads_legacy_neoforge_listing(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["neoforge_legacy_versions"]
    fake.bodies[url] = json.dumps({"versions": ["1.20.1-47.1.105", "47.1.82"]}).encode()
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and fails[0].subject == "1.20.1 neoforge" and "1.20.1-47.1.106 not listed" in fails[0].message


def test_verify_reports_newer_legacy_neoforge_as_info(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["neoforge_legacy_versions"]
    fake.bodies[url] = json.dumps({"versions": ["1.20.1-47.1.106", "1.20.1-47.1.107", "47.1.999"]}).encode()
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    assert [i.message for i in statuses(report, "INFO") if i.subject == "1.20.1 neoforge"] == ["a newer stable build exists: 1.20.1-47.1.107"]


def test_verify_forge_recommended_moved_is_info(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["forge_promotions"]
    body = json.loads(fake.bodies[url])
    body["promos"]["1.21.1-recommended"] = "52.1.30"
    body["promos"]["1.21.1-latest"] = "52.1.31"
    fake.bodies[url] = json.dumps(body).encode()
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    infos = [i.message for i in statuses(report, "INFO") if i.subject == "1.21.1 forge"]
    assert any("52.1.30" in m for m in infos) and any("52.1.31" in m for m in infos)


def test_verify_forge_promotion_missing_fails(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["forge_promotions"]
    body = json.loads(fake.bodies[url])
    del body["promos"]["1.20.1-recommended"]
    fake.bodies[url] = json.dumps(body).encode()
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and fails[0].subject == "1.20.1 forge"


def test_verify_fails_on_forge_sidecar_mismatch(fake: FakePublishers) -> None:
    url = fake.lock["game_versions"]["1.21.11"]["forge"]["installer"]["url"] + ".sha512"
    fake.bodies[url] = b"0" * 128
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and fails[0].subject == "1.21.11 forge" and "sha512 mismatch" in fails[0].message


def test_verify_fails_when_quilt_meta_disagrees(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["quilt_meta_loader"]
    listing = json.loads(fake.bodies[url])
    for item in listing:
        if item["version"] == "0.30.2":
            item["hashes"]["sha256"] = "0" * 64
    fake.bodies[url] = json.dumps(listing).encode()
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 3 and all(f.subject.endswith("quilt.loader") and "sha256" in f.message for f in fails)


def test_verify_newer_stable_quilt_loader_is_info(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["quilt_meta_loader"]
    listing = json.loads(fake.bodies[url])
    listing.insert(0, {"version": "0.30.10", "file_size": 1, "hashes": {}})
    fake.bodies[url] = json.dumps(listing).encode()
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    assert sum("0.30.10" in i.message for i in statuses(report, "INFO")) == 3


def test_verify_fails_when_quilt_profile_names_other_intermediary(fake: FakePublishers) -> None:
    url = fake.lock["sources"]["quilt_meta_loader_game"].format(game_version="1.21.11")
    fake.bodies[url] = json.dumps([{"loader": {"version": "0.30.2"}, "intermediary": {"maven": "org.quiltmc:hashed:1.21.11"}}]).encode()
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and fails[0].subject == "1.21.11 quilt.loader"


def test_verify_warns_when_qfapi_appears_for_a_fabric_api_version(fake: FakePublishers) -> None:
    fake.modrinth_listing(rl.QFAPI_PROJECT, "1.21.1", "quilt", [{"version_number": "12.0.0+0.116.17-1.21.1", "version_type": "release"}])
    report = rl.verify(fake.lock)
    assert statuses(report, "FAIL") == []
    warns = statuses(report, "WARN")
    assert len(warns) == 1 and warns[0].subject == "1.21.1 quilt.api" and "12.0.0" in warns[0].message


def test_verify_fails_when_qfapi_file_changes(fake: FakePublishers) -> None:
    api = fake.lock["game_versions"]["1.20.1"]["quilt"]["api"]
    url = f"{fake.lock['sources']['modrinth_api']}/version/{api['modrinth_version_id']}"
    body = json.loads(fake.bodies[url])
    body["files"][0]["size"] += 1
    fake.bodies[url] = json.dumps(body).encode()
    fails = statuses(rl.verify(fake.lock), "FAIL")
    assert len(fails) == 1 and fails[0].subject == "1.20.1 quilt.api" and "size" in fails[0].message


def test_verify_null_loader_is_info(fake: FakePublishers) -> None:
    fake.lock["game_versions"]["1.20.1"]["forge"] = None
    fake.lock["game_versions"]["1.20.1"]["forge_note"] = "test case"
    report = rl.verify(fake.lock, only="1.20.1")
    assert statuses(report, "FAIL") == []
    assert any(i.subject == "1.20.1 forge" and "test case" in i.message for i in statuses(report, "INFO"))


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
