#!/usr/bin/env python3
"""Check, summarise and re-verify research-lock.json.

research-lock.json pins every external artefact Jarrow depends on for the two
game versions (Mojang server jar and mappings, Fabric Loader, intermediary,
Fabric API, the Fabric installer and server launcher, NeoForge, Parchment,
Vineflower) together with the hashes the publishers themselves provide.

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
REQUIRED_GAME_VERSIONS = ("1.21.1", "1.21.11")
HEX_LENGTHS = {"sha1": 40, "sha256": 64, "sha512": 128}
HASH_SOURCES = {"maven-sidecar", "mojang-version-json", "mojang-manifest", "modrinth-api", "observed"}
METADATA_LIMIT = 8 * 1024 * 1024  # refuse to read anything bigger as "metadata"
LAUNCHER_LIMIT = 1024 * 1024
TIMEOUT = 60

_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
_MAVEN_RANGE = re.compile(r"^[\[(][^\[\]()]*[\])]$")
_GAME = re.compile(r"^1\.(\d+)(?:\.(\d+))?$")


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
    version = _check_text(node, "version", path, errors)
    m = _GAME.match(game)
    if version is not None and m:
        prefix = f"{m.group(1)}.{m.group(2) or 0}."
        if not version.startswith(prefix):
            errors.add(f"{path}.version", f"NeoForge versions for {game} start with {prefix!r}")
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
        if version is not None and isinstance(inst.get("url"), str) and f"neoforge-{version}-installer.jar" not in inst["url"]:
            errors.add(f"{path}.installer.url", "must point at this version's installer jar")
    if node.get("runtime_names") != "mojang-official":
        errors.add(f"{path}.runtime_names", "expected 'mojang-official'")
    _check_text(node, "install_command", path, errors)
    toml = _get(node, "mods_toml", path, errors)
    if toml is not None:
        p = f"{path}.mods_toml"
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
        for key, url in sources.items():
            _check_url(url, f"$.sources.{key}", errors)
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
                errors.add(f"$.game_versions.{game}", "missing (both pinned game versions are required)")
        for game, entry in games.items():
            path = f"$.game_versions.{game}"
            if not _GAME.match(game):
                errors.add(path, "game version keys look like 1.21.x")
            if not isinstance(entry, dict):
                errors.add(path, "expected an object")
                continue
            for key in ("mojang", "fabric", "neoforge"):
                if key not in entry:
                    errors.add(f"{path}.{key}", "missing")
            if "mojang" in entry:
                _check_mojang(entry["mojang"], f"{path}.mojang", errors)
                if jdk is not None and isinstance(entry["mojang"], dict):
                    java = entry["mojang"].get("java_major")
                    if isinstance(java, int) and isinstance(jdk.get("major"), int) and java > jdk["major"]:
                        errors.add(f"{path}.mojang.java_major", "needs a newer JDK than $.jdk.major")
            if "fabric" in entry:
                _check_fabric(entry["fabric"], game, f"{path}.fabric", errors)
            if "neoforge" in entry:
                _check_neoforge(entry["neoforge"], game, f"{path}.neoforge", errors)
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
        # Pins are read from what these URLs serve, so a redirect may never leave HTTPS
        # (security review F-10).
        if urllib.parse.urlsplit(urllib.parse.urljoin(req.full_url, newurl)).scheme != "https":
            raise urllib.error.HTTPError(
                newurl, code, f"refusing a redirect from {req.full_url} to a non-HTTPS URL", headers, fp
            )
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
        live = fetch_json(f"{self.lock['sources']['modrinth_api']}/version/{api['modrinth_version_id']}")
        problems = []
        if live.get("project_id") != api["modrinth_project_id"]:
            problems.append(f"project {live.get('project_id')}")
        if live.get("version_number") != api["version"]:
            problems.append(f"version {live.get('version_number')}")
        if game not in live.get("game_versions", []):
            problems.append(f"game_versions {live.get('game_versions')}")
        if "fabric" not in live.get("loaders", []):
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
        published = parse_sidecar(fetch(api["maven_url"] + ".sha1", limit=4096).decode("ascii", "replace"))
        self.report.add("OK" if published == api["maven_sha1"] else "FAIL", subject,
                        "Fabric maven .sha1 matches" if published == api["maven_sha1"] else f"Fabric maven .sha1 is {published}")
        self.reachable(subject, api)
        params = urllib.parse.urlencode({"game_versions": json.dumps([game]), "loaders": json.dumps(["fabric"])})
        versions = fetch_json(f"{self.lock['sources']['modrinth_api']}/project/{api['modrinth_project_id']}/version?{params}")
        newest = next((v["version_number"] for v in versions if v.get("version_type") == "release"), None)
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
        listing = self.cached_json(self.lock["sources"]["neoforge_versions"])
        versions = listing.get("versions", [])
        if node["version"] not in versions:
            self.report.add("FAIL", subject, f"{node['version']} not listed by the NeoForge maven API")
        else:
            self.report.add("OK", subject, f"{node['version']} listed by the NeoForge maven API")
            prefix = node["version"].rsplit(".", 1)[0] + "."

            def build(v: str) -> int:
                tail = v[len(prefix):].split("-", 1)[0]
                return int(tail) if tail.isdigit() else -1

            same_line = [v for v in versions if v.startswith(prefix) and build(v) >= 0]
            stable = [v for v in same_line if not v.endswith("-beta")]
            pool = stable if stable else same_line
            newest = max(pool, key=build) if pool else None
            if newest and newest != node["version"]:
                self.report.add("INFO", subject, f"a newer {'stable' if stable else 'beta'} build exists: {newest}")
            if node["beta"] and stable:
                self.report.add("WARN", subject, "pinned a beta although a stable build now exists")
        self.sidecars(subject, node["installer"])
        self.reachable(subject, node["installer"])

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
        v.guard(f"{game} fabric", lambda: v.fabric(game, entry["fabric"]))
        v.guard(f"{game} neoforge", lambda: v.neoforge(game, entry["neoforge"]))
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
            ("fabric loader", e["fabric"]["loader"]["version"], e["fabric"]["loader"]),
            ("intermediary", e["fabric"]["intermediary"]["version"], e["fabric"]["intermediary"]),
            ("fabric api", e["fabric"]["api"]["version"], e["fabric"]["api"]),
            ("fabric installer", e["fabric"]["installer"]["version"], e["fabric"]["installer"]),
            ("server launcher", "meta-generated", e["fabric"]["server_launcher"]),
            ("neoforge", e["neoforge"]["version"] + (" (beta)" if e["neoforge"]["beta"] else ""), e["neoforge"]["installer"]),
        ]
        if e.get("parchment"):
            rows.append(("parchment", e["parchment"]["version"], e["parchment"]))
        for name, version, art in rows:
            lines.append(f"  {name:<17} {version:<20} {art['size']:>10,} B  {short(art)}")
        if not e.get("parchment"):
            lines.append("  parchment         none (see notes)")
        toml = e["neoforge"]["mods_toml"]
        lines.append(f"  mods.toml         modLoader={toml['modLoader']} loaderVersion={toml['loaderVersion']} "
                     f"neoforge={toml['neoforge_dependency_range']} minecraft={toml['minecraft_dependency_range']}")
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
