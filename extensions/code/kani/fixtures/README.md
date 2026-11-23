# Kani reader fixtures

Every fixture is hand-written to Kani's `--output-format regular` standard output as `kani-driver/src/harness_runner.rs`
prints it and Kani's own tests expect it (`tests/expected/*/expected`; the Phase 11 research, section B6). Kani was not
installable on the authoring host: it runs on Linux x86_64 and macOS only (ADR 0389 D12), and its compiler and CBMC
were not set up there. Each fixture is to be replaced by a recording of a real `cargo-kani --workspace
--output-format regular` run, its absolute paths replaced by `/project`.

| Fixture | Source |
|---------|--------|
| `one-crate.txt` | hand-written: crate `demo` (`/project`), one harness `proofs::check_add` with two of its twelve checks shown, ` ** 0 of 12 failed`, `VERIFICATION:- SUCCESSFUL`, `Verification Time: 0.3897019s`, `Manual Harness Summary:` and `Complete - 1 successfully verified harnesses, 0 failures, 1 total.` |
