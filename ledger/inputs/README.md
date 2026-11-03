# ledger/inputs

The numbers `LEDGER.md` is rendered from. Nobody edits these files by hand: the jobs named below write them, and `tools/ledger.py` reads them. Each file is optional; a missing file means the number does not exist yet, and the ledger says which milestone produces it. The schemas are in the docstring of `tools/ledger.py`.

| File | Written by | From |
| --- | --- | --- |
| `board-summary.json` | The `publish-ledger` job of `.github/workflows/board.yml`, committed as jarrow-bot together with `LEDGER.md` | `jarrow-board merge` (`board-summary.json` in [docs/board.md](../../docs/board.md)) |
| `coverage.json` | The rule tables and the Fabric API index | Milestone M2 |
| `wasm.json` | The `crates/wasm` size check | Milestone M3 |
| `perf.json` | CLI and browser timings | Milestones M2 and M3 |

`tools/pr_ready.py` fails any change whose `LEDGER.md` differs from `python tools/ledger.py --stdout`, so an input and the ledger always change together.
