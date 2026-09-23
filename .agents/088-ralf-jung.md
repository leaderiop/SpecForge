# 088 — Ralf Jung

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** RustBelt/Miri; semantic soundness and testing-as-oracle
**SpecForge anchors:** soundness story for the Rust core crates, testing-as-oracle via specforge-test + specforge-report.json protocol, crates/specforge-* implementation discipline

## Why this engineer
Jung proved Rust's core type system sound (RustBelt) and built Miri, an interpreter that detects undefined behavior by executing real programs — methodology SpecForge's Rust implementation already leans on and should honor explicitly: keep crates/specforge-* free of unsafe and let the ~2,900-test suite plus specforge-report.json tracing act as the semantic oracle against which graph and diagnostic behavior is pinned. His "derive the semantics by testing the model against real inputs" loop is exactly how @specforge/formal passes should be validated against the 219-file self-spec corpus.

## References for SpecForge
**Key works**
- [RustBelt: Securing the Foundations of the Rust Programming Language (PhD thesis)](https://plv.mpi-sws.org/rustbelt/thesis.pdf) — Saarland University, 2020. End-to-end blueprint for a soundness claim about a language core — the standard a "the Rust core is sound" story must meet.
- [Miri (interpreter for Rust's mid-level IR)](https://github.com/rust-lang/miri) — GitHub, rust-lang/miri. UB detection by execution — precedent for interpreter-as-oracle over compiler/assembler artifacts.
- [Stacked Borrows: An Aliasing Model for Rust](https://plv.mpi-sws.org/rustbelt/stacked-borrows/) — POPL 2020. Model-derivation-by-testing: propose semantics, validate against code at scale — the loop for tuning formal-pass thresholds.
- [Ralf's Ramblings](https://www.ralfj.de/blog/) — ongoing work incl. Tree Borrows; unusually readable soundness engineering.

## Study first
1. Miri's oracle role — what specforge-test's verify attributes pin down
2. RustBelt thesis intro — scoping a soundness claim to a TCB
3. Stacked/Tree Borrows evolution — how tested models get revised without breaking users
