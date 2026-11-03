#!/usr/bin/env python3
"""Check, summarise and re-verify research-lock.json.

research-lock.json pins every external artefact Jarrow depends on for the
three game versions (1.20.1, 1.21.1, 1.21.11) and the four loaders (Fabric,
NeoForge, Forge, Quilt): the Mojang server jar and mappings; Fabric Loader,
intermediary, Fabric API, the Fabric installer and server launcher; NeoForge;
Forge; Quilt Loader, its API library and installer; Parchment; Vineflower.
Each pin carries the hashes the publishers themselves provide. A loader block
may be null only when a sibling '<loader>_note' says why.

Modes (stdlib only, no third-party packages):

  --check    offline schema validation (default). No network.
  --print    offline human summary of what is pinned.
  --verify   re-fetch every publisher metadata source, check that each pinned
             URL is reachable (HEAD, or a 1-byte ranged GET when HEAD is
             refused) with the pinned size, and that each pinned hash equals
             the publisher's own hash file, manifest field or API value.
             Large artefacts (game jars, installers, mappings) are never
             downloaded; only metadata and the ~180 KB generated Fabric
             server launcher are read.

Exit status: 0 when nothing failed, 1 when a check failed (with --strict,
warnings count as failures), 2 for a usage error or an unreadable lock file.

Usage:
  py -3.12 tools/research_lock.py [--check | --print | --verify] [--lock PATH]
                                  [--only GAME_VERSION] [--strict] [--json]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LOCK = ROOT / "research-lock.json"
USER_AGENT = "MosheByte/jarrow-research/0.1 (github.com/MosheByte/jarrow)"
REQUIRED_GAME_VERSIONS = ("1.20.1", "1.21.1", "1.21.11")
LOADERS = ("fabric", "neoforge", "forge", "quilt")
REQUIRED_SOURCES = (
    "mojang_version_manifest", "fabric_meta_loader", "fabric_meta_installer", "fabric_meta_intermediary",
    "modrinth_api", "neoforge_versions", "neoforge_legacy_versions", "forge_promotions",
    "quilt_meta_loader", "quilt_meta_loader_game", "quilt_meta_installer",
)
HEX_LENGTHS = {"sha1": 40, "sha256": 64, "sha512": 128}
HASH_SOURCES = {"maven-sidecar", "mojang-version-json", "mojang-manifest", "modrinth-api", "observed"}
RUNTIME_NAMES = {"srg", "mojang-official", "intermediary"}
FABRIC_API_PROJECT = "P7dR8mSH"
QFAPI_PROJECT = "qvIfYCYJ"
FORGE_MAVEN = "https://maven.minecraftforge.net/net/minecraftforge/forge/"
QUILT_MAVEN = "https://maven.quiltmc.org/repository/release/org/quiltmc/"
METADATA_LIMIT = 8 * 1024 * 1024  # refuse to read anything bigger as "metadata"
LAUNCHER_LIMIT = 1024 * 1024
TIMEOUT = 60

_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
_MAVEN_RANGE = re.compile(r"^[\[(][^\[\]()]*[\])]$")
_GAME = re.compile(r"^1\.(\d+)(?:\.(\d+))?$")
_FORGE_VERSION = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")
_STABLE_VERSION = re.compile(r"^\d+(?:\.\d+)*$")


def neoforge_line(game: str) -> dict | None:
    """What a NeoForge pin looks like for a game version, or None when NeoForge has no build for it.

    1.20.1 is the Forge 47.1 fork published as net.neoforged:forge (Maven version '<game>-47.1.N',
    SRG member names at runtime, META-INF/mods.toml, dependency id 'forge'). From 1.20.2 on, NeoForge
    is net.neoforged:neoforge with versions '<minor>.<patch>.N' and Mojang names at runtime.
    """
    m = _GAME.match(game)
    if not m:
        return None
    minor, patch = int(m.group(1)), int(m.group(2) or 0)
    if (minor, patch) == (20, 1):
        return {
            "artifact": "net.neoforged:forge",
            "prefix": "47.1.",
            "maven": "net.neoforged:forge:{game}-{version}:installer",
            "installer": "forge-{game}-{version}-installer.jar",
            "listed_as": "{game}-{version}",
            "runtime_names": "srg",
            "manifest": "META-INF/mods.toml",
            "dependency_id": "forge",
            "listing_source": "neoforge_legacy_versions",
        }
    if (minor, patch) >= (20, 2):
        return {
            "artifact": "net.neoforged:neoforge",
            "prefix": f"{minor}.{patch}.",
            "maven": "net.neoforged:neoforge:{version}:installer",
            "installer": "neoforge-{version}-installer.jar",
            "listed_as": "{version}",
            "runtime_names": "mojang-official",
            "manifest": "META-INF/neoforge.mods.toml",
            "dependency_id": "neoforge",
            "listing_source": "neoforge_versions",
        }
    return None


# --------------------------------------------------------------------------
# Offline validation
# --------------------------------------------------------------------------


class _Errors:
    def __init__(self) -> None:
        self.items: list[str] = []

    def add(self, path: str, message: str) -> None:
        self.items.append(f"{path}: {message}")


def _get(node: Any, key: str, path: str, errors: _Errors, kind: type | tuple[type, ...] = dict) -> Any:
    if not isinstance(node, dict):
        errors.add(path, "expected an object")
        return None
    if key not in node:
        errors.add(f"{path}.{key}", "missing")
        return None
    value = node[key]
    if kind is not None and not isinstance(value, kind) or (kind is int and isinstance(value, bool)):
        name = kind.__name__ if isinstance(kind, type) else "/".join(k.__name__ for k in kind)
        errors.add(f"{path}.{key}", f"expected {name}, got {type(value).__name__}")
        return None
    return value


def _check_url(value: Any, path: str, errors: _Errors) -> None:
    if not isinstance(value, str):
        errors.add(path, "expected a URL string")
        return
    parsed = urllib.parse.urlparse(value)
    if parsed.scheme != "https" or not parsed.netloc:
        errors.add(path, f"must be an absolute https URL, got {value!r}")


def _check_hash(value: Any, algo: str, path: str, errors: _Errors) -> None:
    length = HEX_LENGTHS[algo]
    if not isinstance(value, str) or not re.fullmatch(f"[0-9a-f]{{{length}}}", value):
        errors.add(path, f"must be {length} lowercase hex characters ({algo})")


def _check_text(node: dict, key: str, path: str, errors: _Errors) -> str | None:
    value = _get(node, key, path, errors, str)
    if value is not None and not value.strip():
        errors.add(f"{path}.{key}", "must not be empty")
        return None
    return value


def _check_artifact(node: Any, path: str, errors: _Errors, algos: tuple[str, ...], source: str | None = None) -> None:
    if not isinstance(node, dict):
        errors.add(path, "expected an object")
        return
    _check_url(node.get("url"), f"{path}.url", errors)
    size = node.get("size")
    if not isinstance(size, int) or isinstance(size, bool) or size <= 0:
        errors.add(f"{path}.size", "must be a positive integer (bytes)")
    hs = node.get("hash_source")
    if hs not in HASH_SOURCES:
        errors.add(f"{path}.hash_source", f"must be one of {sorted(HASH_SOURCES)}")
    elif source is not None and hs != source:
        errors.add(f"{path}.hash_source", f"expected {source!r}, got {hs!r}")
    for algo in algos:
        if algo not in node:
            errors.add(f"{path}.{algo}", "missing")
        else:
            _check_hash(node[algo], algo, f"{path}.{algo}", errors)


def _check_mojang(node: Any, path: str, errors: _Errors) -> None:
    entry = _get(node, "manifest_entry", path, errors)
    if entry is not None:
        p = f"{path}.manifest_entry"
        _check_url(entry.get("url"), f"{p}.url", errors)
        _check_hash(entry.get("sha1"), "sha1", f"{p}.sha1", errors)
        if isinstance(entry.get("url"), str) and isinstance(entry.get("sha1"), str) and entry["sha1"] not in entry["url"]:
            errors.add(f"{p}.url", "Mojang package URLs are content-addressed; the URL must contain the sha1")
        if entry.get("type") != "release":
            errors.add(f"{p}.type", "pinned game versions must be releases")
        _check_text(entry, "release_time", p, errors)
        if entry.get("hash_source") != "mojang-manifest":
            errors.add(f"{p}.hash_source", "expected 'mojang-manifest'")
    java = _get(node, "java_major", path, errors, int)
    if java is not None and java < 8:
        errors.add(f"{path}.java_major", "implausible Java major version")
    for key in ("server_jar", "server_mappings", "client_mappings"):
        art = _get(node, key, path, errors)
        if art is None:
            continue
        _check_artifact(art, f"{path}.{key}", errors, ("sha1",), "mojang-version-json")
        url, sha1 = art.get("url"), art.get("sha1")
        if isinstance(url, str) and isinstance(sha1, str) and sha1 not in url:
            errors.add(f"{path}.{key}.url", "Mojang object URLs are content-addressed; the URL must contain the sha1")
        if key.endswith("mappings") and art.get("format") != "proguard":
            errors.add(f"{path}.{key}.format", "expected 'proguard'")


def _check_fabric(node: Any, game: str, path: str, errors: _Errors) -> None:
    loader = _get(node, "loader", path, errors)
    if loader is not None:
        _check_text(loader, "version", f"{path}.loader", errors)
        _check_text(loader, "maven", f"{path}.loader", errors)
        if loader.get("stable") is not True:
            errors.add(f"{path}.loader.stable", "the pinned Fabric Loader must be a stable release")
        _check_artifact(loader, f"{path}.loader", errors, ("sha1", "sha256", "sha512"), "maven-sidecar")

    inter = _get(node, "intermediary", path, errors)
    if inter is not None:
        if inter.get("version") != game:
            errors.add(f"{path}.intermediary.version", f"must equal the game version {game!r}")
        maven = _check_text(inter, "maven", f"{path}.intermediary", errors)
        if maven is not None and not maven.endswith(":v2"):
            errors.add(f"{path}.intermediary.maven", "must name the tiny-v2 classifier (':v2')")
        _check_artifact(inter, f"{path}.intermediary", errors, ("sha1", "sha256", "sha512"), "maven-sidecar")

    api = _get(node, "api", path, errors)
    if api is not None:
        p = f"{path}.api"
        version = _check_text(api, "version", p, errors)
        if version is not None and not version.endswith("+" + game):
            errors.add(f"{p}.version", f"Fabric API versions carry the game version as build metadata; expected '...+{game}'")
        if api.get("modrinth_project_id") != "P7dR8mSH":
            errors.add(f"{p}.modrinth_project_id", "expected the Fabric API project id 'P7dR8mSH'")
        _check_text(api, "modrinth_version_id", p, errors)
        _check_artifact(api, p, errors, ("sha1", "sha512"), "modrinth-api")
        _check_url(api.get("maven_url"), f"{p}.maven_url", errors)
        _check_hash(api.get("maven_sha1"), "sha1", f"{p}.maven_sha1", errors)
        if api.get("maven_sha1") != api.get("sha1"):
            errors.add(f"{p}.maven_sha1", "must equal the Modrinth sha1 (same file on two hosts)")

    inst = _get(node, "installer", path, errors)
    if inst is not None:
        _check_text(inst, "version", f"{path}.installer", errors)
        _check_artifact(inst, f"{path}.installer", errors, ("sha1", "sha256", "sha512"), "maven-sidecar")

    launcher = _get(node, "server_launcher", path, errors)
    if launcher is not None:
        p = f"{path}.server_launcher"
        pattern = _check_text(launcher, "url_pattern", p, errors)
        _check_artifact(launcher, p, errors, (), "observed")
        _check_hash(launcher.get("observed_sha256"), "sha256", f"{p}.observed_sha256", errors)
        if pattern is not None:
            for placeholder in ("{game_version}", "{loader_version}", "{installer_version}"):
                if placeholder not in pattern:
                    errors.add(f"{p}.url_pattern", f"missing placeholder {placeholder}")
            if loader is not None and inst is not None:
                try:
                    expected = pattern.format(
                        game_version=game, loader_version=loader.get("version"), installer_version=inst.get("version")
                    )
                except (KeyError, IndexError, ValueError):
                    expected = None
                if expected != launcher.get("url"):
                    errors.add(f"{p}.url", f"must be url_pattern filled with this game, loader and installer version ({expected})")

    harness = _get(node, "harness", path, errors)
    if harness is not None:
        if harness.get("method") not in ("server-launcher", "installer"):
            errors.add(f"{path}.harness.method", "must be 'server-launcher' or 'installer'")
        steps = harness.get("steps")
        if not isinstance(steps, list) or not steps or not all(isinstance(s, str) and s.strip() for s in steps):
            errors.add(f"{path}.harness.steps", "must be a non-empty list of strings")


def _check_neoforge(node: Any, game: str, path: str, errors: _Errors) -> None:
    if not isinstance(node, dict):
        errors.add(path, "expected an object")
        return
    line = neoforge_line(game)
    if line is None:
        errors.add(path, f"NeoForge has no build line for {game}; use null plus a 'neoforge_note'")
        return
    version = _check_text(node, "version", path, errors)
    prefix = line["prefix"]
    if version is not None and not version.startswith(prefix):
        errors.add(f"{path}.version", f"NeoForge versions for {game} start with {prefix!r}")
    if "artifact" in node or line["artifact"] != "net.neoforged:neoforge":
        if node.get("artifact") != line["artifact"]:
            errors.add(f"{path}.artifact", f"expected {line['artifact']!r} for {game}")
    if version is not None:
        expected_maven = line["maven"].format(game=game, version=version)
        if node.get("maven") != expected_maven:
            errors.add(f"{path}.maven", f"expected {expected_maven!r}")
        listed = line["listed_as"].format(game=game, version=version)
        if listed != version and node.get("maven_version") != listed:
            errors.add(f"{path}.maven_version", f"expected {listed!r}")
    beta = node.get("beta")
    channel = node.get("channel")
    if not isinstance(beta, bool):
        errors.add(f"{path}.beta", "must be true or false")
    if channel not in ("release", "beta"):
        errors.add(f"{path}.channel", "must be 'release' or 'beta'")
    elif isinstance(beta, bool) and beta != (channel == "beta"):
        errors.add(f"{path}.channel", "channel and beta disagree")
    if version is not None and isinstance(beta, bool) and beta != version.endswith("-beta"):
        errors.add(f"{path}.beta", "a '-beta' version must be flagged beta=true, and only then")
    inst = _get(node, "installer", path, errors)
    if inst is not None:
        _check_artifact(inst, f"{path}.installer", errors, ("sha1", "sha256", "sha512"), "maven-sidecar")
        jar = line["installer"].format(game=game, version=version)
        if version is not None and isinstance(inst.get("url"), str) and jar not in inst["url"]:
            errors.add(f"{path}.installer.url", "must point at this version's installer jar")
    if node.get("runtime_names") != line["runtime_names"]:
        errors.add(f"{path}.runtime_names", f"expected {line['runtime_names']!r} for {game}")
    _check_text(node, "install_command", path, errors)
    toml = _get(node, "mods_toml", path, errors)
    if toml is not None:
        p = f"{path}.mods_toml"
        if "file" in toml or line["manifest"] != "META-INF/neoforge.mods.toml":
            if toml.get("file") != line["manifest"]:
                errors.add(f"{p}.file", f"expected {line['manifest']!r} for {game}")
        if "loader_dependency_id" in toml or line["dependency_id"] != "neoforge":
            if toml.get("loader_dependency_id") != line["dependency_id"]:
                errors.add(f"{p}.loader_dependency_id", f"expected {line['dependency_id']!r} for {game}")
        if toml.get("modLoader") != "javafml":
            errors.add(f"{p}.modLoader", "expected 'javafml'")
        for key in ("loaderVersion", "neoforge_dependency_range", "minecraft_dependency_range"):
            value = _check_text(toml, key, p, errors)
            if value is not None and not _MAVEN_RANGE.match(value):
                errors.add(f"{p}.{key}", f"not a Maven version range: {value!r}")
        for key in ("modLoader_mandatory", "loaderVersion_mandatory"):
            if not isinstance(toml.get(key), bool):
                errors.add(f"{p}.{key}", "must be true or false")
        nf_range = toml.get("neoforge_dependency_range")
        if version is not None and isinstance(nf_range, str) and version not in nf_range:
            errors.add(f"{p}.neoforge_dependency_range", "the lower bound must be the pinned NeoForge version")


def _check_forge(node: Any, game: str, path: str, errors: _Errors) -> None:
    if not isinstance(node, dict):
        errors.add(path, "expected an object")
        return
    version = _check_text(node, "version", path, errors)
    major = None
    if version is not None:
        m = _FORGE_VERSION.match(version)
        if not m:
            errors.add(f"{path}.version", "Forge versions look like <major>.<minor>.<build>")
        else:
            major = m.group(1)
    mv = f"{game}-{version}"
    if version is not None and node.get("maven_version") != mv:
        errors.add(f"{path}.maven_version", f"expected {mv!r}")
    promotion = node.get("promotion")
    if promotion not in ("recommended", "latest"):
        errors.add(f"{path}.promotion", "must be 'recommended' or 'latest' (promotions_slim.json)")
    else:
        if node.get("promotions_key") != f"{game}-{promotion}":
            errors.add(f"{path}.promotions_key", f"expected {game}-{promotion!s}")
        if promotion == "latest":
            _check_text(node, "promotion_note", path, errors)
    if version is not None and node.get("maven") != f"net.minecraftforge:forge:{mv}:installer":
        errors.add(f"{path}.maven", f"expected 'net.minecraftforge:forge:{mv}:installer'")
    inst = _get(node, "installer", path, errors)
    if inst is not None:
        _check_artifact(inst, f"{path}.installer", errors, ("sha1", "sha256", "sha512"), "maven-sidecar")
        expected = f"{FORGE_MAVEN}{mv}/forge-{mv}-installer.jar"
        if version is not None and inst.get("url") != expected:
            errors.add(f"{path}.installer.url", f"expected {expected}")
    if node.get("runtime_names") not in ("srg", "mojang-official"):
        errors.add(f"{path}.runtime_names", "must be 'srg' or 'mojang-official', as read from the installer's install_profile.json")
    for key in ("runtime_names_evidence", "eventbus", "mixin"):
        _check_text(node, key, path, errors)
    command = _check_text(node, "install_command", path, errors)
    if command is not None and version is not None and (f"forge-{mv}-installer.jar" not in command or "--installServer" not in command):
        errors.add(f"{path}.install_command", "must run this version's installer with --installServer")
    toml = _get(node, "mods_toml", path, errors)
    if toml is not None:
        p = f"{path}.mods_toml"
        if toml.get("file") != "META-INF/mods.toml":
            errors.add(f"{p}.file", "expected 'META-INF/mods.toml'")
        if toml.get("modLoader") != "javafml":
            errors.add(f"{p}.modLoader", "expected 'javafml'")
        for key in ("loaderVersion", "forge_dependency_range", "minecraft_dependency_range"):
            value = _check_text(toml, key, p, errors)
            if value is not None and not _MAVEN_RANGE.match(value):
                errors.add(f"{p}.{key}", f"not a Maven version range: {value!r}")
        if major is not None and toml.get("loaderVersion") != f"[{major},)":
            errors.add(f"{p}.loaderVersion", f"javafml's version is the Forge major version; expected '[{major},)'")
        fr = toml.get("forge_dependency_range")
        if version is not None and isinstance(fr, str) and version not in fr:
            errors.add(f"{p}.forge_dependency_range", "the lower bound must be the pinned Forge version")


def _check_quilt(node: Any, game: str, fabric: Any, path: str, errors: _Errors) -> None:
    if not isinstance(node, dict):
        errors.add(path, "expected an object")
        return
    loader = _get(node, "loader", path, errors)
    if loader is not None:
        p = f"{path}.loader"
        version = _check_text(loader, "version", p, errors)
        if version is not None and not _STABLE_VERSION.match(version):
            errors.add(f"{p}.version", "the pinned Quilt Loader must not be a pre-release (no '-beta.N')")
        if loader.get("stable") is not True:
            errors.add(f"{p}.stable", "the pinned Quilt Loader must be a stable release")
        if version is not None and loader.get("maven") != f"org.quiltmc:quilt-loader:{version}":
            errors.add(f"{p}.maven", f"expected 'org.quiltmc:quilt-loader:{version}'")
        _check_artifact(loader, p, errors, ("sha1", "sha256", "sha512"), "maven-sidecar")
        if isinstance(loader.get("url"), str) and not loader["url"].startswith(QUILT_MAVEN + "quilt-loader/"):
            errors.add(f"{p}.url", f"must be on {QUILT_MAVEN}quilt-loader/")
        _check_text(loader, "provides_fabricloader", p, errors)
    if node.get("runtime_names") != "intermediary":
        errors.add(f"{path}.runtime_names", "expected 'intermediary' (Quilt Loader's production namespace)")
    if node.get("intermediary") != f"net.fabricmc:intermediary:{game}":
        errors.add(f"{path}.intermediary", f"expected 'net.fabricmc:intermediary:{game}'")
    elif isinstance(fabric, dict) and isinstance(fabric.get("intermediary"), dict):
        if fabric["intermediary"].get("maven") != node["intermediary"] + ":v2":
            errors.add(f"{path}.intermediary", "must be the same intermediary as fabric.intermediary")
    _check_text(node, "runtime_names_evidence", path, errors)

    api = _get(node, "api", path, errors)
    if api is not None:
        p = f"{path}.api"
        _check_text(api, "name", p, errors)
        _check_url(api.get("source"), f"{p}.source", errors)
        kind = api.get("kind")
        if kind == "qfapi":
            version = _check_text(api, "version", p, errors)
            if version is not None and not version.endswith("-" + game):
                errors.add(f"{p}.version", f"QFAPI versions end with the game version; expected '...-{game}'")
            if api.get("modrinth_project_id") != QFAPI_PROJECT:
                errors.add(f"{p}.modrinth_project_id", f"expected the QFAPI/QSL project id {QFAPI_PROJECT!r}")
            _check_text(api, "modrinth_version_id", p, errors)
            _check_artifact(api, p, errors, ("sha1", "sha512"), "modrinth-api")
        elif kind == "fabric-api":
            if api.get("same_as") != "fabric.api":
                errors.add(f"{p}.same_as", "expected 'fabric.api'")
            fabric_api = fabric.get("api") if isinstance(fabric, dict) else None
            if not isinstance(fabric_api, dict) or api.get("version") != fabric_api.get("version"):
                errors.add(f"{p}.version", "must equal the pinned fabric.api version")
            _check_text(api, "why", p, errors)
        else:
            errors.add(f"{p}.kind", "must be 'qfapi' or 'fabric-api'")

    inst = _get(node, "installer", path, errors)
    if inst is not None:
        p = f"{path}.installer"
        version = _check_text(inst, "version", p, errors)
        if version is not None and inst.get("maven") != f"org.quiltmc:quilt-installer:{version}":
            errors.add(f"{p}.maven", f"expected 'org.quiltmc:quilt-installer:{version}'")
        _check_artifact(inst, p, errors, ("sha1", "sha256", "sha512"), "maven-sidecar")

    harness = _get(node, "harness", path, errors)
    if harness is not None:
        if harness.get("method") != "installer":
            errors.add(f"{path}.harness.method", "expected 'installer'")
        steps = harness.get("steps")
        if not isinstance(steps, list) or not steps or not all(isinstance(s, str) and s.strip() for s in steps):
            errors.add(f"{path}.harness.steps", "must be a non-empty list of strings")


def _check_parchment(node: Any, note: Any, game: str, path: str, errors: _Errors) -> None:
    if node is None:
        if not isinstance(note, str) or not note.strip():
            errors.add(path, f"null Parchment for {game} needs a non-empty sibling 'parchment_note' saying why")
        return
    if not isinstance(node, dict):
        errors.add(path, "expected an object or null")
        return
    version = _check_text(node, "version", path, errors)
    maven = _check_text(node, "maven", path, errors)
    if maven is not None and f"parchment-{game}:" not in maven:
        errors.add(f"{path}.maven", f"must be the parchment-{game} artifact")
    if version is not None and not re.fullmatch(r"\d{4}\.\d{2}\.\d{2}", version):
        errors.add(f"{path}.version", "Parchment release versions are dates (YYYY.MM.DD)")
    _check_url(node.get("metadata_url"), f"{path}.metadata_url", errors)
    _check_artifact(node, path, errors, ("sha1", "sha256", "sha512"), "maven-sidecar")


def validate(lock: Any) -> list[str]:
    """Return a list of schema errors; an empty list means the lock is well formed."""
    errors = _Errors()
    if not isinstance(lock, dict):
        return ["$: the lock must be a JSON object"]
    if lock.get("schema_version") != 1:
        errors.add("$.schema_version", "expected 1")
    locked_at = _get(lock, "locked_at", "$", errors, str)
    if locked_at is not None and not _DATE.match(locked_at):
        errors.add("$.locked_at", "expected YYYY-MM-DD")
    _check_text(lock, "user_agent", "$", errors)
    jdk = _get(lock, "jdk", "$", errors)
    if jdk is not None:
        _get(jdk, "major", "$.jdk", errors, int)
    sources = _get(lock, "sources", "$", errors)
    if sources is not None:
        if not sources:
            errors.add("$.sources", "must list the metadata sources")
        for key in REQUIRED_SOURCES:
            if key not in sources:
                errors.add(f"$.sources.{key}", "missing (--verify reads it)")
        for key, url in sources.items():
            _check_url(url, f"$.sources.{key}", errors)
        pattern = sources.get("quilt_meta_loader_game")
        if isinstance(pattern, str) and "{game_version}" not in pattern:
            errors.add("$.sources.quilt_meta_loader_game", "missing placeholder {game_version}")
    glob = _get(lock, "global", "$", errors)
    if glob is not None:
        vf = _get(glob, "vineflower", "$.global", errors)
        if vf is not None:
            _check_text(vf, "version", "$.global.vineflower", errors)
            _check_text(vf, "maven", "$.global.vineflower", errors)
            _check_url(vf.get("metadata_url"), "$.global.vineflower.metadata_url", errors)
            _check_artifact(vf, "$.global.vineflower", errors, ("sha1",), "maven-sidecar")
    notes = _get(lock, "notes", "$", errors, list)
    if notes is not None and not all(isinstance(n, str) and n.strip() for n in notes):
        errors.add("$.notes", "every note must be a non-empty string")

    games = _get(lock, "game_versions", "$", errors)
    if games is not None:
        for game in REQUIRED_GAME_VERSIONS:
            if game not in games:
                errors.add(f"$.game_versions.{game}", f"missing (all pinned game versions are required: {', '.join(REQUIRED_GAME_VERSIONS)})")
        by_game = jdk.get("by_game_version") if isinstance(jdk, dict) else None
        if by_game is not None and not isinstance(by_game, dict):
            errors.add("$.jdk.by_game_version", "expected an object of game version -> Java major")
            by_game = None
        for game, entry in games.items():
            path = f"$.game_versions.{game}"
            if not _GAME.match(game):
                errors.add(path, "game version keys look like 1.21.x")
            if not isinstance(entry, dict):
                errors.add(path, "expected an object")
                continue
            if "mojang" not in entry:
                errors.add(f"{path}.mojang", "missing")
            else:
                _check_mojang(entry["mojang"], f"{path}.mojang", errors)
                if jdk is not None and isinstance(entry["mojang"], dict):
                    java = entry["mojang"].get("java_major")
                    if isinstance(java, int) and isinstance(jdk.get("major"), int) and java > jdk["major"]:
                        errors.add(f"{path}.mojang.java_major", "needs a newer JDK than $.jdk.major")
                    if by_game is not None and isinstance(java, int) and by_game.get(game) != java:
                        errors.add(f"$.jdk.by_game_version.{game}", f"must equal the Mojang javaVersion.majorVersion ({java})")
            for key in LOADERS:
                if key not in entry:
                    errors.add(f"{path}.{key}", f"missing (use null plus a '{key}_note' when no build exists)")
                    continue
                if entry[key] is None:
                    note = entry.get(f"{key}_note")
                    if not isinstance(note, str) or not note.strip():
                        errors.add(f"{path}.{key}", f"null {key} for {game} needs a non-empty sibling '{key}_note' saying why")
                    continue
                if key == "fabric":
                    _check_fabric(entry[key], game, f"{path}.{key}", errors)
                elif key == "neoforge":
                    _check_neoforge(entry[key], game, f"{path}.{key}", errors)
                elif key == "forge":
                    _check_forge(entry[key], game, f"{path}.{key}", errors)
                else:
                    _check_quilt(entry[key], game, entry.get("fabric"), f"{path}.{key}", errors)
            if "parchment" not in entry:
                errors.add(f"{path}.parchment", "missing (use null plus a 'parchment_note' when none exists)")
            else:
                _check_parchment(entry["parchment"], entry.get("parchment_note"), game, f"{path}.parchment", errors)
    return errors.items


def load_lock(path: Path) -> Any:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


# --------------------------------------------------------------------------
# Network helpers (used by --verify only)
# --------------------------------------------------------------------------


class _KeepHeadRedirect(urllib.request.HTTPRedirectHandler):
    """Follow redirects without turning a HEAD into a full GET."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: D401 - urllib signature
        new = super().redirect_request(req, fp, code, msg, headers, newurl)
        if new is not None:
            new.method = req.get_method()
            for key, value in req.header_items():
                if key.lower() == "range":
                    new.add_header(key, value)
        return new


