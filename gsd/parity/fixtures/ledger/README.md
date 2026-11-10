# Ledger (fixture)

A ledger of money on the command line: `ledger add <cents> <memo>` prints an entry's store line, `ledger total < store`
prints the total. A Rust package with no dependencies, its own workspace. It exists to give the GSD A/B a project whose
CI is red; see [gsd/parity/README.md](../../README.md) for how its goldens are recorded (`record-ledger.sh`).

- `.planning/` is hand-written from GSD's templates: milestone-less, phases 1, 2 and 3. Phase 1 is executed and
  verified (`01-VERIFICATION.md`: passed, 4/4); phase 2 is planned; phase 3 is empty. Quick task `261105-tcl` came
  after the verification: it added `column` and folded the total in an `i32`, so `total_beyond_i32_is_exact`, which
  proves phase 1's fourth truth, fails. `STATE.md` lists it under Quick Tasks Completed; nothing in `.planning/` says
  the test fails.
- `src/`, `tests/`: what phase 1 and the quick task built. `cargo nextest run` fails one test of six;
  `record-ledger.sh` records that run in `goldens/ledger/rust-test.xml`, and the run with the fold restored to an
  `i64`, the state the verifier saw, in `goldens/ledger/rust-test-before-quick.xml`.
- `spec/`: the SpecForge twin of `.planning/`: project, decisions, requirements, roadmap, the three plans with their
  must-haves, phase 1's summaries, and the quick task as a plan with no phase. Each of phase 1's truths cites the test
  that proves it (`proof`, `rust:ledger::<test binary>#<test name>`, a fact `@rust/test` collects from a nextest report).
  It loads `@code/core`, `@rust/cargo`, `@rust/source` and `@rust/test`.
- `specforge-harness`: the ledger lines of phase 1's eight verifier verdicts, recorded over the run before the quick
  task (`GSD_BLESS=1 cargo test the_twin_holds_its_verifier_verdicts`). Over the recorded red run the one verdict of
  the truth the failing test proves is stale (`tests changed`), and `specforge trace LEDG-02` shows the test
  `(failed)`: `gsd/core/tests/cli/parity_ledger.rs`.
