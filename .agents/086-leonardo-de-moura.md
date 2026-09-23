# 086 — Leonardo de Moura

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Z3 co-creator; Lean 4 creator
**SpecForge anchors:** SMT backend for future `analyze --prove`, condition_check discharge path, Wasm validation_engine extension point

## Why this engineer
De Moura built Z3, the SMT solver that quietly discharges most industrial verification conditions, and Lean 4, the proof assistant that turns undischarged goals into interactive proof work. If SpecForge's `analyze --prove` path ever exists, Z3 is its likeliest engine: condition entities and property entities become SMT formulas, W036–W040 checks become satisfiability queries, and counterexamples become concrete pre-states reported in diagnostics. His Z3→Lean arc is also the pattern for SpecForge's ladder — automatic checking first, human proof only for the residue.

## References for SpecForge
**Key works**
- **Z3: An Efficient SMT Solver** — TACAS 2008 (LNCS 4963). The architecture paper: DPLL(T), theory combination — what a condition-encoding backend would rest on.
- [Z3Prover/z3](https://github.com/Z3Prover/z3) — GitHub. The solver itself; SMT-LIB interface is the contract an analyze backend would target.
- [leanprover/lean4](https://github.com/leanprover/lean4) — GitHub. Lean 4: dependent types, metaprogramming, kernel-checked proofs.
- [Lean FRO](https://lean-fro.org) — his current organization advancing Lean's ecosystem.
- **The Lean 4 Theorem Prover and Programming Language** — PLP keynote, 2021. Design retrospective: performance, elaboration, and the tactic/kernel split.

## Study first
1. Z3 TACAS paper — how contracts become satisfiability queries
2. SMT-LIB input format — the interface `analyze --prove` should emit
3. Unsat cores in Z3 — minimal counterexamples for failed condition checks