_OPENER = urllib.request.build_opener(_KeepHeadRedirect)


class NetError(Exception):
    pass


def _open(url: str, method: str = "GET", headers: dict | None = None):
    hdrs = {"User-Agent": USER_AGENT}
    hdrs.update(headers or {})
    last: Exception | None = None
    for attempt in range(4):
        req = urllib.request.Request(url, method=method, headers=hdrs)
        try:
            return _OPENER.open(req, timeout=TIMEOUT)
        except urllib.error.HTTPError as err:
            if err.code in (429, 500, 502, 503, 504) and attempt < 3:
                last = err
                time.sleep(2**attempt)
                continue
            raise
        except (urllib.error.URLError, TimeoutError, ConnectionError) as err:
            last = err
            if attempt < 3:
                time.sleep(2**attempt)
                continue
    raise NetError(f"{url}: {last}")


def fetch(url: str, limit: int = METADATA_LIMIT) -> bytes:
    """GET a small resource; refuses anything larger than `limit` bytes."""
    try:
        with _open(url) as resp:
            length = resp.headers.get("Content-Length")
            if length is not None and int(length) > limit:
                raise NetError(f"{url}: {length} bytes is larger than the {limit}-byte metadata limit")
            data = resp.read(limit + 1)
    except urllib.error.HTTPError as err:
        raise NetError(f"{url}: HTTP {err.code}") from err
    if len(data) > limit:
        raise NetError(f"{url}: response is larger than the {limit}-byte metadata limit")
    return data


