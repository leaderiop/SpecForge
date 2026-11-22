# LCOV reader fixtures

None of the coverage tools is installed where these were written, so each fixture is hand-written to the grammar or
source named below. Phase 11's packs re-record them with the real tools.

| Fixture | Source |
|---------|--------|
| `llvm-cov.info` | hand-written to llvm-cov's `CoverageExporterLcov` output (`cargo llvm-cov --lcov`): absolute `SF`, `FN:<line>,<mangled name>` with no end line, `FNDA`, `DA`, `BRDA`, the file totals and `end_of_record`. Crate `demo` at `/project`: `add` (lines 3-5), `Board::len` in an impl (12-14), `tests::adds` in `mod tests` (21-24), `helper` with two monomorphizations (27-34), and line 37 in no item |
| `items.json` | hand-written served facts: the `item` facts (with their places in `src/lib.rs`) and the root `package` a coverage collector receives for the crate above |
