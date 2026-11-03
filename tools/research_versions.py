#!/usr/bin/env python3
"""Rank Minecraft 1.21 minors by Fabric download volume on Modrinth.

Modrinth has no per-game-version download statistic, so this uses a proxy:
for the top N Fabric mods by downloads, sum the downloads of every version
file, splitting each file's downloads evenly across the 1.21 minors it lists.
The result decides the two pinned versions in research-lock.json (D-002).

Usage: python tools/research_versions.py [--top 200] [--out path.json]
"""
from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.parse
import urllib.request
from collections import defaultdict

API = "https://api.modrinth.com/v2"
UA = "MosheByte/jarrow-research/0.1 (github.com/MosheByte/jarrow)"


def get(path: str, params: dict | None = None):
    url = API + path
    if params:
        url += "?" + urllib.parse.urlencode(params)
    for attempt in range(5):
        req = urllib.request.Request(url, headers={"User-Agent": UA})
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                return json.load(resp)
        except urllib.error.HTTPError as err:
            if err.code == 429:
                time.sleep(2 ** attempt)
                continue
            raise
    raise RuntimeError(f"rate limited: {url}")


def top_fabric_mods(n: int) -> list[dict]:
    facets = json.dumps([["project_type:mod"], ["categories:fabric"]])
    hits: list[dict] = []
    offset = 0
    while len(hits) < n:
        page = get("/search", {"facets": facets, "index": "downloads", "limit": 100, "offset": offset})
        if not page["hits"]:
            break
        hits.extend(page["hits"])
        offset += 100
    return hits[:n]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--top", type=int, default=200)
    ap.add_argument("--out")
    args = ap.parse_args()

    minors = [
        g["version"]
        for g in get("/tag/game_version")
        if g["version_type"] == "release" and (g["version"] == "1.21" or g["version"].startswith("1.21."))
    ]
    score: dict[str, float] = defaultdict(float)
    for project in top_fabric_mods(args.top):
        versions = get(f"/project/{project['project_id']}/version", {"loaders": json.dumps(["fabric"])})
        for v in versions:
            hit = [g for g in v["game_versions"] if g in minors]
            if not hit:
                continue
            share = v["downloads"] / len(hit)
            for g in hit:
                score[g] += share
    ranked = sorted(score.items(), key=lambda kv: kv[1], reverse=True)
    result = {
        "method": f"top {args.top} Fabric mods by downloads; per-file downloads split across listed 1.21 minors",
        "ranking": [{"game_version": g, "weighted_downloads": round(d)} for g, d in ranked],
        "pinned": [g for g, _ in ranked[:2]],
    }
    text = json.dumps(result, indent=2)
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(text + "\n")
    print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
