# LCOV reader fixtures

None of the coverage tools is installed where these were written, so each fixture is hand-written to the grammar or
source named below. Phase 11's packs re-record them with the real tools.

| Fixture | Source |
|---------|--------|
| `llvm-cov.info` | hand-written to llvm-cov's `CoverageExporterLcov` output (`cargo llvm-cov --lcov`): absolute `SF`, `FN:<line>,<mangled name>` with no end line, `FNDA`, `DA`, `BRDA`, the file totals and `end_of_record`. Crate `demo` at `/project`: `add` (lines 3-5), `Board::len` in an impl (12-14), `tests::adds` in `mod tests` (21-24), `helper` with two monomorphizations (27-34), and line 37 in no item |
| `c8.info` | hand-written to istanbul-reports lcovonly (`c8 --reporter=lcovonly`): an empty `TN`, relative `SF:src/board.js`, `FN:<line>,<name>`, the totals after each family. Package `board`: `createBoard` (lines 2-8), class `Board` (10-18) with `constructor` (11-13) and `add` (14-17), and a top-level call on line 20 in no item |
| `lcov2.info` | hand-written to geninfo(1) LCOV 2.2: `#` comment, `TN`, `VER`, function groups `FNL:<index>,<start>,<end>` with their aliases `FNA:<index>,<hits>,<name>` (one name holding a comma), `MCDC`/`MCF`/`MCH`. The same counts as `llvm-cov.info`, in group form |
| `items.json` | hand-written served facts: the `item` facts (with their places) and the root `package` a coverage collector receives, for the crate `demo` (`src/lib.rs`) and the npm package `board` (`src/board.js`) |
