"""Tests for tools/pr_ready.py: each check passing and failing on a real git repository."""
from __future__ import annotations

import io
import shutil
import zipfile

import pytest

import pr_ready
from conftest import TOOLS, TRAILER, VALID_DECISIONS, GitRepo


def tree(r: GitRepo) -> pr_ready.Context:
    return pr_ready.Context(pr_ready.Repo(r.root))


def span(r: GitRepo, base: str, head: str) -> pr_ready.Context:
    return pr_ready.Context(pr_ready.Repo(r.root), base, head)


def problems(check: str, ctx: pr_ready.Context) -> list[str]:
    result = pr_ready.run(ctx, [check])[check]
    assert result is not None, f"{check} was skipped"
    return result


def zip_bytes(names: list[str]) -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as archive:
        for name in names:
            archive.writestr(name, b"\xca\xfe\xba\xbe" if name.endswith(".class") else b"x")
    return buf.getvalue()


# Secret-looking values are assembled at run time so this file never contains one.
def fake(prefix: str, body: str) -> str:
    return prefix + body


FAKE_SECRETS = {
    "a GitHub token": fake("gh" + "p_", "A1b2" * 9),
    "a GitHub fine-grained token": fake("github" + "_pat_", "11ABCDEFG0" + "x" * 30),
    "an npm token": fake("np" + "m_", "Z9" * 18),
    "a GitLab token": fake("gl" + "pat-", "abcDEF123_" * 2),
    "a Slack token": fake("xo" + "xb-", "1234567890-abcdef"),
    "an AWS access key id": fake("AK" + "IA", "ABCDEFGHIJ234567"),
    "a private key block": "-----" + "BEGIN RSA PRIVATE" + " KEY-----",
    "a Discord webhook URL": fake("https://discord.com/api/" + "webhooks/", "123456789/abcDEF_-xyz"),
    "a Slack webhook URL": fake("https://hooks.slack.com/" + "services/", "T0000/B0000/" + "a" * 24),
    "a literal CLOUDFLARE_API_TOKEN value": fake("CLOUDFLARE_API" + "_TOKEN=", "q" * 40),
}


# ------------------------------------------------------------------------- baseline