def fetch_json(url: str) -> Any:
    return json.loads(fetch(url).decode("utf-8"))


@dataclass
class Probe:
    status: int
    size: int | None
    content_type: str | None


def probe(url: str) -> Probe:
    """Reachability check without downloading: HEAD, else a 1-byte ranged GET."""
    try:
        with _open(url, method="HEAD") as resp:
            length = resp.headers.get("Content-Length")
            return Probe(resp.status, int(length) if length else None, resp.headers.get("Content-Type"))
    except urllib.error.HTTPError as err:
        if err.code not in (403, 405, 501):
            return Probe(err.code, None, None)
    except NetError:
        pass
    try:
        with _open(url, headers={"Range": "bytes=0-0"}) as resp:
            total = None
            content_range = resp.headers.get("Content-Range", "")
            if "/" in content_range and content_range.rsplit("/", 1)[1].isdigit():
                total = int(content_range.rsplit("/", 1)[1])
            elif resp.status == 200 and resp.headers.get("Content-Length"):
                total = int(resp.headers["Content-Length"])
            resp.read(1)
            return Probe(resp.status, total, resp.headers.get("Content-Type"))
    except urllib.error.HTTPError as err:
        return Probe(err.code, None, None)


def parse_sidecar(text: str) -> str:
    """A Maven .sha1/.sha256/.sha512 file holds the hex digest, sometimes followed by a file name."""
    token = text.strip().split()[0] if text.strip() else ""
    return token.lower()


