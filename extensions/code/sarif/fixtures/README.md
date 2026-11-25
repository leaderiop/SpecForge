# SARIF reader fixtures

Neither clippy-sarif nor ESLint was installed where the first fixtures were written, so those are hand-written to the
SARIF 2.1.0 specification and the tool shape named below. `eslint-10.15.0.sarif` is a real recording; its absolute
paths were replaced with one command:
`sed -E 's#/private/tmp/[^"<> ]*/research-tools/[^/"<> ]*#/project#g' <recording> > eslint-10.15.0.sarif`.

| Fixture | Source |
|---------|--------|
| `clippy.sarif` | hand-written to SARIF 2.1.0 and the clippy-sarif shape (`cargo clippy --message-format=json \| clippy-sarif`): driver `clippy` with the rules `clippy::needless_return` and `clippy::unwrap_used`, results with `ruleId`, `ruleIndex` and level `warning`, `uri` `src/lib.rs` relative to `%SRCROOT%`. One result in the method `Board::len` (line 7), one in the test fn `tests::reads` (lines 16-17), one on line 24, which no item holds |
| `eslint.sarif` | hand-written to the @microsoft/eslint-formatter-sarif shape: driver `ESLint` with rule metadata and no `ruleIndex` on any result, `artifacts[]` holding the absolute percent-encoded `file:///project/src/board%20view.js`, levels `error` and `warning`. One result giving its file by `uri`, one by `artifactLocation.index` alone, one of a rule with no metadata (`local/no-raw-dom`), one suppressed in source |
| `rule-less.sarif` | hand-written to SARIF 2.1.0 §3.27.10: driver `demo-lint` with rule `R1` (no `defaultConfiguration`) and `R2` (default `note`). A result naming no rule, a result of `R1` stating no level, a `kind` `pass` with no level, `R2` by `ruleIndex` alone and by `rule.id` with no location, and two results of `R1` at one line |
| `eslint-10.15.0.sarif` | recorded with ESLint 10.15.0 and @microsoft/eslint-formatter-sarif 3.1.0 by the Phase 11 research (`eslint --format @microsoft/eslint-formatter-sarif --output-file <report> .` over the package `board`, `js.configs.recommended` with `no-console` warn and `eqeqeq` error), absolute paths replaced by `/project`. Four results in `src/board view.js` (percent-encoded `board%20view.js`), each by `uri` and `index`, with `ruleIndex` |
| `eslint-10.15.0.items.json` | hand-written served facts for that recording: the function `view` (lines 1-5 of the recorded `src/board view.js`) and the package `board`. Its own file: `view` would overlap the hand-written `render` of `items.json` in the same file and change the `eslint` snapshot |
| `items.json` | hand-written served facts: the `item` facts (with their places) and the root `package` a lint collector receives, for the crate `demo` (`src/lib.rs`) and the npm package `board` (`src/board view.js`) |