def test_baseline_passes_in_tree_mode(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    assert pr_ready.main(["--all", "--repo", str(repo.root)]) == 0
    out = capsys.readouterr().out
    assert "all checks passed" in out
    assert "skip  trailer" in out
    assert "skip  research-lock" in out


def test_baseline_passes_in_range_mode(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    base = repo.head()
    repo.write("README.md", "# Test\n\nMore.\n")
    head = repo.commit("Edit readme")
    assert pr_ready.main(["--base", base, "--head", head, "--repo", str(repo.root)]) == 0
    out = capsys.readouterr().out
    assert "ok    trailer" in out
    assert "ok    research-lock" in out


def test_unknown_commit_is_a_usage_error(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    code = pr_ready.main(["--base", "0" * 40, "--head", repo.head(), "--repo", str(repo.root)])
    assert code == 2
    assert "fetch-depth: 0" in capsys.readouterr().err


def test_base_requires_head() -> None:
    with pytest.raises(SystemExit):
        pr_ready.main(["--base", "abc"])


def test_check_flag_selects_checks(repo: GitRepo, capsys: pytest.CaptureFixture[str]) -> None:
    repo.write("LEDGER.md", "hand edited\n")
    repo.commit("Break the ledger")
    assert pr_ready.main(["--all", "--check", "secrets", "--repo", str(repo.root)]) == 0
    out = capsys.readouterr().out
    assert "ledger" not in out
    assert pr_ready.main(["--all", "--check", "ledger", "--repo", str(repo.root)]) == 1


def test_github_annotations(repo: GitRepo, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("GITHUB_ACTIONS", "true")
    repo.write("LEDGER.md", "hand edited\n")
    repo.commit("Break the ledger")
    assert pr_ready.main(["--all", "--check", "ledger", "--repo", str(repo.root)]) == 1
    assert "::error title=pr_ready ledger::LEDGER.md differs" in capsys.readouterr().out


# -------------------------------------------------------------------------- trailer


def test_trailer_present_passes(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("a.txt", "a\n")
    repo.commit("Add a")
    repo.write("b.txt", "b\n")
    head = repo.commit("Add b")
    assert problems("trailer", span(repo, base, head)) == []


def test_trailer_missing_fails(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("a.txt", "a\n")
    repo.commit("Add a")
    repo.write("b.txt", "b\n")
    head = repo.commit("Add b without trailer", trailer=False)
    found = problems("trailer", span(repo, base, head))
    assert len(found) == 1
    assert head[:12] in found[0]
    assert "Add b without trailer" in found[0]
    assert TRAILER in found[0]


def test_trailer_must_be_in_final_paragraph(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("a.txt", "a\n")
    head = repo.commit(f"Add a\n\n{TRAILER}\n\nMore text after the trailer.", trailer=False)
    assert len(problems("trailer", span(repo, base, head))) == 1


def test_trailer_key_is_case_insensitive(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("a.txt", "a\n")
    message = "Add a\n\nCo-authored-by: Claude Opus 5.5 <noreply@anthropic.com>\n"
    head = repo.commit(message, trailer=False)
    assert problems("trailer", span(repo, base, head)) == []


def test_trailer_with_other_trailers_passes(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("a.txt", "a\n")
    message = f"Add a\n\nSigned-off-by: Someone <s@example.invalid>\n{TRAILER}\n"
    head = repo.commit(message, trailer=False)
    assert problems("trailer", span(repo, base, head)) == []


def test_wrong_model_in_trailer_fails(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("a.txt", "a\n")
    message = "Add a\n\nCo-Authored-By: Someone Else <noreply@anthropic.com>\n"
    head = repo.commit(message, trailer=False)
    assert len(problems("trailer", span(repo, base, head))) == 1


def test_bot_commits_are_exempt(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("Cargo.lock", "# bumped\n")
    bot = "dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>"
    head = repo.commit("deps: bump serde", author=bot, trailer=False)
    assert problems("trailer", span(repo, base, head)) == []


def test_empty_range_passes(repo: GitRepo) -> None:
    head = repo.head()
    assert problems("trailer", span(repo, head, head)) == []


# ------------------------------------------------------------------------ artifacts


@pytest.mark.parametrize("name", ["libs/mod.jar", "Foo.class", "web/app.war", "x/app.EAR", "UPPER.JAR"])
def test_banned_extensions_fail(repo: GitRepo, name: str) -> None:
    repo.write(name, b"PK\x03\x04 not really")
    repo.commit(f"Add {name}")
    found = problems("artifacts", tree(repo))
    assert len(found) == 1 and found[0].startswith(name)


def test_zip_with_classes_fails(repo: GitRepo) -> None:
    repo.write("fixtures/bundle.zip", zip_bytes(["a/B.class", "README"]))
    repo.commit("Add zip")
    found = problems("artifacts", tree(repo))
    assert found and "a/B.class" in found[0]


def test_disguised_jar_fails(repo: GitRepo) -> None:
    repo.write("fixtures/innocent.bin", zip_bytes(["META-INF/MANIFEST.MF", "com/x/Mod.class"]))
    repo.commit("Add disguised archive")
    assert len(problems("artifacts", tree(repo))) == 1


def test_zip_with_nested_jar_fails(repo: GitRepo) -> None:
    repo.write("fixtures/bundle.zip", zip_bytes(["mods/inner.jar"]))
    repo.commit("Add zip with a jar")
    assert len(problems("artifacts", tree(repo))) == 1


def test_zip_without_classes_passes(repo: GitRepo) -> None:
    repo.write("fixtures/resources.zip", zip_bytes(["assets/x/lang/en_us.json", "pack.mcmeta"]))
    repo.commit("Add resource zip")
    assert problems("artifacts", tree(repo)) == []


DECOMPILED = {
    "cfr": ["/*", " * Decompiled with CFR 0.152.", " */", "package com.example;", "", "public class A {}"],
    "procyon": ["//", "// Decompiled by Procyon v0.6.0", "//", "", "package com.example;", "class B {}"],
    "intellij": [
        "//",
        "// Source code recreated from a .class file by IntelliJ IDEA",
        "// (powered by FernFlower decompiler)",
        "//",
        "",
        "package com.example;",
    ],
    "vineflower": ["// Vineflower 1.11.1", "package com.example;", "class C {}"],
}


@pytest.mark.parametrize("style", sorted(DECOMPILED))
def test_decompiled_java_fails(repo: GitRepo, style: str) -> None:
    repo.write(f"src/{style}/A.java", "\n".join(DECOMPILED[style]) + "\n")
    repo.commit("Add decompiled source")
    found = problems("artifacts", tree(repo))
    assert len(found) == 1 and "looks decompiled" in found[0]


def test_decompiled_source_with_other_extension_fails(repo: GitRepo) -> None:
    repo.write("notes/A.txt", "\n".join(DECOMPILED["cfr"]) + "\n")
    repo.commit("Add renamed decompiled source")
    assert len(problems("artifacts", tree(repo))) == 1


def test_jd_gui_footer_fails(repo: GitRepo) -> None:
    body = ["package com.example;", "", "public class D {"] + ["  int f;"] * 50 + ["}", ""]
    footer = ["/* Location:              C:\\mods\\x.jar", " * Qualified Name:     com.example.D", " * JD-Core Version:    1.1.3", " */"]
    repo.write("src/D.java", "\n".join(body + footer) + "\n")
    repo.commit("Add JD output")
    assert len(problems("artifacts", tree(repo))) == 1


def test_ordinary_sources_pass(repo: GitRepo) -> None:
    java = ["/*", " * Copyright 2026 The Jarrow authors. MIT license.", " */", "package dev.jarrow.probe;", "", "final class Probe {}"]
    repo.write("harness/probe/Probe.java", "\n".join(java) + "\n")
    repo.write("docs/tierb.md", "# Tier B\n\n* Vineflower decompiles the JAR (CFR and Procyon were considered).\n")
    repo.write("crates/report/src/tierb.rs", "//! Decompiles with Vineflower inside the job; nothing is kept.\n")
    repo.write("tools/tierb/run.py", "# Decompiled with Vineflower in a scratch dir, then wiped.\n")
    repo.commit("Add ordinary sources")
    assert problems("artifacts", tree(repo)) == []


def test_tools_do_not_trip_their_own_checks(repo: GitRepo) -> None:
    (repo.root / "tools" / "tests").mkdir(parents=True)
    for name in ["pr_ready.py", "ledger.py", "requirements-dev.txt"]:
        shutil.copy(TOOLS / name, repo.root / "tools" / name)
    for test_file in sorted((TOOLS / "tests").glob("*.py")):
        shutil.copy(test_file, repo.root / "tools" / "tests" / test_file.name)
    repo.commit("Add tools")
    assert problems("artifacts", tree(repo)) == []
    assert problems("secrets", tree(repo)) == []


def test_untracked_jars_are_ignored(repo: GitRepo) -> None:
    (repo.root / "scratch").mkdir()
    (repo.root / "scratch/mod.jar").write_bytes(b"PK\x03\x04")
    assert problems("artifacts", tree(repo)) == []


# -------------------------------------------------------------------- research-lock


def test_research_lock_change_needs_decision(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("research-lock.json", '{"game_versions": ["1.21.1"]}\n')
    head = repo.commit("Pin versions")
    found = problems("research-lock", span(repo, base, head))
    assert len(found) == 1 and "DECISIONS.md did not" in found[0]


def test_research_lock_change_needs_a_new_entry(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("research-lock.json", '{"game_versions": ["1.21.1"]}\n')
    repo.write("DECISIONS.md", VALID_DECISIONS + "\nA reworded paragraph.\n")
    head = repo.commit("Pin versions, touch decisions")
    found = problems("research-lock", span(repo, base, head))
    assert len(found) == 1 and "gained no new" in found[0]


def test_research_lock_change_with_new_entry_passes(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("research-lock.json", '{"game_versions": ["1.21.1"]}\n')
    entry = "\n".join(
        [
            "## D-002 \u2014 Lock loader versions",
            "",
            "**Date:** 2026-11-03",
            "**Decision:** Lock them.",
            "**Why:** Reproducible harness.",
            "**Reversal cost:** Low.",
            "**Links:** research-lock.json",
            "",
        ]
    )
    repo.write("DECISIONS.md", VALID_DECISIONS + "\n" + entry)
    head = repo.commit("Pin versions with a decision")
    assert problems("research-lock", span(repo, base, head)) == []


def test_research_lock_untouched_passes(repo: GitRepo) -> None:
    base = repo.head()
    repo.write("README.md", "# Changed\n")
    head = repo.commit("Docs only")
    assert problems("research-lock", span(repo, base, head)) == []


def test_research_lock_uses_the_merge_base(repo: GitRepo) -> None:
    fork = repo.head()
    repo.git("checkout", "-q", "-b", "feature")
    repo.write("research-lock.json", "{}\n")
    head = repo.commit("Lock without decision")
    repo.git("checkout", "-q", "main")
    repo.write("other.txt", "main moved on\n")
    base = repo.commit("Main moves")
    assert fork != base
    assert len(problems("research-lock", span(repo, base, head))) == 1


# ------------------------------------------------------------------------ decisions


def decisions(*entries: tuple[str, str], fields: tuple[str, ...] = pr_ready.DECISION_FIELDS) -> str:
    out = ["# Decisions", ""]
    for heading, _title in entries:
        out += [heading, ""] + [f"{f} x" for f in fields] + [""]
    return "\n".join(out)


def test_valid_decisions_pass(repo: GitRepo) -> None:
    assert problems("decisions", tree(repo)) == []


def test_many_decisions_pass(repo: GitRepo) -> None:
    heads = [(f"## D-{i:03d} \u2014 Title {i}", "") for i in range(1, 13)]
    repo.write("DECISIONS.md", decisions(*heads))
    repo.commit("Twelve decisions")
    assert problems("decisions", tree(repo)) == []


def test_bulleted_fields_pass(repo: GitRepo) -> None:
    text = "## D-001 \u2014 Bullets\n\n" + "\n".join(f"- {f} x" for f in pr_ready.DECISION_FIELDS) + "\n"
    repo.write("DECISIONS.md", text)
    repo.commit("Bullets")
    assert problems("decisions", tree(repo)) == []


@pytest.mark.parametrize(
    "heading",
    [
        "## D-001 - Hyphen instead of em dash",
        "## D-1 \u2014 Two few digits",
        "## D-0001 \u2014 Too many digits",
        "## D-001 \u2014",
        "## Notes",
    ],
)
def test_malformed_headings_fail(repo: GitRepo, heading: str) -> None:
    repo.write("DECISIONS.md", decisions((heading, "")))
    repo.commit("Malformed heading")
    found = problems("decisions", tree(repo))
    assert found and "does not read" in found[0]


def test_missing_field_fails(repo: GitRepo) -> None:
    fields = tuple(f for f in pr_ready.DECISION_FIELDS if f != "**Reversal cost:**")
    repo.write("DECISIONS.md", decisions(("## D-001 \u2014 No reversal cost", ""), fields=fields))
    repo.commit("Missing field")
    found = problems("decisions", tree(repo))
    assert found == ["DECISIONS.md:3: D-001 lacks **Reversal cost:**"]


def test_gap_fails(repo: GitRepo) -> None:
    repo.write("DECISIONS.md", decisions(("## D-001 \u2014 One", ""), ("## D-003 \u2014 Three", "")))
    repo.commit("Gap")
    found = problems("decisions", tree(repo))
    assert len(found) == 1 and "D-003 found where D-002 was expected" in found[0]


def test_must_start_at_one(repo: GitRepo) -> None:
    repo.write("DECISIONS.md", decisions(("## D-002 \u2014 Two", "")))
    repo.commit("Starts at two")
    assert "D-002 found where D-001 was expected" in problems("decisions", tree(repo))[0]


def test_decreasing_ids_fail(repo: GitRepo) -> None:
    repo.write(
        "DECISIONS.md",
        decisions(("## D-001 \u2014 One", ""), ("## D-002 \u2014 Two", ""), ("## D-002 \u2014 Again", "")),
    )
    repo.commit("Repeated id")
    assert len(problems("decisions", tree(repo))) == 1


def test_fenced_headings_are_ignored(repo: GitRepo) -> None:
    text = VALID_DECISIONS + "\n```markdown\n## D-012 \u2014 <title> (format example)\n## anything\n```\n"
    repo.write("DECISIONS.md", text)
    repo.commit("Format example")
    assert problems("decisions", tree(repo)) == []


def test_missing_decisions_fails(repo: GitRepo) -> None:
    repo.remove("DECISIONS.md")
    repo.commit("Remove decisions")
    assert "not tracked" in problems("decisions", tree(repo))[0]


def test_crlf_decisions_pass(repo: GitRepo) -> None:
    repo.write("DECISIONS.md", VALID_DECISIONS.replace("\n", "\r\n"))
    repo.commit("CRLF")
    assert problems("decisions", tree(repo)) == []


# --------------------------------------------------------------------------- ledger


def test_generated_ledger_passes(repo: GitRepo) -> None:
    assert problems("ledger", tree(repo)) == []


def test_hand_edited_ledger_fails(repo: GitRepo) -> None:
    text = (repo.root / "LEDGER.md").read_text(encoding="utf-8")
    repo.write("LEDGER.md", text.replace("No nightly has run yet.", "Everything works.", 1))
    repo.commit("Hand edit")
    assert "never edited by hand" in problems("ledger", tree(repo))[0]


def test_crlf_ledger_passes(repo: GitRepo) -> None:
    text = (repo.root / "LEDGER.md").read_text(encoding="utf-8")
    repo.write("LEDGER.md", text.replace("\n", "\r\n"))
    assert problems("ledger", tree(repo)) == []


def test_ledger_must_follow_inputs(repo: GitRepo) -> None:
    repo.write(
        "ledger/inputs/wasm.json",
        '{"schema": 1, "commit": "' + "a" * 40 + '", "raw_bytes": 10, "gzip_bytes": 5, "budget_gzip_bytes": 4194304}',
    )
    repo.commit("New input, stale ledger")
    assert len(problems("ledger", tree(repo))) == 1
    repo.regenerate_ledger()
    repo.commit("Regenerate")
    assert problems("ledger", tree(repo)) == []


def test_invalid_ledger_inputs_fail(repo: GitRepo) -> None:
    repo.write("ledger/inputs/wasm.json", '{"schema": 2}')
    repo.commit("Bad input")
    found = problems("ledger", tree(repo))
    assert found and "ledger inputs are invalid" in found[0] and "schema" in found[0]


def test_missing_ledger_fails(repo: GitRepo) -> None:
    repo.remove("LEDGER.md")
    repo.commit("Remove ledger")
    assert "not tracked" in problems("ledger", tree(repo))[0]


# --------------------------------------------------------------------- architecture


def test_documented_crates_pass(repo: GitRepo) -> None:
    assert problems("architecture", tree(repo)) == []


def test_undocumented_crate_fails(repo: GitRepo) -> None:
    repo.write("crates/classfile/Cargo.toml", '[package]\nname = "jarrow-classfile"\n')
    repo.commit("Add classfile")
    found = problems("architecture", tree(repo))
    assert found == [
        "crates/classfile has a Cargo.toml but is not mentioned as 'crates/classfile' in docs/architecture.md"
    ]


def test_prefix_mentions_do_not_count(repo: GitRepo) -> None:
    repo.write("crates/rule/Cargo.toml", '[package]\nname = "jarrow-rule"\n')
    repo.write("docs/architecture.md", "- crates/report\n- crates/rules-old (gone)\n")
    repo.commit("Prefix only")
    assert len(problems("architecture", tree(repo))) == 1


def test_nested_manifests_and_dirs_without_manifest_are_ignored(repo: GitRepo) -> None:
    repo.write("crates/report/fuzz/Cargo.toml", '[package]\nname = "fuzz"\n')
    repo.write("crates/notes/README.md", "not a crate\n")
    repo.commit("Nested")
    assert problems("architecture", tree(repo)) == []


def test_missing_architecture_doc_fails(repo: GitRepo) -> None:
    repo.remove("docs/architecture.md")
    repo.commit("Remove doc")
    assert "docs/architecture.md is missing" in problems("architecture", tree(repo))[0]


# -------------------------------------------------------------------------- secrets


@pytest.mark.parametrize("label", sorted(FAKE_SECRETS))
def test_secrets_are_found(repo: GitRepo, label: str, capsys: pytest.CaptureFixture[str]) -> None:
    value = FAKE_SECRETS[label]
    repo.write("config/notes.md", f"line one\nsecret: {value}\n")
    repo.commit("Leak")
    found = problems("secrets", tree(repo))
    assert found == [f"config/notes.md:2: looks like {label} (value not shown)"]
    assert pr_ready.main(["--all", "--check", "secrets", "--repo", str(repo.root)]) == 1
    captured = capsys.readouterr()
    assert value not in captured.out and value not in captured.err


def test_secret_references_pass(repo: GitRepo) -> None:
    workflow = "\n".join(
        [
            "env:",
            "  CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}",
            "run: |",
            '  CLOUDFLARE_API_TOKEN="$TOKEN" npx wrangler whoami',
            "  gh secret set CLOUDFLARE_API_TOKEN --repo MosheByte/jarrow",
            "  echo https://discord.com/api/webhooks/... is configured in DISCORD_WEBHOOK",
            "",
        ]
    )
    repo.write(".github/workflows/deploy.yml", workflow)
    repo.write("docs/secrets.md", "CLOUDFLARE_API_TOKEN: a token with Pages edit rights.\n")
    repo.commit("References only")
    assert problems("secrets", tree(repo)) == []


def test_binary_files_are_not_scanned(repo: GitRepo) -> None:
    repo.write("assets/blob.bin", b"\x00\xff" + FAKE_SECRETS["a GitHub token"].encode())
    repo.commit("Binary")
    assert problems("secrets", tree(repo)) == []


def test_secrets_in_skipped_dirs_are_ignored(repo: GitRepo) -> None:
    repo.write("site/node_modules/pkg/index.js", FAKE_SECRETS["an npm token"] + "\n")
    repo.commit("Vendored by mistake")
    assert problems("secrets", tree(repo)) == []


def test_problems_print_on_one_line_each(capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch, repo: GitRepo) -> None:
    # A tracked path may hold a newline. Printed as is, the text after it would start a log
    # line, and a line starting with "::" is a workflow command (security review F-08).
    monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
    spoofed = "x.jar\n::error::spoofed annotation\r\n::add-mask::y: JVM archives and class files are never committed"
    monkeypatch.setattr(pr_ready, "run", lambda ctx, only=None: {"artifacts": [spoofed]})
    assert pr_ready.main(["--all", "--repo", str(repo.root)]) == 1
    out = capsys.readouterr().out
    assert not [line for line in out.splitlines() if line.startswith("::")]
    assert "x.jar\\n::error::spoofed annotation\\r\\n::add-mask::y" in out