# --------------------------------------------------------------------------
# Verification
# --------------------------------------------------------------------------


@dataclass
class Result:
    status: str  # OK, WARN, FAIL, INFO
    subject: str
    message: str


@dataclass
class Report:
    results: list[Result] = field(default_factory=list)

    def add(self, status: str, subject: str, message: str) -> None:
        self.results.append(Result(status, subject, message))

    def count(self, status: str) -> int:
        return sum(1 for r in self.results if r.status == status)


class _Verifier:
    def __init__(self, lock: dict, report: Report) -> None:
        self.lock = lock
        self.report = report
        self._cache: dict[str, Any] = {}

    def cached_json(self, url: str) -> Any:
        if url not in self._cache:
            self._cache[url] = fetch_json(url)
        return self._cache[url]

    def guard(self, subject: str, fn: Callable[[], None]) -> None:
        try:
            fn()
        except (NetError, urllib.error.URLError, ValueError, KeyError, IndexError, TypeError) as err:
            self.report.add("FAIL", subject, f"could not check: {err}")

    # --- shared checks -----------------------------------------------------

    def reachable(self, subject: str, art: dict) -> None:
        p = probe(art["url"])
        if p.status not in (200, 206):
            self.report.add("FAIL", subject, f"HTTP {p.status} for {art['url']}")
        elif p.size is not None and p.size != art["size"]:
            self.report.add("FAIL", subject, f"size {p.size} != pinned {art['size']}")
        elif p.size is None:
            self.report.add("WARN", subject, "reachable, but the server did not report a size")
        else:
            self.report.add("OK", subject, f"reachable, {p.size} bytes")

    def sidecars(self, subject: str, art: dict, algos: tuple[str, ...] = ("sha1", "sha256", "sha512"), url: str | None = None) -> None:
        base = url or art["url"]
        for algo in algos:
            if algo not in art:
                continue
            published = parse_sidecar(fetch(base + "." + algo, limit=4096).decode("ascii", "replace"))
            if published == art[algo]:
                self.report.add("OK", subject, f"{algo} matches the publisher's .{algo} file")
            else:
                self.report.add("FAIL", subject, f"{algo} mismatch: publisher {published}, pinned {art[algo]}")

    def modrinth_file(self, subject: str, api: dict, game: str, loader: str) -> None:
        """Compare a pinned Modrinth version (id, version, game, loader, primary file) with the live API."""
        live = fetch_json(f"{self.lock['sources']['modrinth_api']}/version/{api['modrinth_version_id']}")
        problems = []
        if live.get("project_id") != api["modrinth_project_id"]:
            problems.append(f"project {live.get('project_id')}")
        if live.get("version_number") != api["version"]:
            problems.append(f"version {live.get('version_number')}")
        if game not in live.get("game_versions", []):
            problems.append(f"game_versions {live.get('game_versions')}")
        if loader not in live.get("loaders", []):
            problems.append(f"loaders {live.get('loaders')}")
        primary = next((f for f in live.get("files", []) if f.get("primary")), None)
        if primary is None:
            problems.append("no primary file")
        else:
            if primary.get("url") != api["url"]:
                problems.append(f"url {primary.get('url')}")
            if primary.get("size") != api["size"]:
                problems.append(f"size {primary.get('size')}")
            for algo in ("sha1", "sha512"):
                if primary.get("hashes", {}).get(algo) != api[algo]:
                    problems.append(f"{algo} {primary.get('hashes', {}).get(algo)}")
        if problems:
            self.report.add("FAIL", subject, "Modrinth disagrees: " + "; ".join(problems))
        else:
            self.report.add("OK", subject, "Modrinth version, file url, size, sha1 and sha512 match")

    def modrinth_releases(self, project: str, game: str, loader: str) -> list[dict]:
        """Release versions of a Modrinth project for one game version and loader, newest first."""
        params = urllib.parse.urlencode({"game_versions": json.dumps([game]), "loaders": json.dumps([loader])})
        versions = fetch_json(f"{self.lock['sources']['modrinth_api']}/project/{project}/version?{params}")
        return [v for v in versions if v.get("version_type") == "release"]

    # --- per source ---------------------------------------------------------

    def mojang(self, game: str, node: dict) -> None:
        subject = f"{game} mojang"
        manifest = self.cached_json(self.lock["sources"]["mojang_version_manifest"])
        entries = {v["id"]: v for v in manifest["versions"]}
        pinned = node["manifest_entry"]
        live = entries.get(game)
        if live is None:
            self.report.add("FAIL", subject, "game version no longer listed in the version manifest")
        elif live["url"] == pinned["url"] and live["sha1"] == pinned["sha1"]:
            self.report.add("OK", subject, "manifest entry url and sha1 match")
        else:
            self.report.add("WARN", subject, f"manifest drift: Mojang now lists {live['sha1']} (pinned {pinned['sha1']}); checking the pinned file")
        raw = fetch(pinned["url"])
        digest = hashlib.sha1(raw).hexdigest()
        if digest != pinned["sha1"]:
            self.report.add("FAIL", subject, f"version JSON sha1 {digest} != pinned {pinned['sha1']}")
            return
        self.report.add("OK", subject, "version JSON sha1 matches the manifest entry")
        vj = json.loads(raw.decode("utf-8"))
        java = vj.get("javaVersion", {}).get("majorVersion")
        if java == node["java_major"]:
            self.report.add("OK", subject, f"javaVersion.majorVersion is {java}")
        else:
            self.report.add("FAIL", subject, f"javaVersion.majorVersion {java} != pinned {node['java_major']}")
        for key, download in (("server_jar", "server"), ("server_mappings", "server_mappings"), ("client_mappings", "client_mappings")):
            art = node[key]
            live_dl = vj["downloads"].get(download, {})
            same = all(live_dl.get(f) == art[f] for f in ("url", "sha1", "size"))
            self.report.add("OK" if same else "FAIL", f"{subject} {key}",
                            "url, sha1 and size match the version JSON" if same else f"version JSON has {live_dl}")
            self.reachable(f"{subject} {key}", art)

    def fabric(self, game: str, node: dict) -> None:
        loader = node["loader"]
        subject = f"{game} fabric.loader"
        loaders = self.cached_json(self.lock["sources"]["fabric_meta_loader"])
        listed = [v for v in loaders if v["version"] == loader["version"]]
        if not listed:
            self.report.add("FAIL", subject, f"{loader['version']} not listed by Fabric meta")
        elif not listed[0].get("stable"):
            self.report.add("FAIL", subject, f"{loader['version']} is no longer marked stable")
        else:
            self.report.add("OK", subject, f"{loader['version']} listed and stable")
            newest = next((v["version"] for v in loaders if v.get("stable")), None)
            if newest and newest != loader["version"]:
                self.report.add("INFO", subject, f"a newer stable loader exists: {newest} (re-lock needs a D- entry)")
        profile = fetch_json(f"https://meta.fabricmc.net/v2/versions/loader/{game}/{loader['version']}")
        if profile.get("intermediary", {}).get("version") == game:
            self.report.add("OK", subject, f"meta serves a {game} launch profile for this loader")
        else:
            self.report.add("FAIL", subject, f"meta has no {game} profile for loader {loader['version']}")
        self.sidecars(subject, loader)
        self.reachable(subject, loader)

        inter = node["intermediary"]
        subject = f"{game} fabric.intermediary"
        listed = fetch_json(self.lock["sources"]["fabric_meta_intermediary"].format(game_version=game))
        coordinate = inter["maven"].rsplit(":", 1)[0]
        if any(v.get("maven") == coordinate for v in listed):
            self.report.add("OK", subject, f"{coordinate} listed by Fabric meta")
        else:
            self.report.add("FAIL", subject, f"{coordinate} not listed by Fabric meta")
        self.sidecars(subject, inter)
        self.reachable(subject, inter)

        api = node["api"]
        subject = f"{game} fabric.api"
        self.modrinth_file(subject, api, game, "fabric")
        published = parse_sidecar(fetch(api["maven_url"] + ".sha1", limit=4096).decode("ascii", "replace"))
        self.report.add("OK" if published == api["maven_sha1"] else "FAIL", subject,
                        "Fabric maven .sha1 matches" if published == api["maven_sha1"] else f"Fabric maven .sha1 is {published}")
        self.reachable(subject, api)
        versions = self.modrinth_releases(api["modrinth_project_id"], game, "fabric")
        newest = next((v["version_number"] for v in versions), None)
        if newest and newest != api["version"]:
            self.report.add("INFO", subject, f"a newer Fabric API release exists for {game}: {newest}")

        inst = node["installer"]
        subject = f"{game} fabric.installer"
        installers = self.cached_json(self.lock["sources"]["fabric_meta_installer"])
        if any(v.get("version") == inst["version"] for v in installers):
            self.report.add("OK", subject, f"{inst['version']} listed by Fabric meta")
        else:
            self.report.add("FAIL", subject, f"{inst['version']} not listed by Fabric meta")
        self.sidecars(subject, inst)
        self.reachable(subject, inst)

        launcher = node["server_launcher"]
        subject = f"{game} fabric.server_launcher"
        data = fetch(launcher["url"], limit=LAUNCHER_LIMIT)
        digest = hashlib.sha256(data).hexdigest()
        if digest == launcher["observed_sha256"] and len(data) == launcher["size"]:
            self.report.add("OK", subject, "generated launcher matches the observed sha256 and size")
        else:
            self.report.add("WARN", subject, f"generated launcher changed (sha256 {digest}, {len(data)} bytes); harness falls back to the pinned installer")

    def neoforge(self, game: str, node: dict) -> None:
        subject = f"{game} neoforge"
        line = neoforge_line(game)
        if line is None:
            self.report.add("FAIL", subject, f"NeoForge has no build line for {game}")
            return
        listing = self.cached_json(self.lock["sources"][line["listing_source"]])
        versions = listing.get("versions", [])
        listed = line["listed_as"].format(game=game, version=node["version"])
        if listed not in versions:
            self.report.add("FAIL", subject, f"{listed} not listed by the NeoForge maven API ({line['artifact']})")
        else:
            self.report.add("OK", subject, f"{listed} listed by the NeoForge maven API ({line['artifact']})")
            prefix = listed.rsplit(".", 1)[0] + "."

            def build(v: str) -> int:
                tail = v[len(prefix):].split("-", 1)[0]
                return int(tail) if tail.isdigit() else -1

            same_line = [v for v in versions if v.startswith(prefix) and build(v) >= 0]
            stable = [v for v in same_line if not v.endswith("-beta")]
            pool = stable if stable else same_line
            newest = max(pool, key=build) if pool else None
            if newest and newest != listed:
                self.report.add("INFO", subject, f"a newer {'stable' if stable else 'beta'} build exists: {newest}")
            if node["beta"] and stable:
                self.report.add("WARN", subject, "pinned a beta although a stable build now exists")
        self.sidecars(subject, node["installer"])
        self.reachable(subject, node["installer"])

    def forge(self, game: str, node: dict) -> None:
        subject = f"{game} forge"
        promos = self.cached_json(self.lock["sources"]["forge_promotions"]).get("promos", {})
        key = node["promotions_key"]
        live = promos.get(key)
        if live is None:
            self.report.add("FAIL", subject, f"promotions_slim.json has no {key!r}")
        elif live == node["version"]:
            self.report.add("OK", subject, f"{key} is {live}, the pinned build")
        else:
            self.report.add("INFO", subject, f"{key} is now {live}, pinned {node['version']} (re-lock needs a D- entry)")
        if node["promotion"] == "latest" and promos.get(f"{game}-recommended"):
            self.report.add("INFO", subject, f"a recommended build now exists: {promos[f'{game}-recommended']}")
        latest = promos.get(f"{game}-latest")
        if node["promotion"] == "recommended" and latest and latest != node["version"]:
            self.report.add("INFO", subject, f"a newer build that is not recommended exists: {latest}")
        self.sidecars(subject, node["installer"])
        self.reachable(subject, node["installer"])

    def quilt(self, game: str, node: dict, fabric: dict | None) -> None:
        src = self.lock["sources"]
        loader = node["loader"]
        subject = f"{game} quilt.loader"
        listing = self.cached_json(src["quilt_meta_loader"])
        entry = next((x for x in listing if x.get("version") == loader["version"]), None)
        if entry is None:
            self.report.add("FAIL", subject, f"{loader['version']} not listed by Quilt meta")
        else:
            problems = []
            if entry.get("file_size") != loader["size"]:
                problems.append(f"file_size {entry.get('file_size')}")
            for algo in ("sha1", "sha256", "sha512"):
                if entry.get("hashes", {}).get(algo) != loader[algo]:
                    problems.append(f"{algo} {entry.get('hashes', {}).get(algo)}")
            if problems:
                self.report.add("FAIL", subject, "Quilt meta disagrees: " + "; ".join(problems))
            else:
                self.report.add("OK", subject, f"{loader['version']} listed by Quilt meta with the pinned size and hashes")
            stable = [x["version"] for x in listing if _STABLE_VERSION.match(str(x.get("version", "")))]
            newest = max(stable, key=lambda v: tuple(int(p) for p in v.split(".")), default=None)
            if newest and newest != loader["version"]:
                self.report.add("INFO", subject, f"a newer stable Quilt Loader exists: {newest} (re-lock needs a D- entry)")
        per_game = fetch_json(src["quilt_meta_loader_game"].format(game_version=game))
        profile = next((x for x in per_game if x.get("loader", {}).get("version") == loader["version"]), None)
        if profile is None:
            self.report.add("FAIL", subject, f"Quilt meta has no {game} profile for loader {loader['version']}")
        elif profile.get("intermediary", {}).get("maven") != node["intermediary"]:
            self.report.add("FAIL", subject, f"Quilt meta pairs it with {profile.get('intermediary')}, pinned {node['intermediary']}")
        else:
            self.report.add("OK", subject, f"meta pairs loader {loader['version']} with {node['intermediary']}")
        self.sidecars(subject, loader)
        self.reachable(subject, loader)

        api = node["api"]
        subject = f"{game} quilt.api"
        if api["kind"] == "qfapi":
            self.modrinth_file(subject, api, game, "quilt")
            self.reachable(subject, api)
            newest = next((v["version_number"] for v in self.modrinth_releases(QFAPI_PROJECT, game, "quilt")), None)
            if newest and newest != api["version"]:
                self.report.add("INFO", subject, f"a newer QFAPI release exists for {game}: {newest}")
        else:
            releases = self.modrinth_releases(QFAPI_PROJECT, game, "quilt")
            if releases:
                self.report.add("WARN", subject, f"a QFAPI release now exists for {game} ({releases[0]['version_number']}); the lock still uses Fabric API")
            else:
                self.report.add("OK", subject, f"Modrinth lists no QFAPI release for {game}; Quilt uses the pinned Fabric API")
            fabric_version = (fabric or {}).get("api", {}).get("version")
            if fabric_version != api["version"]:
                self.report.add("FAIL", subject, f"pins Fabric API {api['version']}, fabric.api is {fabric_version}")

        inst = node["installer"]
        subject = f"{game} quilt.installer"
        installers = self.cached_json(src["quilt_meta_installer"])
        entry = next((x for x in installers if x.get("version") == inst["version"]), None)
        if entry is None:
            self.report.add("FAIL", subject, f"{inst['version']} not listed by Quilt meta")
        else:
            same = entry.get("file_size") == inst["size"] and all(entry.get("hashes", {}).get(a) == inst[a] for a in ("sha1", "sha256", "sha512"))
            self.report.add("OK" if same else "FAIL", subject,
                            f"{inst['version']} listed by Quilt meta with the pinned size and hashes" if same else f"Quilt meta disagrees: {entry}")
            if installers and installers[0].get("version") != inst["version"]:
                self.report.add("INFO", subject, f"Quilt meta's newest installer is {installers[0].get('version')}")
        self.sidecars(subject, inst)
        self.reachable(subject, inst)

    def parchment(self, game: str, node: dict | None) -> None:
        subject = f"{game} parchment"
        if node is None:
            self.report.add("INFO", subject, "no Parchment pinned (see notes)")
            return
        xml = fetch(node["metadata_url"]).decode("utf-8", "replace")
        if f"<version>{node['version']}</version>" in xml:
            self.report.add("OK", subject, f"{node['version']} listed in maven-metadata.xml")
        else:
            self.report.add("FAIL", subject, f"{node['version']} not in maven-metadata.xml")
        release = re.search(r"<release>([^<]+)</release>", xml)
        if release and release.group(1) != node["version"]:
            self.report.add("INFO", subject, f"Parchment's current release is {release.group(1)}")
        self.sidecars(subject, node)
        self.reachable(subject, node)

    def vineflower(self, node: dict) -> None:
        subject = "global vineflower"
        xml = fetch(node["metadata_url"]).decode("utf-8", "replace")
        if f"<version>{node['version']}</version>" in xml:
            self.report.add("OK", subject, f"{node['version']} listed on Maven Central")
        else:
            self.report.add("FAIL", subject, f"{node['version']} not listed on Maven Central")
        release = re.search(r"<release>([^<]+)</release>", xml)
        if release and release.group(1) != node["version"]:
            self.report.add("INFO", subject, f"Maven Central's current release is {release.group(1)}")
        self.sidecars(subject, node)
        self.reachable(subject, node)


