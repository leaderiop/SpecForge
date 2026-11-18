---
id: 0001
title: z3 becomes a collected-proof pack; the host's prove step and @specforge/formal go
status: open
trigger: truth.expression is built (until then the host's current prove step can stay)
created: 2026-11-18
updated: 2026-11-18
links:
  - docs/adr/0009-host-passes-read-declarations.md
  - docs/adr/0079-the-smt-solver-is-one-port-that-analyze-and-doctor-take.md
  - docs/adr/0002-test-runner-extensions.md
  - docs/research/specforge-gsd-beyond-parity.html (rules R8, X3; truth fields table)
  - https://claude.ai/artifact/Rc4VXSa7roDy7obNpjFBwh (explainer, v2)
---

# z3 becomes a collected-proof pack

## Why

`specforge analyze --prove` asks z3 whether declared bounds are consistent and entail declared claims. The three
jobs (translate the spec to SMT-LIB2, run z3, read the answer) all sit in the host, in `specforge-ops`. The
project's own plans say `@specforge/formal` goes (beyond-GSD rule R8, "Proof is language-native", and metric X3,
`@specforge/*` starters 9 → 0), and that a truth's optional `expression` is "judged by an SMT judge" without saying
where that judge lives. This todo is that judge's place.

## Today (read 2026-11-18, not re-run)

- Prove pass: `crates/app/ops/src/analysis/prove` (called from `crates/app/ops/src/analysis/analyze.rs:364`). It
  reads fields by role (`registries.fields()`), not by extension name.
- Solver port and adapter: `crates/app/ops/src/solver.rs` (`Solver`, `Z3`, `ScriptedSolver` in `testing.rs`).
  Probe deadline 5 s (`Z3::PROBE_TIMEOUT`), solve deadline 30 s (`--prove-timeout`). ADR 0079.
- z3 missing is one message, `W098`, which doctor also uses as its toolchain finding.
- `extensions/formal` still exists (2,669 lines of Rust) and is not loaded by this repository's root since PR #184.
- The old limits that `prove` read (governance `constraint` bounds) were dropped by the conversion, so
  `analyze --prove` on the root likely finds nothing. NOT CHECKED.

## Target (proposal, not agreed)

- A small pack named for the tool, e.g. `@smt/z3` (name is a proposal). Pure guest code: translate a truth's
  `expression` to a script, read z3's answer into one result record. Same shape as a test runner's
  `collect__<runner>` export (ADR 0002).
- The host only runs `z3` through the Process port, with consent and the existing deadlines. A guest cannot start
  programs, so this stays in the host.
- The answer is a collected fact (like a test result) with its own reference scheme, e.g. `smt:`. `truth.proof`
  cites it, and `@gsd/core`'s computed `proof` gives proven / failed / no_test.
- Removed from the host: the prove pass, the `--prove` flag, the rule that a proved claim discharges a `verify`
  line, and the special W098 wording (doctor uses the generic "tool missing, these codes are off" finding).
- Dropped: `@specforge/formal`.

## Tasks

- [ ] T1 Decide where the expression grammar lives (ADR 0009 needs one without tree-sitter, and a way to carry an
  expression through pass snapshots). Write the ADR.
- [ ] T2 Define the proof result: record kind in `@code/core` (or the pack), reference scheme, fields (status,
  solver version, script digest, timeout).
- [ ] T3 Write the pack: translate, read the answer, tests with `ScriptedSolver`-style fakes. Re-vendor the blob on
  linux/amd64 (`docs/agents/revendor.md`; a macOS build fails the bootstrap job).
- [ ] T4 Wire `truth.expression` so an expression produces a collect request, and `truth.proof` can cite the result.
- [ ] T5 Doctor: replace the z3-specific finding with the generic missing-tool finding; keep the 5 s probe.
- [ ] T6 Remove the host's prove step: pass, `--prove`, `--prove-timeout` (or move it to the collect call), funnel,
  W098 wording. No backward-compat shims (the project is pre-release).
- [ ] T7 Retire `@specforge/formal`: delete `extensions/formal` and its blob, and migrate or drop what still loads
  it (examples, `integrations/rust/spec`, the old-spec corpus under `fixtures/old-spec`).
- [ ] T8 Docs: ADR for the pack, update ADR 0009 and 0079 status lines, `ARCHITECTURE.md`/`crates/README.md` via
  `cargo gate crate-map`.

## Open questions

- [ ] Q1 Where does the maths grammar live (T1)?
- [ ] Q2 Does R4's decimal literal (plan 00, ADR 0361) already cover part of it? NOT CHECKED.
- [ ] Q3 What does `analyze --prove` return on the swapped root today? Run it to confirm.
- [ ] Q4 What replaces "a proved claim discharges a `verify property` obligation"?
- [ ] Q5 Is the pack named `@smt/z3`, or by another ecosystem name? Should `--prove` survive as a flag on collect?

## Related, separate

- The z3 test flake: `a_z3_whose_version_fails_does_not_run` (`solver.rs:302`) failed twice under local gate
  load (`z3 --version did not answer within 5s`); 3/3 green alone, none seen in CI. Pre-existing code (`eeefc9a1c`).
  Not part of this todo. The new design keeps the same door and deadline, so it needs fixing either way.

## Not decided

Everything under "Target". The plans fix only that formal is dropped, that proof is collected like a test, and that
an SMT judge handles `truth.expression`.
