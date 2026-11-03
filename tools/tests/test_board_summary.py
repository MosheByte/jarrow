"""Tests for tools/board_summary.py: titles, bodies, idempotent posting. Offline: gh is faked."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

import board_summary

SHA = "0123456789abcdef0123456789abcdef01234567"
ROOT = Path(__file__).resolve().parents[2]
# The board.json `jarrow-board merge` writes, kept current by crates/board/tests/sample.rs.
CRATE_SAMPLE = ROOT / "crates" / "board" / "tests" / "fixtures" / "board-sample.json"

REASON_KINDS = [
    "no_converter",
    "download_failed",
    "crash",
    "mixin_failure",
    "class_not_found",
    "registry_conflict",
    "entries_differ",
    "harness_load_failure",
    "unsupported_native",
    "unsupported_transformer",
    "tier_b_required",
    "opt_out",
]


def direction(name: str = "fabric-neoforge", mods: int = 1000, reasons: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    """A direction with a column, as in board.json schema 2."""
    if reasons is None:
        reasons = [{"kind": "no_converter", "label": "No converter yet (milestone M2)", "count": mods}]
    source, target = name.split("-")
    return {
        "id": name,
        "from": source,
        "to": target,
        "column": True,
        "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
        "list_fetched_at": "2026-11-01T02:00:00Z",
        "rules_version": "none",
        "totals": {"mods": mods, "green": 0, "yellow": 0, "red": mods},
        "top_reasons": reasons,
    }


def no_column(name: str) -> dict[str, Any]:
    source, target = name.split("-")
    return {
        "id": name,
        "from": source,
        "to": target,
        "column": False,
        "game_versions": [],
        "list_fetched_at": None,
        "rules_version": None,
        "totals": {"mods": 0, "green": 0, "yellow": 0, "red": 0},
        "top_reasons": [],
    }


def board(*columns: dict[str, Any]) -> dict[str, Any]:
    """All 12 directions in build order; the given ones have a column."""
    columns = columns or (direction(),)
    by_id = {c["id"]: c for c in columns}
    return {
        "schema": 2,
        "generated_at": "2026-11-04T03:10:00Z",
        "started_at": "2026-11-04T02:20:00Z",
        "run_id": "nightly-20261104",
        "engine": {"version": "0.1.0", "git_sha": SHA},
        "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
        "totals": {"mods": 0, "green": 0, "yellow": 0, "red": 0},
        "directions": [by_id.get(d, no_column(d)) for d in board_summary.DIRECTIONS],
        "stats": {"shards": 10, "files": 1800, "max_shard_wall_ms": 1_260_000},
        "rows": [],
    }


def write(tmp_path: Path, data: dict[str, Any]) -> Path:
    path = tmp_path / "board.json"
    path.write_text(json.dumps(data), encoding="utf-8")
    return path


RUN_URL = "https://github.com/MosheByte/jarrow/actions/runs/42"


# ----------------------------------------------------------------------------- rendering


def test_single_direction_title_matches_the_contract() -> None:
    summary = board_summary.read_board(board())
    [post] = board_summary.posts(summary, RUN_URL, "MosheByte/jarrow")
    assert post.title == "Nightly 2026-11-04: 0 green, 0 yellow, 1000 red"
    assert post.direction == "fabric-neoforge"


def test_several_directions_get_one_titled_post_each() -> None:
    summary = board_summary.read_board(board(direction("forge-neoforge", 10), direction("fabric-neoforge", 20)))
    titles = [p.title for p in board_summary.posts(summary, RUN_URL, "MosheByte/jarrow")]
    assert titles == [
        "Nightly 2026-11-04 (fabric-neoforge): 0 green, 0 yellow, 20 red",
        "Nightly 2026-11-04 (forge-neoforge): 0 green, 0 yellow, 10 red",
    ]


def test_body_has_totals_links_marker_and_the_honest_note() -> None:
    summary = board_summary.read_board(board())
    [post] = board_summary.posts(summary, RUN_URL, "MosheByte/jarrow")
    body = post.body
    assert body.startswith("<!-- jarrow-nightly date=2026-11-04 direction=fabric-neoforge -->\n")
    assert "**No converter exists yet** (milestone M2), so every mod on this board is red" in body
    assert "| Red | 1,000 | 100.0% |" in body
    assert "| Green | 0 | 0.0% |" in body
    assert "| Total | 1,000 | |" in body
    assert "**Fabric to NeoForge** on Minecraft 1.20.1, 1.21.1, 1.21.11, run `nightly-20261104`" in body
    assert f"- Workflow run: {RUN_URL}" in body
    assert "- Board: https://jarrow.pages.dev/board" in body
    assert "https://github.com/MosheByte/jarrow/releases/tag/board-latest" in body
    assert "the slowest shard took 21.0 minutes" in body
    assert "guarantee" not in body.lower()


def test_only_the_top_ten_reasons_are_listed_in_order() -> None:
    reasons = [
        {"kind": kind, "label": f"Label {kind}", "count": 100 - i}
        for i, kind in enumerate(reversed(REASON_KINDS))
    ]
    total = sum(r["count"] for r in reasons)
    summary = board_summary.read_board(board(direction(mods=total, reasons=reasons)))
    body = board_summary.posts(summary, None, "MosheByte/jarrow")[0].body
    rows = [line for line in body.splitlines() if line.startswith("| Label ")]
    assert len(rows) == 10
    assert rows[0] == "| Label opt_out | `opt_out` | 100 |"
    assert "### Top reasons (10 of 12)" in body
    assert "Label no_converter" not in body  # the twelfth, least frequent
    # A partial no_converter count still states that no converter exists.
    assert "**No converter exists yet** (milestone M2), so 89 of " in body
    assert "- Workflow run: not available (local run)" in body


def test_no_note_when_no_mod_is_red_for_lack_of_a_converter() -> None:
    d = direction(mods=3, reasons=[{"kind": "crash", "label": "Server crash", "count": 2}])
    d["totals"] = {"mods": 3, "green": 1, "yellow": 0, "red": 2}
    summary = board_summary.read_board(board(d))
    post = board_summary.posts(summary, RUN_URL, "MosheByte/jarrow")[0]
    assert "No converter exists yet" not in post.body
    assert post.title == "Nightly 2026-11-04: 1 green, 0 yellow, 2 red"


def test_table_cells_are_escaped() -> None:
    d = direction(mods=1, reasons=[{"kind": "crash", "label": "a | b", "count": 1}])
    body = board_summary.posts(board_summary.read_board(board(d)), None, "o/r")[0].body
    assert "| a \\| b | `crash` | 1 |" in body


def test_directions_without_a_column_get_no_post() -> None:
    summary = board_summary.read_board(board())
    assert [c.direction for c in summary.columns] == ["fabric-neoforge"]
    assert [p.direction for p in board_summary.posts(summary, None, "o/r")] == ["fabric-neoforge"]


def test_the_board_crates_board_writes_gives_one_post() -> None:
    data = json.loads(CRATE_SAMPLE.read_text(encoding="utf-8"))
    summary = board_summary.read_board(data)
    [post] = board_summary.posts(summary, RUN_URL, "MosheByte/jarrow")
    column = data["directions"][0]
    totals = column["totals"]
    assert post.direction == "fabric-neoforge"
    assert post.title == f"Nightly 2026-11-04: 0 green, 0 yellow, {totals['red']} red"
    assert "**No converter exists yet** (milestone M2)" in post.body
    assert "`no_converter`" in post.body and "`download_failed`" in post.body
    assert f"**Fabric to NeoForge** on Minecraft {', '.join(column['game_versions'])}" in post.body


@pytest.mark.parametrize(
    "mutate",
    [
        lambda b: b.update(schema=1),
        lambda b: b.update(started_at="yesterday"),
        lambda b: b.update(directions=[]),
        lambda b: b["directions"].pop(),
        lambda b: b["directions"].reverse(),
        lambda b: b["directions"][0].update(id="fabric-fabric"),
        lambda b: b["directions"][0].update(id="rift-neoforge"),
        lambda b: b["directions"][0].update({"from": "quilt"}),
        lambda b: b["directions"][0].update(column="yes"),
        lambda b: b["directions"][0]["totals"].update(red=-1),
        lambda b: b["directions"][0]["totals"].update(red=999),
        lambda b: b["directions"][0]["top_reasons"][0].update(count="many"),
        lambda b: b["directions"][1]["totals"].update(mods=3, red=3),
        lambda b: b["directions"][0].update(column=False),
        lambda b: b.pop("engine"),
    ],
)
def test_malformed_boards_are_rejected(mutate: Any) -> None:
    data = board()
    mutate(data)
    with pytest.raises(board_summary.BoardError):
        board_summary.read_board(data)


# ------------------------------------------------------------------------------- posting


class FakeGh:
    """Answers the three GraphQL calls; records every call."""

    def __init__(self, categories: dict[str, str], existing: list[dict[str, Any]] | None = None) -> None:
        self.categories = categories
        self.existing = existing or []
        self.calls: list[dict[str, Any]] = []
        self.created: list[dict[str, Any]] = []

    def __call__(self, args: list[str], stdin: str) -> str:
        assert args == ["api", "graphql", "--input", "-"]
        request = json.loads(stdin)
        self.calls.append(request)
        query, variables = request["query"], request["variables"]
        if query.startswith("mutation"):
            self.created.append(variables)
            url = f"https://github.com/MosheByte/jarrow/discussions/{len(self.created)}"
            return json.dumps({"data": {"createDiscussion": {"discussion": {"url": url}}}})
        if "discussionCategories" in query:
            nodes = [{"id": i, "name": n} for n, i in self.categories.items()]
            return json.dumps({"data": {"repository": {"id": "R_1", "discussionCategories": {"nodes": nodes}}}})
        if "discussions(" in query:
            assert variables["category"] in self.categories.values()
            return json.dumps({"data": {"repository": {"discussions": {"nodes": self.existing}}}})
        raise AssertionError(f"unexpected query {query}")


def test_posts_to_nightly_results(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    gh = FakeGh({"Announcements": "C_a", "Nightly results": "C_n"})
    path = write(tmp_path, board())
    assert board_summary.main(["--board", str(path), "--run-url", RUN_URL], runner=gh, env={}) == 0
    assert len(gh.created) == 1
    created = gh.created[0]
    assert created["category"] == "C_n"
    assert created["repo"] == "R_1"
    assert created["title"] == "Nightly 2026-11-04: 0 green, 0 yellow, 1000 red"
    assert "posted in Nightly results" in capsys.readouterr().out


def test_falls_back_to_announcements(tmp_path: Path) -> None:
    gh = FakeGh({"Announcements": "C_a", "Q&A": "C_q"})
    assert board_summary.main(["--board", str(write(tmp_path, board()))], runner=gh, env={}) == 0
    assert gh.created[0]["category"] == "C_a"


def test_fails_without_either_category(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    gh = FakeGh({"Q&A": "C_q"})
    assert board_summary.main(["--board", str(write(tmp_path, board()))], runner=gh, env={}) == 1
    assert gh.created == []
    assert "Nightly results" in capsys.readouterr().err


def test_does_not_post_twice_for_the_same_date_and_direction(tmp_path: Path) -> None:
    existing = [
        {
            "title": "Nightly 2026-11-04: 0 green, 0 yellow, 999 red",
            "url": "https://github.com/MosheByte/jarrow/discussions/7",
            "body": "<!-- jarrow-nightly date=2026-11-04 direction=fabric-neoforge -->\nedited",
        },
        {"title": "Nightly 2026-11-03: 0 green, 0 yellow, 1000 red", "url": "u", "body": None},
    ]
    gh = FakeGh({"Nightly results": "C_n"}, existing)
    path = write(tmp_path, board(direction("fabric-neoforge"), direction("forge-neoforge", 5)))
    assert board_summary.main(["--board", str(path)], runner=gh, env={}) == 0
    # fabric-neoforge already has its post for the date; forge-neoforge is new.
    assert [c["title"] for c in gh.created] == ["Nightly 2026-11-04 (forge-neoforge): 0 green, 0 yellow, 5 red"]


def test_a_second_run_posts_nothing(tmp_path: Path) -> None:
    gh = FakeGh({"Nightly results": "C_n"})
    path = write(tmp_path, board())
    assert board_summary.main(["--board", str(path)], runner=gh, env={}) == 0
    gh.existing = [{"title": c["title"], "url": "u", "body": c["body"]} for c in gh.created]
    assert board_summary.main(["--board", str(path)], runner=gh, env={}) == 0
    assert len(gh.created) == 1


def test_dry_run_prints_and_never_calls_gh(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    def no_gh(args: list[str], stdin: str) -> str:
        raise AssertionError("dry run called gh")

    env = {"GITHUB_SERVER_URL": "https://github.com", "GITHUB_REPOSITORY": "MosheByte/jarrow", "GITHUB_RUN_ID": "99"}
    path = write(tmp_path, board())
    assert board_summary.main(["--board", str(path), "--dry-run"], runner=no_gh, env=env) == 0
    out = capsys.readouterr().out
    assert out.startswith("# Nightly 2026-11-04: 0 green, 0 yellow, 1000 red\n")
    assert "https://github.com/MosheByte/jarrow/actions/runs/99" in out


def test_graphql_errors_and_gh_failures_exit_1(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    def errors(args: list[str], stdin: str) -> str:
        return json.dumps({"errors": [{"message": "Resource not accessible by integration"}]})

    def broken(args: list[str], stdin: str) -> str:
        raise board_summary.GitHubError("gh api failed: HTTP 401")

    path = write(tmp_path, board())
    assert board_summary.main(["--board", str(path)], runner=errors, env={}) == 1
    assert "Resource not accessible" in capsys.readouterr().err
    assert board_summary.main(["--board", str(path)], runner=broken, env={}) == 1


def test_bad_input_exits_2(tmp_path: Path) -> None:
    path = tmp_path / "board.json"
    path.write_text("{not json", encoding="utf-8")
    assert board_summary.main(["--board", str(path), "--dry-run"], env={}) == 2
    assert board_summary.main(["--board", str(tmp_path / "missing.json"), "--dry-run"], env={}) == 2


def test_repo_must_be_owner_slash_name() -> None:
    with pytest.raises(board_summary.GitHubError):
        board_summary.GitHub("not a repo")


def test_run_url_from_the_environment() -> None:
    env = {"GITHUB_SERVER_URL": "https://github.com/", "GITHUB_REPOSITORY": "o/r", "GITHUB_RUN_ID": "5"}
    assert board_summary.default_run_url(env) == "https://github.com/o/r/actions/runs/5"
    assert board_summary.default_run_url({}) is None