def verify(lock: dict, only: str | None = None) -> Report:
    report = Report()
    v = _Verifier(lock, report)
    for game, entry in lock["game_versions"].items():
        if only and game != only:
            continue
        v.guard(f"{game} mojang", lambda: v.mojang(game, entry["mojang"]))
        for key, check in (
            ("fabric", lambda: v.fabric(game, entry["fabric"])),
            ("neoforge", lambda: v.neoforge(game, entry["neoforge"])),
            ("forge", lambda: v.forge(game, entry["forge"])),
            ("quilt", lambda: v.quilt(game, entry["quilt"], entry.get("fabric"))),
        ):
            if entry.get(key) is None:
                report.add("INFO", f"{game} {key}", f"no {key} pinned ({key}_note: {entry.get(f'{key}_note')})")
            else:
                v.guard(f"{game} {key}", check)
        v.guard(f"{game} parchment", lambda: v.parchment(game, entry.get("parchment")))
    if not only:
        v.guard("global vineflower", lambda: v.vineflower(lock["global"]["vineflower"]))
    return report


# --------------------------------------------------------------------------
# Printing
# --------------------------------------------------------------------------


def summary_lines(lock: dict) -> list[str]:
    def short(art: dict) -> str:
        for algo in ("sha256", "sha1", "sha512"):
            if algo in art:
                return f"{algo}:{art[algo][:12]}"
        if "observed_sha256" in art:
            return f"observed sha256:{art['observed_sha256'][:12]}"
        return "-"

    lines = [f"research-lock.json  locked {lock['locked_at']}  JDK {lock['jdk']['major']}", ""]
    for game, e in lock["game_versions"].items():
        lines.append(f"Minecraft {game}  (Java {e['mojang']['java_major']}, released {e['mojang']['manifest_entry']['release_time'][:10]})")
        rows = [
            ("vanilla server", game, e["mojang"]["server_jar"]),
            ("server mappings", "proguard", e["mojang"]["server_mappings"]),
            ("client mappings", "proguard", e["mojang"]["client_mappings"]),
        ]
        fb, nf, fg, qu = (e.get(k) for k in LOADERS)
        if fb:
            rows += [
                ("fabric loader", fb["loader"]["version"], fb["loader"]),
                ("intermediary", fb["intermediary"]["version"], fb["intermediary"]),
                ("fabric api", fb["api"]["version"], fb["api"]),
                ("fabric installer", fb["installer"]["version"], fb["installer"]),
                ("server launcher", "meta-generated", fb["server_launcher"]),
            ]
        if nf:
            rows.append(("neoforge", nf["version"] + (" (beta)" if nf["beta"] else ""), nf["installer"]))
        if fg:
            rows.append(("forge", f"{fg['version']} ({fg['promotion']})", fg["installer"]))
        if qu:
            rows.append(("quilt loader", qu["loader"]["version"], qu["loader"]))
            if qu["api"]["kind"] == "qfapi":
                rows.append(("quilt api (qfapi)", qu["api"]["version"], qu["api"]))
            rows.append(("quilt installer", qu["installer"]["version"], qu["installer"]))
        if e.get("parchment"):
            rows.append(("parchment", e["parchment"]["version"], e["parchment"]))
        for name, version, art in rows:
            lines.append(f"  {name:<17} {version:<22} {art['size']:>10,} B  {short(art)}")
        for key in LOADERS:
            if not e.get(key):
                lines.append(f"  {key:<17} none ({key}_note)")
        if qu and qu["api"]["kind"] == "fabric-api":
            lines.append(f"  quilt api         {qu['api']['version']} (the pinned Fabric API; no QFAPI for {game})")
        if not e.get("parchment"):
            lines.append("  parchment         none (see notes)")
        names = (["fabric=intermediary"] if fb else []) + [f"{k}={e[k]['runtime_names']}" for k in ("neoforge", "forge", "quilt") if e.get(k)]
        lines.append("  runtime names     " + " ".join(names))
        if nf:
            toml = nf["mods_toml"]
            lines.append(f"  neoforge toml     {toml.get('file', 'META-INF/neoforge.mods.toml')} modLoader={toml['modLoader']} loaderVersion={toml['loaderVersion']} "
                         f"{toml.get('loader_dependency_id', 'neoforge')}={toml['neoforge_dependency_range']} minecraft={toml['minecraft_dependency_range']}")
        if fg:
            toml = fg["mods_toml"]
            lines.append(f"  forge toml        {toml['file']} modLoader={toml['modLoader']} loaderVersion={toml['loaderVersion']} "
                         f"forge={toml['forge_dependency_range']} minecraft={toml['minecraft_dependency_range']}")
        lines.append("")
    vf = lock["global"]["vineflower"]
    lines.append(f"Global: vineflower {vf['version']}  {vf['size']:,} B  {short(vf)}")
    lines.append(f"Notes: {len(lock['notes'])} (read them in the file)")
    return lines


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="offline schema validation (default)")
    mode.add_argument("--print", action="store_true", dest="print_", help="offline summary of the pinned artefacts")
    mode.add_argument("--verify", action="store_true", help="re-fetch publisher metadata and compare every pin")
    ap.add_argument("--lock", type=Path, default=DEFAULT_LOCK, help="path to research-lock.json")
    ap.add_argument("--only", help="verify one game version only")
    ap.add_argument("--strict", action="store_true", help="with --verify, treat warnings as failures")
    ap.add_argument("--json", action="store_true", help="with --verify, print results as JSON")
    args = ap.parse_args(argv)

    try:
        lock = load_lock(args.lock)
    except (OSError, json.JSONDecodeError) as err:
        print(f"error: cannot read {args.lock}: {err}", file=sys.stderr)
        return 2
    errors = validate(lock)
    if errors:
        print(f"{args.lock}: {len(errors)} schema error(s)", file=sys.stderr)
        for e in errors:
            print(f"  {e}", file=sys.stderr)
        return 1
    if args.only and args.only not in lock["game_versions"]:
        print(f"error: {args.only} is not pinned in {args.lock}", file=sys.stderr)
        return 2

    if args.print_:
        print("\n".join(summary_lines(lock)))
        return 0
    if not args.verify:
        games = ", ".join(lock["game_versions"])
        print(f"{args.lock.name}: schema OK ({games})")
        return 0

    report = verify(lock, args.only)
    fails = report.count("FAIL") + (report.count("WARN") if args.strict else 0)
    if args.json:
        print(json.dumps({"results": [r.__dict__ for r in report.results],
                          "counts": {s: report.count(s) for s in ("OK", "WARN", "FAIL", "INFO")},
                          "passed": fails == 0}, indent=2))
    else:
        for r in report.results:
            print(f"{r.status:<4} {r.subject}: {r.message}")
        print(f"\n{report.count('OK')} ok, {report.count('WARN')} warnings, {report.count('FAIL')} failed, {report.count('INFO')} info")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
