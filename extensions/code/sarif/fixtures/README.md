# SARIF reader fixtures

Neither clippy-sarif nor ESLint is installed where these were written, so each fixture is hand-written to the SARIF
2.1.0 specification and the tool shape named below. Phase 11's packs re-record them with the real tools.

| Fixture | Source |
|---------|--------|
| `clippy.sarif` | hand-written to SARIF 2.1.0 and the clippy-sarif shape (`cargo clippy --message-format=json \| clippy-sarif`): driver `clippy` with the rules `clippy::needless_return` and `clippy::unwrap_used`, results with `ruleId`, `ruleIndex` and level `warning`, `uri` `src/lib.rs` relative to `%SRCROOT%`. One result in the method `Board::len` (line 7), one in the test fn `tests::reads` (lines 16-17), one on line 24, which no item holds |
| `eslint.sarif` | hand-written to the @microsoft/eslint-formatter-sarif shape: driver `ESLint` with rule metadata and no `ruleIndex` on any result, `artifacts[]` holding the absolute percent-encoded `file:///project/src/board%20view.js`, levels `error` and `warning`. One result giving its file by `uri`, one by `artifactLocation.index` alone, one of a rule with no metadata (`local/no-raw-dom`), one suppressed in source |
| `rule-less.sarif` | hand-written to SARIF 2.1.0 §3.27.10: driver `demo-lint` with rule `R1` (no `defaultConfiguration`) and `R2` (default `note`). A result naming no rule, a result of `R1` stating no level, a `kind` `pass` with no level, `R2` by `ruleIndex` alone and by `rule.id` with no location, and two results of `R1` at one line |
| `items.json` | hand-written served facts: the `item` facts (with their places) and the root `package` a lint collector receives, for the crate `demo` (`src/lib.rs`) and the npm package `board` (`src/board view.js`) |
