# SARIF reader fixtures

Neither clippy-sarif nor ESLint is installed where these were written, so each fixture is hand-written to the SARIF
2.1.0 specification and the tool shape named below. Phase 11's packs re-record them with the real tools.

| Fixture | Source |
|---------|--------|
| `clippy.sarif` | hand-written to SARIF 2.1.0 and the clippy-sarif shape (`cargo clippy --message-format=json \| clippy-sarif`): driver `clippy` with the rules `clippy::needless_return` and `clippy::unwrap_used`, results with `ruleId`, `ruleIndex` and level `warning`, `uri` `src/lib.rs` relative to `%SRCROOT%`. One result in the method `Board::len` (line 7), one in the test fn `tests::reads` (lines 16-17), one on line 24, which no item holds |
| `items.json` | hand-written served facts: the `item` facts (with their places) and the root `package` a lint collector receives, for the crate `demo` (`src/lib.rs`) |
