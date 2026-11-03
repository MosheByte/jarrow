# jarrow-report

The report model of [Jarrow](https://github.com/jarrow-dev/jarrow), a converter from Fabric to NeoForge for Minecraft mod JARs. Every surface that shows a conversion (the CLI, the browser, the nightly board) uses this crate's vocabulary.

**This is a pre-alpha release. Jarrow converts nothing yet.** What the crate has today:

- `Tier`: whether a mod can be converted (Tier A, Tier B) or why it cannot (unsupported).
- `Outcome`: a harness result (green, yellow, red), or `NotRun` when no result exists.
- `ReportEntry`: one structured line of a conversion report.
- `tokens`: the design tokens, rendered as CSS (`tokens::css()`) or JSON (`tokens::json()`), plus badge colours.

The types implement serde's `Serialize` and `Deserialize`. The full report document and its Markdown rendering are not built yet (milestone M1), and the API will change before 0.1.0.

```toml
[dependencies]
jarrow-report = "=0.1.0-alpha.1"
```

## Links

- Repository and roadmap: https://github.com/jarrow-dev/jarrow
- Architecture: [docs/architecture.md](https://github.com/jarrow-dev/jarrow/blob/main/docs/architecture.md)

MIT licensed.
